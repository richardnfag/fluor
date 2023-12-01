use serde_derive::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Function {
    name: String,
    language: String,
    source: String,
    method: String,
    path: String,
    cpu: String,
    memory: String,
    uptime: String,
}