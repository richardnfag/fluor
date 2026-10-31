use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WasmHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WasmFunctionRequest {
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub headers: Vec<WasmHeader>,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WasmFunctionResponse {
    pub status: u16,
    pub headers: Vec<WasmHeader>,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WasmFunctionErrorCode {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    #[default]
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WasmFunctionError {
    pub code: WasmFunctionErrorCode,
    pub message: String,
}

impl WasmFunctionError {
    pub fn http_status(&self) -> u16 {
        match self.code {
            WasmFunctionErrorCode::BadRequest => 400,
            WasmFunctionErrorCode::Unauthorized => 401,
            WasmFunctionErrorCode::Forbidden => 403,
            WasmFunctionErrorCode::NotFound => 404,
            WasmFunctionErrorCode::Conflict => 409,
            WasmFunctionErrorCode::Internal => 500,
        }
    }
}

pub type WasmInvocation = Result<WasmFunctionResponse, WasmFunctionError>;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WasmRuntime: Send + Sync + std::fmt::Debug {
    fn load_function(&self, name: &str, path: &str) -> anyhow::Result<()>;
    async fn invoke(
        &self,
        function_name: &str,
        request: &WasmFunctionRequest,
    ) -> anyhow::Result<WasmInvocation>;
}
