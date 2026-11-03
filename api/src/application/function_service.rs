use crate::domain::entities::{DomainError, Function};
use crate::domain::ports::FunctionRepository;
use crate::domain::wasm_runtime::WasmRuntime;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tokio::task;
use tracing::{info, warn};

const MAX_BACKGROUND_PRELOADS: usize = 1;

pub struct FunctionService {
    repository: Arc<dyn FunctionRepository>,
    runtime: Arc<dyn WasmRuntime>,
    storage_path: String,
    preload_semaphore: Arc<Semaphore>,
    preloading_functions: Arc<Mutex<HashMap<String, Option<String>>>>,
}

impl FunctionService {
    pub fn new(
        repository: Arc<dyn FunctionRepository>,
        runtime: Arc<dyn WasmRuntime>,
        storage_path: String,
    ) -> Self {
        if let Err(e) = fs::create_dir_all(&storage_path) {
            warn!("Failed to create storage directory {}: {}", storage_path, e);
        }
        Self {
            repository,
            runtime,
            storage_path,
            preload_semaphore: Arc::new(Semaphore::new(MAX_BACKGROUND_PRELOADS)),
            preloading_functions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn store_wasm(&self, name: &str, source_path: &str) -> Result<String, DomainError> {
        let source = Path::new(source_path);
        if !source.exists() {
            return Err(DomainError::Internal(format!(
                "Source file not found: {}",
                source_path
            )));
        }

        let file_name = format!("{}.wasm", name);
        let dest = Path::new(&self.storage_path).join(&file_name);

        fs::copy(source, &dest)
            .map_err(|e| DomainError::Internal(format!("Failed to copy Wasm binary: {}", e)))?;

        info!("Stored Wasm for {} at {:?}", name, dest);

        dest.to_str()
            .map(str::to_string)
            .ok_or_else(|| DomainError::Internal("Invalid path encoding".to_string()))
    }

    pub async fn create_function(&self, mut function: Function) -> Result<Function, DomainError> {
        if !function.executable.is_empty() {
            let new_path = self.store_wasm(&function.name, &function.executable)?;
            function.executable = new_path;
        }

        let created = self.repository.save(&function).await?;

        self.preload_function(created.clone());

        Ok(created)
    }

    pub async fn update_function(&self, mut function: Function) -> Result<Function, DomainError> {
        if !function.executable.is_empty() {
            let new_path = self.store_wasm(&function.name, &function.executable)?;
            function.executable = new_path;
        }

        let updated = self.repository.update(&function).await?;
        self.preload_function(updated.clone());
        Ok(updated)
    }

    pub async fn list_functions(&self) -> Result<Vec<Function>, DomainError> {
        self.repository.find_all().await
    }

    pub async fn preload_existing(&self) -> Result<(), DomainError> {
        let functions = self.repository.find_all().await?;
        for function in functions {
            self.preload_function(function);
        }
        Ok(())
    }

    pub async fn get_function(&self, name: &str) -> Result<Function, DomainError> {
        self.repository
            .find_by_name(name)
            .await?
            .ok_or_else(|| DomainError::NotFound(name.to_string()))
    }

    pub async fn delete_function(&self, name: &str) -> Result<(), DomainError> {
        self.repository.delete(name).await
    }

    fn preload_function(&self, function: Function) {
        if function.executable.is_empty() {
            return;
        }

        let runtime = self.runtime.clone();
        let name = function.name.clone();
        let executable = function.executable.clone();
        let preload_semaphore = self.preload_semaphore.clone();
        let preloading_functions = self.preloading_functions.clone();

        task::spawn(async move {
            {
                let mut loading = preloading_functions.lock().await;
                if let Some(pending) = loading.get_mut(&name) {
                    *pending = Some(executable);
                    info!(
                        function_name = %name,
                        "Queued newer component behind the in-flight preload"
                    );
                    return;
                }
                loading.insert(name.clone(), None);
            }

            let mut executable = executable;
            loop {
                let permit = match preload_semaphore.clone().acquire_owned().await {
                    Ok(permit) => permit,
                    Err(error) => {
                        preloading_functions.lock().await.remove(&name);
                        warn!(
                            function_name = %name,
                            error = %error,
                            "Background preload queue is shutting down"
                        );
                        return;
                    }
                };

                let runtime = runtime.clone();
                let load_name = name.clone();
                let load_executable = executable.clone();
                let result = task::spawn_blocking(move || {
                    runtime.load_function(&load_name, &load_executable)
                })
                .await;

                drop(permit);

                match result {
                    Ok(Ok(())) => info!(function_name = %name, "Function preloaded in background"),
                    Ok(Err(error)) => {
                        warn!(function_name = %name, error = %error, "Failed to preload wasm in background")
                    }
                    Err(error) => {
                        warn!(function_name = %name, error = %error, "Background preload task panicked")
                    }
                }

                let mut loading = preloading_functions.lock().await;
                match loading.get_mut(&name).and_then(|pending| pending.take()) {
                    Some(next) => executable = next,
                    None => {
                        loading.remove(&name);
                        break;
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::MockFunctionRepository;
    use crate::domain::wasm_runtime::MockWasmRuntime;
    use mockall::predicate::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::sync::Mutex as StdMutex;
    use tempfile::tempdir;
    use tokio::sync::oneshot;
    use tokio::time::{timeout, Duration};

    fn expect_background_load(runtime: &mut MockWasmRuntime) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        let tx = StdMutex::new(Some(tx));
        runtime.expect_load_function().returning(move |_, _| {
            if let Some(tx) = tx.lock().expect("preload notifier").take() {
                let _ = tx.send(());
            }
            Ok(())
        });
        rx
    }

    async fn wait_for_preload(rx: oneshot::Receiver<()>) {
        timeout(Duration::from_secs(2), rx)
            .await
            .expect("background preload timed out")
            .expect("background preload notifier dropped");
    }

    #[tokio::test]
    async fn test_create_function() {
        let mut repo = MockFunctionRepository::new();
        let mut runtime = MockWasmRuntime::new();
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().to_str().unwrap().to_string();

        repo.expect_save()
            .with(always())
            .returning(|f| Ok(f.clone()));

        let loaded = expect_background_load(&mut runtime);

        let service = FunctionService::new(Arc::new(repo), Arc::new(runtime), storage_path.clone());

        // Create a dummy wasm file
        let source_file = temp_dir.path().join("test.wasm");
        fs::write(&source_file, "dummy content").unwrap();

        let function = Function {
            name: "test-func".to_string(),
            executable: source_file.to_str().unwrap().to_string(),
            ..Default::default()
        };

        let result = service.create_function(function).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert!(created.executable.starts_with(&storage_path));
        assert!(Path::new(&created.executable).exists());
        wait_for_preload(loaded).await;
    }
    #[tokio::test]
    async fn test_get_function_found() {
        let mut repo = MockFunctionRepository::new();
        let runtime = MockWasmRuntime::new();
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().to_str().unwrap().to_string();

        let function = Function {
            name: "test-func".to_string(),
            ..Default::default()
        };

        repo.expect_find_by_name()
            .with(eq("test-func"))
            .returning(move |_| Ok(Some(function.clone())));

        let service = FunctionService::new(Arc::new(repo), Arc::new(runtime), storage_path);
        let result = service.get_function("test-func").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "test-func");
    }

    #[tokio::test]
    async fn test_get_function_not_found() {
        let mut repo = MockFunctionRepository::new();
        let runtime = MockWasmRuntime::new();
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().to_str().unwrap().to_string();

        repo.expect_find_by_name()
            .with(eq("unknown"))
            .returning(|_| Ok(None));

        let service = FunctionService::new(Arc::new(repo), Arc::new(runtime), storage_path);
        let result = service.get_function("unknown").await;

        assert!(matches!(result, Err(DomainError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_functions() {
        let mut repo = MockFunctionRepository::new();
        let runtime = MockWasmRuntime::new();
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().to_str().unwrap().to_string();

        repo.expect_find_all().returning(|| {
            Ok(vec![
                Function {
                    name: "f1".to_string(),
                    ..Default::default()
                },
                Function {
                    name: "f2".to_string(),
                    ..Default::default()
                },
            ])
        });

        let service = FunctionService::new(Arc::new(repo), Arc::new(runtime), storage_path);
        let result = service.list_functions().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_update_function() {
        let mut repo = MockFunctionRepository::new();
        let mut runtime = MockWasmRuntime::new();
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().to_str().unwrap().to_string();

        repo.expect_update().returning(|f| Ok(f.clone()));

        let loaded = expect_background_load(&mut runtime);

        // Create dummy file for update
        let source_file = temp_dir.path().join("update.wasm");
        fs::write(&source_file, "updated content").unwrap();

        let service = FunctionService::new(Arc::new(repo), Arc::new(runtime), storage_path);

        let function = Function {
            name: "test-func".to_string(),
            executable: source_file.to_str().unwrap().to_string(),
            ..Default::default()
        };

        let result = service.update_function(function).await;
        assert!(result.is_ok());
        wait_for_preload(loaded).await;
    }

    #[tokio::test]
    async fn test_update_during_preload_loads_the_newer_binary() {
        let mut repo = MockFunctionRepository::new();
        let mut runtime = MockWasmRuntime::new();
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().to_str().unwrap().to_string();

        repo.expect_update().returning(|f| Ok(f.clone()));

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_in_load = calls.clone();
        let (started_tx, started_rx) = oneshot::channel();
        let started_tx = StdMutex::new(Some(started_tx));
        let (release_tx, release_rx) = mpsc::channel();
        let release_rx = StdMutex::new(release_rx);

        runtime.expect_load_function().returning(move |_, _| {
            let call = calls_in_load.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                if let Some(tx) = started_tx.lock().expect("preload start").take() {
                    let _ = tx.send(());
                }
                let _ = release_rx.lock().expect("preload release").recv();
            }
            Ok(())
        });

        let first = temp_dir.path().join("first.wasm");
        let second = temp_dir.path().join("second.wasm");
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();

        let service = FunctionService::new(Arc::new(repo), Arc::new(runtime), storage_path);
        service
            .update_function(Function {
                name: "test-func".to_string(),
                executable: first.to_str().unwrap().to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        timeout(Duration::from_secs(2), started_rx)
            .await
            .expect("first preload did not start")
            .expect("first preload notifier dropped");

        service
            .update_function(Function {
                name: "test-func".to_string(),
                executable: second.to_str().unwrap().to_string(),
                ..Default::default()
            })
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        release_tx.send(()).expect("release preload");

        timeout(Duration::from_secs(2), async {
            while calls.load(Ordering::SeqCst) < 2 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("newer preload did not run");
    }

    #[tokio::test]
    async fn test_preload_existing_loads_functions_with_executables() {
        let mut repo = MockFunctionRepository::new();
        let mut runtime = MockWasmRuntime::new();
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().to_str().unwrap().to_string();
        let wasm_path = temp_dir.path().join("healthz.wasm");
        fs::write(&wasm_path, "wasm").unwrap();
        let executable = wasm_path.to_str().unwrap().to_string();

        repo.expect_find_all().returning(move || {
            Ok(vec![
                Function {
                    name: "healthz".to_string(),
                    executable: executable.clone(),
                    ..Default::default()
                },
                Function {
                    name: "empty".to_string(),
                    ..Default::default()
                },
            ])
        });

        let loaded = expect_background_load(&mut runtime);
        let service = FunctionService::new(Arc::new(repo), Arc::new(runtime), storage_path);

        service.preload_existing().await.unwrap();
        wait_for_preload(loaded).await;
    }

    #[tokio::test]
    async fn test_delete_function() {
        let mut repo = MockFunctionRepository::new();
        let runtime = MockWasmRuntime::new();
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().to_str().unwrap().to_string();

        repo.expect_delete()
            .with(eq("test-func"))
            .returning(|_| Ok(()));

        let service = FunctionService::new(Arc::new(repo), Arc::new(runtime), storage_path);
        let result = service.delete_function("test-func").await;

        assert!(result.is_ok());
    }
}
