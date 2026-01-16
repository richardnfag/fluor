use crate::domain::entities::{DomainError, Function};
use crate::domain::ports::{FunctionRepository, TriggerRepository};
use crate::domain::wasm_runtime::WasmRuntime;
use ahash::RandomState;
use opentelemetry::{KeyValue, global};
use papaya::HashMap;
use std::sync::Arc;
use std::time::Instant;
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

    #[instrument(skip(self, body), fields(function_name, function_status))]
    pub fn invoke_http(&self, method: &str, path: &str, body: &str) -> Result<String, DomainError> {
        let key = RouteKey {
            method: HttpMethod::from(method),
            path: path.to_string(),
        };

        let function = { self.routes.pin().get(&key).cloned() };

        if let Some(func) = function {
            tracing::Span::current().record("function_name", &func.name);

            if let Some(rt) = &func.runtime {
                info!(function_name = func.name, "Function {} started", func.name);
                let start = Instant::now();
                let result = rt.invoke(&func.name, body);
                let duration_ms = start.elapsed().as_millis() as u64;

                let meter = global::meter("fluor-api");
                let counter = meter.u64_counter("function_invocations").build();
                let histogram = meter.u64_histogram("function_duration_ms").build();

                let status = match &result {
                    Ok(_) => {
                        info!(function_name = func.name, "Function {} exited with status ok", func.name);
                        "ok"
                    },
                    Err(e) => {
                        info!(function_name = func.name, "Function {} exited with error: {}", func.name, e);
                        "error"
                    },
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Trigger;
    use crate::domain::ports::{MockFunctionRepository, MockTriggerRepository};
    use crate::domain::wasm_runtime::MockWasmRuntime;
    use mockall::predicate::*;

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
            .with(eq("test-func"), eq("body"))
            .returning(|_, _| Ok("response".to_string()));

        let service = InvocationService::new(
            Arc::new(trigger_repo),
            Arc::new(function_repo),
            Arc::new(runtime),
        );

        // 1. Load routes
        let result = service.load_routes().await;
        assert!(result.is_ok());

        // 2. Invoke
        let result = service.invoke_http("POST", "/test", "body");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "response");
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

        let result = service.invoke_http("GET", "/unknown", "");
        assert!(matches!(result, Err(DomainError::NotFound(_))));
    }
}
