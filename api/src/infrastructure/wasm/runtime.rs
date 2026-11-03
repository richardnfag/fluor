use crate::domain::wasm_runtime::{
    WasmFunctionError, WasmFunctionErrorCode, WasmFunctionRequest, WasmFunctionResponse,
    WasmHeader, WasmInvocation, WasmRuntime,
};
use ahash::RandomState;
use async_trait::async_trait;
use papaya::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tracing;
use wasmtime::component::{Component, InstancePre, Linker};
use wasmtime::{
    Config, Engine, InstanceAllocationStrategy, OptLevel, PoolingAllocationConfig, Store,
};
use wasmtime_wasi::p2::add_to_linker_async;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    inline: "
    package fluor:fun@0.2.0;

    interface handler {
        record header {
            name: string,
            value: string,
        }

        record request {
            method: string,
            path: string,
            query: option<string>,
            headers: list<header>,
            body: string,
        }

        record response {
            status: u16,
            headers: list<header>,
            body: string,
        }

        enum error-code {
            bad-request,
            unauthorized,
            forbidden,
            not-found,
            conflict,
            internal,
        }

        record error {
            code: error-code,
            message: string,
        }

        handle: func(request: request) -> result<response, error>;
    }

    world function {
        export handler;
    }
    ",
    world: "function",
    imports: { default: async | trappable },
    exports: { default: async },
});

struct FluorState {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for FluorState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

#[derive(Clone)]
pub struct WasmtimeRuntime {
    engine: Engine,
    linker: Arc<Linker<FluorState>>,
    cache: Arc<HashMap<String, InstancePre<FluorState>, RandomState>>,
}
impl WasmtimeRuntime {
    pub fn new() -> anyhow::Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);

        config.async_stack_size(4 * 1024 * 1024);
        config.memory_guard_size(4 * 1024 * 1024);

        let mut pool_config = PoolingAllocationConfig::default();

        pool_config.max_component_instance_size(256 * 1024 * 1024); // 256MB
        pool_config.total_component_instances(5000);
        pool_config.max_tables_per_component(20);

        pool_config.max_unused_warm_slots(2000);

        config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool_config));
        config.cranelift_opt_level(OptLevel::Speed);

        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        add_to_linker_async(&mut linker)?;

        let cache = Arc::new(HashMap::builder().hasher(RandomState::new()).build());

        Ok(Self {
            engine,
            linker: Arc::new(linker),
            cache,
        })
    }

    fn precompiled_path(wasm_path: &Path) -> PathBuf {
        wasm_path.with_extension("cwasm")
    }

    fn should_recompile(wasm_path: &Path, precompiled_path: &Path) -> bool {
        if !precompiled_path.exists() {
            return true;
        }

        let wasm_mtime = std::fs::metadata(wasm_path)
            .and_then(|meta| meta.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let precompiled_mtime = std::fs::metadata(precompiled_path)
            .and_then(|meta| meta.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        precompiled_mtime < wasm_mtime
    }

    fn compile_component(
        &self,
        wasm_path: &Path,
        precompiled_path: &Path,
    ) -> anyhow::Result<Component> {
        let component = Component::from_file(&self.engine, wasm_path)?;
        let serialized = component.serialize()?;
        let temp_path = precompiled_path.with_extension("cwasm.tmp");
        std::fs::write(&temp_path, serialized)?;
        std::fs::rename(&temp_path, precompiled_path)?;
        Ok(component)
    }

    fn load_component(&self, wasm_path: &Path) -> anyhow::Result<Component> {
        let precompiled_path = Self::precompiled_path(wasm_path);

        if Self::should_recompile(wasm_path, &precompiled_path) {
            return self.compile_component(wasm_path, &precompiled_path);
        }

        match unsafe { Component::deserialize_file(&self.engine, &precompiled_path) } {
            Ok(component) => Ok(component),
            Err(error) => {
                tracing::warn!(
                    wasm_path = %wasm_path.display(),
                    precompiled_path = %precompiled_path.display(),
                    error = %error,
                    "Failed to deserialize precompiled component; recompiling from source"
                );
                let _ = std::fs::remove_file(&precompiled_path);
                self.compile_component(wasm_path, &precompiled_path)
            }
        }
    }
}

#[async_trait]
impl WasmRuntime for WasmtimeRuntime {
    fn load_function(&self, name: &str, path: &str) -> anyhow::Result<()> {
        let component = self.load_component(Path::new(path))?;
        let instance_pre = self.linker.instantiate_pre(&component)?;

        self.cache.pin().insert(name.to_string(), instance_pre);

        Ok(())
    }

    async fn invoke(
        &self,
        function_name: &str,
        request: &WasmFunctionRequest,
    ) -> anyhow::Result<WasmInvocation> {
        let instance_pre = {
            let guard = self.cache.pin();
            guard
                .get(function_name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Function '{}' not found", function_name))?
        };

        let stdout = MemoryOutputPipe::new(4096);
        let stderr = MemoryOutputPipe::new(4096);

        let wasi = WasiCtxBuilder::new()
            .stdout(stdout.clone())
            .stderr(stderr.clone())
            .build();

        let state = FluorState {
            ctx: wasi,
            table: ResourceTable::new(),
        };

        let mut store = Store::new(&self.engine, state);

        let instance = instance_pre.instantiate_async(&mut store).await?;
        let bindings = Function::new(&mut store, &instance)?;

        let request = exports::fluor::fun::handler::Request {
            method: request.method.clone(),
            path: request.path.clone(),
            query: request.query.clone(),
            headers: request
                .headers
                .iter()
                .map(|header| exports::fluor::fun::handler::Header {
                    name: header.name.clone(),
                    value: header.value.clone(),
                })
                .collect(),
            body: request.body.clone(),
        };

        let result = bindings
            .interface0
            .call_handle(&mut store, &request)
            .await?;

        let stdout_content = stdout.contents();
        if !stdout_content.is_empty() {
            let log_body = String::from_utf8_lossy(&stdout_content);
            tracing::info!(function_name = %function_name, "{}", log_body);
        }

        let stderr_content = stderr.contents();
        if !stderr_content.is_empty() {
            let log_body = String::from_utf8_lossy(&stderr_content);
            tracing::error!(function_name = %function_name, "{}", log_body);
        }

        Ok(match result {
            Ok(response) => Ok(WasmFunctionResponse {
                status: response.status,
                headers: response
                    .headers
                    .into_iter()
                    .map(|header| WasmHeader {
                        name: header.name,
                        value: header.value,
                    })
                    .collect(),
                body: response.body,
            }),
            Err(error) => Err(WasmFunctionError {
                code: match error.code {
                    exports::fluor::fun::handler::ErrorCode::BadRequest => {
                        WasmFunctionErrorCode::BadRequest
                    }
                    exports::fluor::fun::handler::ErrorCode::Unauthorized => {
                        WasmFunctionErrorCode::Unauthorized
                    }
                    exports::fluor::fun::handler::ErrorCode::Forbidden => {
                        WasmFunctionErrorCode::Forbidden
                    }
                    exports::fluor::fun::handler::ErrorCode::NotFound => {
                        WasmFunctionErrorCode::NotFound
                    }
                    exports::fluor::fun::handler::ErrorCode::Conflict => {
                        WasmFunctionErrorCode::Conflict
                    }
                    exports::fluor::fun::handler::ErrorCode::Internal => {
                        WasmFunctionErrorCode::Internal
                    }
                },
                message: error.message,
            }),
        })
    }
}

impl std::fmt::Debug for WasmtimeRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmtimeRuntime")
            .field("cache_size", &self.cache.pin().len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::WasmtimeRuntime;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn precompiled_path_uses_cwasm_extension() {
        let path = Path::new("/tmp/example-python.wasm");
        assert_eq!(
            WasmtimeRuntime::precompiled_path(path),
            Path::new("/tmp/example-python.cwasm")
        );
    }

    #[test]
    fn recompiles_when_precompiled_artifact_is_missing() {
        let temp = tempdir().unwrap();
        let wasm_path = temp.path().join("example.wasm");

        std::fs::write(&wasm_path, b"wasm").unwrap();

        assert!(WasmtimeRuntime::should_recompile(
            &wasm_path,
            &temp.path().join("example.cwasm")
        ));
    }
}
