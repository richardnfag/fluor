use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WasmRuntime: Send + Sync + std::fmt::Debug {
    fn load_function(&self, name: &str, path: &str) -> anyhow::Result<()>;
    async fn invoke(&self, function_name: &str, input: &str) -> anyhow::Result<String>;
}
