use anyhow::Context;
use std::path::PathBuf;
use std::time::Instant;
use wasmtime::component::{Component, Linker};
use wasmtime::{Cache, Config, Engine, Store};
use wasmtime_wasi::p2::add_to_linker_async;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    path: "../wit",
    world: "function",
    imports: { default: async | trappable },
    exports: { default: async },
});

struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let started = Instant::now();
    let component_path = std::env::args()
        .nth(1)
        .context("usage: cargo run -- <path-to-component.wasm>")?;
    let component_path = PathBuf::from(component_path);
    let precompiled_path = component_path.with_extension("cwasm");

    let mut config = Config::new();
    config.wasm_component_model(true);
    config.cache(Some(Cache::from_file(None)?));
    let _ = config.parallel_compilation(true);

    let engine = Engine::new(&config)?;
    eprintln!("engine_ready={:?}", started.elapsed());
    let mut linker = Linker::new(&engine);
    add_to_linker_async(&mut linker)?;

    let stdout = MemoryOutputPipe::new(4096);
    let stderr = MemoryOutputPipe::new(4096);

    let wasi = WasiCtxBuilder::new()
        .stdout(stdout.clone())
        .stderr(stderr.clone())
        .build();

    let mut store = Store::new(
        &engine,
        HostState {
            ctx: wasi,
            table: ResourceTable::new(),
        },
    );

    let read_started = Instant::now();
    let component = if precompiled_path.exists() {
        eprintln!("read_component={:?}", read_started.elapsed());
        let deserialize_started = Instant::now();
        let component = unsafe { Component::deserialize_file(&engine, &precompiled_path)? };
        eprintln!("deserialize_component={:?}", deserialize_started.elapsed());
        component
    } else {
        let bytes = std::fs::read(&component_path)?;
        eprintln!("read_component={:?}", read_started.elapsed());
        let compile_started = Instant::now();
        let component = Component::new(&engine, bytes)?;
        eprintln!("compile_component={:?}", compile_started.elapsed());
        let serialize_started = Instant::now();
        let serialized = component.serialize()?;
        std::fs::write(&precompiled_path, serialized)?;
        eprintln!("serialize_component={:?}", serialize_started.elapsed());
        component
    };
    let instantiate_started = Instant::now();
    let instance = linker.instantiate_async(&mut store, &component).await?;
    eprintln!("instantiate_component={:?}", instantiate_started.elapsed());

    let bindings_started = Instant::now();
    let bindings = Function::new(&mut store, &instance)?;
    eprintln!("bind_exports={:?}", bindings_started.elapsed());
    let request = exports::fluor::fun::handler::Request {
        method: "POST".to_string(),
        path: "/hello".to_string(),
        query: Some("source=poc".to_string()),
        headers: vec![exports::fluor::fun::handler::Header {
            name: "content-type".to_string(),
            value: "application/json".to_string(),
        }],
        body: "{\"name\":\"fluor\"}".to_string(),
    };

    let call_started = Instant::now();
    let response = bindings
        .interface0
        .call_handle(&mut store, &request)
        .await?;
    eprintln!("call_handle={:?}", call_started.elapsed());
    eprintln!("total={:?}", started.elapsed());

    println!("{response:#?}");
    Ok(())
}
