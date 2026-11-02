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
    Csharp,
    Javascript,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::Rust => "rust",
            Language::Go => "go",
            Language::Csharp => "csharp",
            Language::Javascript => "javascript",
        }
    }
}

impl std::str::FromStr for Language {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "python" => Ok(Language::Python),
            "rust" => Ok(Language::Rust),
            "go" => Ok(Language::Go),
            "csharp" | "c#" => Ok(Language::Csharp),
            "javascript" | "js" => Ok(Language::Javascript),
            _ => Err(()),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::Language;
    use std::str::FromStr;

    #[test]
    fn parses_supported_languages() {
        assert_eq!(Language::from_str("python"), Ok(Language::Python));
        assert_eq!(Language::from_str("rust"), Ok(Language::Rust));
        assert_eq!(Language::from_str("go"), Ok(Language::Go));
        assert_eq!(Language::from_str("csharp"), Ok(Language::Csharp));
        assert_eq!(Language::from_str("C#"), Ok(Language::Csharp));
        assert_eq!(Language::from_str("javascript"), Ok(Language::Javascript));
        assert_eq!(Language::from_str("js"), Ok(Language::Javascript));
        assert!(Language::from_str("moonbit").is_err());
        assert!(Language::from_str("unknown").is_err());
    }
}
