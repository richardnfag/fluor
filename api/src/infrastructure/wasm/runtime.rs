use crate::domain::wasm_runtime::WasmRuntime;
use ahash::RandomState;
use papaya::HashMap;
use std::sync::Arc;
use tracing;
use wasmtime::component::{Component, Linker};
use wasmtime::{
    Config, Engine, InstanceAllocationStrategy, OptLevel, PoolingAllocationConfig, Store,
};
use wasmtime_wasi::pipe::MemoryOutputPipe;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

wasmtime::component::bindgen!({
    inline: "
    package fluor:fun;

    world function {
        export handle: func(input: string) -> string;
    }
    ",
    world: "function",
});

struct FluorState {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for FluorState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

#[derive(Clone)]
pub struct WasmtimeRuntime {
    engine: Engine,
    linker: Arc<Linker<FluorState>>,
    cache: Arc<HashMap<String, Component, RandomState>>,
}
impl WasmtimeRuntime {
    pub fn new() -> anyhow::Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(false);

        let mut pool_config = PoolingAllocationConfig::default();

        pool_config.max_component_instance_size(10 * 1024 * 1024); // 10MB
        pool_config.total_component_instances(1000);

        config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool_config));

        config.cranelift_opt_level(OptLevel::Speed);

        let engine = Engine::new(&config)?;

        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)?;

        let cache = Arc::new(HashMap::builder().hasher(RandomState::new()).build());

        Ok(Self {
            engine,
            linker: Arc::new(linker),
            cache,
        })
    }
}

impl WasmRuntime for WasmtimeRuntime {
    fn load_function(&self, name: &str, path: &str) -> anyhow::Result<()> {
        let bytes = std::fs::read(path)?;

        let component = Component::new(&self.engine, &bytes)?;

        self.cache.pin().insert(name.to_string(), component);
        Ok(())
    }

    fn invoke(&self, function_name: &str, input: &str) -> anyhow::Result<String> {
        let component = {
            let guard = self.cache.pin();
            match guard.get(function_name) {
                Some(c) => c.clone(),
                None => return Err(anyhow::anyhow!("Function '{}' not found", function_name)),
            }
        };

        // stdout and stderr capture
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

        // Measure execution time? InvocationService handles metrics.

        let bindings = Function::instantiate(&mut store, &component, &self.linker)?;

        let result = bindings.call_handle(&mut store, input)?;

        // Log stdout
        let stdout_content = stdout.contents();
        if !stdout_content.is_empty() {
            let log_body = String::from_utf8_lossy(&stdout_content);
            tracing::info!(function_name = %function_name, "{}", log_body);
        }

        // Log stderr
        let stderr_content = stderr.contents();
        if !stderr_content.is_empty() {
            let log_body = String::from_utf8_lossy(&stderr_content);
            tracing::error!(function_name = %function_name, "{}", log_body);
        }

        Ok(result)
    }
}

impl std::fmt::Debug for WasmtimeRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmtimeRuntime")
            .field("cache_size", &self.cache.pin().len())
            .finish()
    }
}
