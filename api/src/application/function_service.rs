use crate::domain::entities::{DomainError, Function};
use crate::domain::ports::FunctionRepository;
use crate::domain::wasm_runtime::WasmRuntime;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

pub struct FunctionService {
    repository: Arc<dyn FunctionRepository>,
    runtime: Arc<dyn WasmRuntime>,
    storage_path: String,
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
            .map(|s| s.to_string())
            .ok_or_else(|| DomainError::Internal("Invalid path encoding".to_string()))
    }

    pub async fn create_function(&self, mut function: Function) -> Result<Function, DomainError> {
        if !function.executable.is_empty() {
            let new_path = self.store_wasm(&function.name, &function.executable)?;
            function.executable = new_path;
        }

        let created = self.repository.save(&function).await?;

        if !created.executable.is_empty()
            && let Err(e) = self
                .runtime
                .load_function(&created.name, &created.executable)
            {
                warn!("Failed to preload wasm: {}", e);
            }

        Ok(created)
    }

    pub async fn update_function(&self, mut function: Function) -> Result<Function, DomainError> {
        if !function.executable.is_empty() {
            let new_path = self.store_wasm(&function.name, &function.executable)?;
            function.executable = new_path;
        }

        let updated = self.repository.update(&function).await?;
        if !updated.executable.is_empty()
            && let Err(e) = self
                .runtime
                .load_function(&updated.name, &updated.executable)
            {
                warn!("Failed to reload wasm: {}", e);
            }
        Ok(updated)
    }

    pub async fn list_functions(&self) -> Result<Vec<Function>, DomainError> {
        self.repository.find_all().await
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::MockFunctionRepository;
    use crate::domain::wasm_runtime::MockWasmRuntime;
    use mockall::predicate::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_function() {
        let mut repo = MockFunctionRepository::new();
        let mut runtime = MockWasmRuntime::new();
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().to_str().unwrap().to_string();

        repo.expect_save()
            .with(always())
            .returning(|f| Ok(f.clone()));

        runtime.expect_load_function().returning(|_, _| Ok(()));

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

        // Expect reload if executable present
        runtime.expect_load_function().returning(|_, _| Ok(()));

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
