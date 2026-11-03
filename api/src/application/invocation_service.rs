use crate::domain::entities::{DomainError, Function};
use crate::domain::ports::{FunctionRepository, TriggerRepository};
use crate::domain::wasm_runtime::{WasmFunctionRequest, WasmInvocation, WasmRuntime};
use ahash::RandomState;
use opentelemetry::{KeyValue, global};
use papaya::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::task;
use tracing::{error, info, instrument, warn};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RouteKey {
    method: HttpMethod,
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Other(String),
}

impl From<&str> for HttpMethod {
    fn from(s: &str) -> Self {
        match s {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "PATCH" => HttpMethod::Patch,
            other => HttpMethod::Other(other.to_string()),
        }
    }
}

type RoutesMap = HashMap<RouteKey, Function, RandomState>;

#[derive(Clone)]
pub struct InvocationService {
    trigger_repository: Arc<dyn TriggerRepository>,
    function_repository: Arc<dyn FunctionRepository>,
    runtime: Arc<dyn WasmRuntime>,
    routes: Arc<RoutesMap>,
}

impl InvocationService {
    pub fn new(
        trigger_repository: Arc<dyn TriggerRepository>,
        function_repository: Arc<dyn FunctionRepository>,
        runtime: Arc<dyn WasmRuntime>,
    ) -> Self {
        Self {
            trigger_repository,
            function_repository,
            runtime,
            routes: Arc::new(HashMap::builder().hasher(RandomState::new()).build()),
        }
    }

    pub async fn load_routes(&self) -> Result<(), DomainError> {
        let triggers = self.trigger_repository.find_all().await?;
        let routes = &self.routes;

        let mut count = 0;
        let pin = routes.pin();

        for t in triggers {
            if let Some(mut func) = self
                .function_repository
                .find_by_name(&t.function_name)
                .await?
            {
                func.runtime = Some(self.runtime.clone());

                let key = RouteKey {
                    method: HttpMethod::from(t.method.as_str()),
                    path: t.path,
                };

                pin.insert(key, func);
                count += 1;
            } else {
                warn!(
                    "Trigger {} points to missing function {}",
                    t.name, t.function_name
                );
            }
        }
        info!("Loaded {} HTTP routes into memory", count);
        Ok(())
    }

    #[instrument(skip(self, request), fields(function_name, function_status))]
    pub async fn invoke_http(
        &self,
        request: WasmFunctionRequest,
    ) -> Result<WasmInvocation, DomainError> {
        let key = RouteKey {
            method: HttpMethod::from(request.method.as_str()),
            path: request.path.clone(),
        };

        let function = { self.routes.pin().get(&key).cloned() };

        if let Some(func) = function {
            tracing::Span::current().record("function_name", &func.name);

            if let Some(rt) = &func.runtime {
                info!(function_name = func.name, "Function {} started", func.name);
                let start = Instant::now();
                let result = Self::invoke_with_lazy_load(rt.clone(), &func, &request).await;
                let duration_ms = start.elapsed().as_millis() as u64;

                let meter = global::meter("fluor-api");
                let counter = meter.u64_counter("function_invocations").build();
                let histogram = meter.u64_histogram("function_duration_ms").build();

                let status = match &result {
                    Ok(_) => {
                        info!(
                            function_name = func.name,
                            "Function {} exited with status ok", func.name
                        );
                        "ok"
                    }
                    Err(e) => {
                        info!(
                            function_name = func.name,
                            "Function {} exited with error: {}", func.name, e
                        );
                        "error"
                    }
                };
                tracing::Span::current().record("function.status", status);

                let attrs = [
                    KeyValue::new("function_name", func.name.clone()),
                    KeyValue::new("status", status.to_string()),
                ];

                counter.add(1, &attrs);
                histogram.record(duration_ms, &attrs);

                result.map_err(|e| {
                    error!(error = %e, "Function invocation failed");
                    DomainError::Internal(e.to_string())
                })
            } else {
                error!("Runtime detached for function {}", func.name);
                Err(DomainError::Internal("Runtime detached".into()))
            }
        } else {
            Err(DomainError::NotFound("Route not found".into()))
        }
    }

    async fn invoke_with_lazy_load(
        runtime: Arc<dyn WasmRuntime>,
        function: &Function,
        request: &WasmFunctionRequest,
    ) -> anyhow::Result<WasmInvocation> {
        match runtime.invoke(&function.name, request).await {
            Ok(result) => Ok(result),
            Err(first_error)
                if !function.executable.is_empty()
                    && first_error
                        .to_string()
                        .contains(&format!("Function '{}' not found", function.name)) =>
            {
                let precompiled_path = Path::new(&function.executable).with_extension("cwasm");

                if !precompiled_path.exists() {
                    info!(
                        function_name = function.name,
                        precompiled_path = %precompiled_path.display(),
                        "Function is still warming up; precompiled artifact not ready"
                    );
                    return Err(anyhow::anyhow!(
                        "Function '{}' is warming up; try again shortly",
                        function.name
                    ));
                }

                info!(
                    function_name = function.name,
                    precompiled_path = %precompiled_path.display(),
                    "Function not preloaded yet; loading precompiled artifact on demand"
                );
                let runtime_for_load = runtime.clone();
                let name = function.name.clone();
                let executable = function.executable.clone();

                task::spawn_blocking(move || runtime_for_load.load_function(&name, &executable))
                    .await
                    .map_err(|error| {
                        anyhow::anyhow!("failed to join lazy load task: {}", error)
                    })??;

                runtime.invoke(&function.name, request).await
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Trigger;
    use crate::domain::ports::{MockFunctionRepository, MockTriggerRepository};
    use crate::domain::wasm_runtime::MockWasmRuntime;
    use mockall::predicate::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_load_routes_and_invoke() {
        let mut trigger_repo = MockTriggerRepository::new();
        let mut function_repo = MockFunctionRepository::new();
        let mut runtime = MockWasmRuntime::new();

        // Setup data
        let trigger = Trigger {
            name: "test-trigger".to_string(),
            function_name: "test-func".to_string(),
            method: "POST".to_string(),
            path: "/test".to_string(),
            readonly: false,
        };

        let function = Function {
            name: "test-func".to_string(),
            runtime: None, // Will be set by service
            ..Default::default()
        };

        // Expectations
        trigger_repo
            .expect_find_all()
            .returning(move || Ok(vec![trigger.clone()]));

        function_repo
            .expect_find_by_name()
            .with(eq("test-func"))
            .returning(move |_| Ok(Some(function.clone())));

        runtime
            .expect_invoke()
            .withf(|name, request| {
                name == "test-func"
                    && request.method == "POST"
                    && request.path == "/test"
                    && request.body == "body"
            })
            .returning(|_, _| {
                Ok(Ok(crate::domain::wasm_runtime::WasmFunctionResponse {
                    status: 200,
                    headers: vec![],
                    body: "response".to_string(),
                }))
            });

        let service = InvocationService::new(
            Arc::new(trigger_repo),
            Arc::new(function_repo),
            Arc::new(runtime),
        );

        // 1. Load routes
        let result = service.load_routes().await;
        assert!(result.is_ok());

        // 2. Invoke
        let result = service
            .invoke_http(WasmFunctionRequest {
                method: "POST".to_string(),
                path: "/test".to_string(),
                query: None,
                headers: vec![],
                body: "body".to_string(),
            })
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().unwrap().body, "response");
    }

    #[tokio::test]
    async fn test_invoke_not_found() {
        let trigger_repo = MockTriggerRepository::new();
        let function_repo = MockFunctionRepository::new();
        let runtime = MockWasmRuntime::new();

        let service = InvocationService::new(
            Arc::new(trigger_repo),
            Arc::new(function_repo),
            Arc::new(runtime),
        );

        let result = service
            .invoke_http(WasmFunctionRequest {
                method: "GET".to_string(),
                path: "/unknown".to_string(),
                query: None,
                headers: vec![],
                body: String::new(),
            })
            .await;
        assert!(matches!(result, Err(DomainError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_invoke_lazy_loads_when_runtime_is_not_ready() {
        let mut trigger_repo = MockTriggerRepository::new();
        let mut function_repo = MockFunctionRepository::new();
        let mut runtime = MockWasmRuntime::new();

        let trigger = Trigger {
            name: "lazy-trigger".to_string(),
            function_name: "lazy-func".to_string(),
            method: "POST".to_string(),
            path: "/lazy".to_string(),
            readonly: false,
        };

        let temp = tempdir().unwrap();
        let wasm_path = temp.path().join("lazy.wasm");
        let cwasm_path = temp.path().join("lazy.cwasm");
        std::fs::write(&wasm_path, b"wasm").unwrap();
        std::fs::write(&cwasm_path, b"compiled").unwrap();

        let function = Function {
            name: "lazy-func".to_string(),
            executable: wasm_path.display().to_string(),
            runtime: None,
            ..Default::default()
        };

        trigger_repo
            .expect_find_all()
            .returning(move || Ok(vec![trigger.clone()]));

        function_repo
            .expect_find_by_name()
            .with(eq("lazy-func"))
            .returning(move |_| Ok(Some(function.clone())));

        {
            let mut seq = mockall::Sequence::new();

            runtime
                .expect_invoke()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|name, _| Err(anyhow::anyhow!("Function '{}' not found", name)));

            runtime
                .expect_load_function()
                .times(1)
                .in_sequence(&mut seq)
                .with(eq("lazy-func"), eq(wasm_path.display().to_string()))
                .returning(|_, _| Ok(()));

            runtime
                .expect_invoke()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| {
                    Ok(Ok(crate::domain::wasm_runtime::WasmFunctionResponse {
                        status: 200,
                        headers: vec![],
                        body: "lazy-ok".to_string(),
                    }))
                });
        }

        let service = InvocationService::new(
            Arc::new(trigger_repo),
            Arc::new(function_repo),
            Arc::new(runtime),
        );

        service.load_routes().await.unwrap();

        let result = service
            .invoke_http(WasmFunctionRequest {
                method: "POST".to_string(),
                path: "/lazy".to_string(),
                query: None,
                headers: vec![],
                body: "{}".to_string(),
            })
            .await
            .unwrap()
            .unwrap();

        assert_eq!(result.body, "lazy-ok");
    }

    #[tokio::test]
    async fn test_invoke_returns_warming_up_when_precompiled_artifact_is_missing() {
        let mut trigger_repo = MockTriggerRepository::new();
        let mut function_repo = MockFunctionRepository::new();
        let mut runtime = MockWasmRuntime::new();

        let temp = tempdir().unwrap();
        let wasm_path = temp.path().join("warming-up.wasm");
        std::fs::write(&wasm_path, b"wasm").unwrap();

        let trigger = Trigger {
            name: "warming-up-trigger".to_string(),
            function_name: "warming-up-func".to_string(),
            method: "POST".to_string(),
            path: "/warming-up".to_string(),
            readonly: false,
        };

        let function = Function {
            name: "warming-up-func".to_string(),
            executable: wasm_path.display().to_string(),
            runtime: None,
            ..Default::default()
        };

        trigger_repo
            .expect_find_all()
            .returning(move || Ok(vec![trigger.clone()]));

        function_repo
            .expect_find_by_name()
            .with(eq("warming-up-func"))
            .returning(move |_| Ok(Some(function.clone())));

        runtime
            .expect_invoke()
            .times(1)
            .returning(|name, _| Err(anyhow::anyhow!("Function '{}' not found", name)));

        runtime.expect_load_function().times(0);

        let service = InvocationService::new(
            Arc::new(trigger_repo),
            Arc::new(function_repo),
            Arc::new(runtime),
        );

        service.load_routes().await.unwrap();

        let result = service
            .invoke_http(WasmFunctionRequest {
                method: "POST".to_string(),
                path: "/warming-up".to_string(),
                query: None,
                headers: vec![],
                body: "{}".to_string(),
            })
            .await;

        assert!(
            matches!(result, Err(DomainError::Internal(message)) if message.contains("warming up"))
        );
    }
}
