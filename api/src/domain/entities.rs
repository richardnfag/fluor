use crate::domain::wasm_runtime::WasmRuntime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Python,
    Rust,
    Go,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Function {
    pub name: String,
    pub language: Language,
    pub executable: String,
    pub cpu: String,
    pub memory: String,
    #[serde(skip)]
    pub runtime: Option<Arc<dyn WasmRuntime>>,
    #[serde(default)]
    pub readonly: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trigger {
    pub name: String,
    pub method: String,
    pub path: String,
    #[serde(rename = "function")]
    pub function_name: String,
    #[serde(default)]
    pub readonly: bool,
}

// Domain Error
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Entity not found: {0}")]
    NotFound(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Already exists: {0}")]
    AlreadyExists(String),
}
