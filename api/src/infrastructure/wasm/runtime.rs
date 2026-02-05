use crate::domain::wasm_runtime::WasmRuntime;
use ahash::RandomState;
use async_trait::async_trait;
use papaya::HashMap;
use std::sync::Arc;
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
    package fluor:fun;

    world function {
        export handle: func(input: string) -> string;
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
        config.async_support(true);

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
}

#[async_trait]
impl WasmRuntime for WasmtimeRuntime {
    fn load_function(&self, name: &str, path: &str) -> anyhow::Result<()> {
        let bytes = std::fs::read(path)?;
        let component = Component::new(&self.engine, &bytes)?;
        let instance_pre = self.linker.instantiate_pre(&component)?;

        self.cache.pin().insert(name.to_string(), instance_pre);

        // Warmup
        let runtime = self.clone();
        let name_cp = name.to_string();

        tokio::spawn(async move {
            let _ = runtime.invoke(&name_cp, "{}").await;
            tracing::info!(function = %name_cp, "Warmup concluído.");
        });

        Ok(())
    }

    async fn invoke(&self, function_name: &str, input: &str) -> anyhow::Result<String> {
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

        let result = bindings.call_handle(&mut store, input).await?;

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
