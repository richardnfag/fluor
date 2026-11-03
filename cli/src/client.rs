use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub language: String,
    pub executable: String,
    pub cpu: String,
    pub memory: String,
    #[serde(default)]
    pub readonly: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Trigger {
    pub name: String,
    pub method: String,
    pub path: String,
    #[serde(rename = "function")]
    pub function: String,
    #[serde(default)]
    pub readonly: bool,
}

#[derive(Debug, Clone)]
pub struct NewFunction {
    pub name: String,
    pub language: String,
    pub wasm: PathBuf,
    pub cpu: String,
    pub memory: String,
}

#[derive(Debug, Clone, Default)]
pub struct FunctionPatch {
    pub language: Option<String>,
    pub wasm: Option<PathBuf>,
    pub cpu: Option<String>,
    pub memory: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewTrigger {
    pub name: String,
    pub method: String,
    pub path: String,
    pub function: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub status: u16,
    pub body: String,
}

pub struct Api {
    base: String,
    http: reqwest::Client,
}

impl Api {
    pub fn new(base: impl Into<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .context("failed to build the HTTP client")?;
        Ok(Self {
            base: base.into().trim_end_matches('/').to_string(),
            http,
        })
    }

    pub async fn list_functions(&self) -> Result<Vec<Function>> {
        self.get_json("/functions").await
    }

    pub async fn get_function(&self, name: &str) -> Result<Function> {
        self.get_json(&format!("/functions/{}", path_segment(name)))
            .await
    }

    pub async fn create_function(&self, function: &NewFunction) -> Result<Function> {
        let metadata = Function {
            name: function.name.clone(),
            language: function.language.clone(),
            executable: function
                .wasm
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("function.wasm")
                .to_string(),
            cpu: function.cpu.clone(),
            memory: function.memory.clone(),
            readonly: false,
        };
        self.send_function(
            reqwest::Method::POST,
            "/functions",
            &metadata,
            Some(&function.wasm),
        )
        .await
    }

    pub async fn update_function(&self, name: &str, patch: &FunctionPatch) -> Result<Function> {
        let mut current = self.get_function(name).await?;
        if let Some(language) = &patch.language {
            current.language.clone_from(language);
        }
        if let Some(cpu) = &patch.cpu {
            current.cpu.clone_from(cpu);
        }
        if let Some(memory) = &patch.memory {
            current.memory.clone_from(memory);
        }
        let path = format!("/functions/{}", path_segment(name));
        self.send_function(reqwest::Method::PUT, &path, &current, patch.wasm.as_deref())
            .await
    }

    pub async fn delete_function(&self, name: &str) -> Result<()> {
        self.delete(&format!("/functions/{}", path_segment(name)))
            .await
    }

    pub async fn list_triggers(&self) -> Result<Vec<Trigger>> {
        self.get_json("/triggers").await
    }

    pub async fn create_trigger(&self, trigger: &NewTrigger) -> Result<Trigger> {
        let body = Trigger {
            name: trigger.name.clone(),
            method: trigger.method.clone(),
            path: trigger.path.clone(),
            function: trigger.function.clone(),
            readonly: false,
        };
        let response = self
            .http
            .post(self.url("/triggers"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_vec(&body)?)
            .send()
            .await
            .with_context(|| format!("failed to reach {}", self.url("/triggers")))?;
        decode(response).await
    }

    pub async fn delete_trigger(&self, name: &str) -> Result<()> {
        self.delete(&format!("/triggers/{}", path_segment(name)))
            .await
    }

    pub async fn invoke(
        &self,
        path: &str,
        method: &str,
        query: Option<&str>,
        headers: &[(String, String)],
        body: &str,
    ) -> Result<Invocation> {
        let url = crate::gateway_url(&self.base, path, query);
        let http_method = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| anyhow!("invalid HTTP method '{method}'"))?;
        let mut request = self.http.request(http_method, &url).body(body.to_string());
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request
            .send()
            .await
            .with_context(|| format!("failed to reach {url}"))?;
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .context("failed to read the response")?;
        Ok(Invocation { status, body })
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let response = self
            .http
            .get(self.url(path))
            .send()
            .await
            .with_context(|| format!("failed to reach {}", self.url(path)))?;
        decode(response).await
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let response = self
            .http
            .delete(self.url(path))
            .send()
            .await
            .with_context(|| format!("failed to reach {}", self.url(path)))?;
        let response = successful(response).await?;
        response
            .bytes()
            .await
            .context("failed to read the response")?;
        Ok(())
    }

    async fn send_function(
        &self,
        method: reqwest::Method,
        path: &str,
        metadata: &Function,
        wasm: Option<&Path>,
    ) -> Result<Function> {
        let url = self.url(path);
        let request = if let Some(wasm) = wasm {
            if !wasm.is_file() {
                return Err(anyhow!("wasm file not found: {}", wasm.display()));
            }
            let metadata = serde_json::to_string(metadata)?;
            let form = reqwest::multipart::Form::new()
                .text("function", metadata)
                .file("file", wasm)
                .await
                .with_context(|| format!("failed to read {}", wasm.display()))?;
            self.http.request(method, &url).multipart(form)
        } else {
            self.http
                .request(method, &url)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(serde_json::to_vec(metadata)?)
        };
        let response = request
            .send()
            .await
            .with_context(|| format!("failed to reach {url}"))?;
        decode(response).await
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }
}

pub(crate) fn path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

async fn decode<T: for<'de> Deserialize<'de>>(response: reqwest::Response) -> Result<T> {
    let response = successful(response).await?;
    let body = response
        .text()
        .await
        .context("failed to read the response")?;
    serde_json::from_str(&body).with_context(|| format!("invalid JSON response: {body}"))
}

async fn successful(response: reqwest::Response) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    Err(anyhow!("{status}: {body}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample() -> Function {
        Function {
            name: "sample".to_string(),
            language: "rust".to_string(),
            executable: "sample.wasm".to_string(),
            cpu: "100m".to_string(),
            memory: "128Mi".to_string(),
            readonly: false,
        }
    }

    #[tokio::test]
    async fn lists_functions() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/functions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(vec![sample()]))
            .mount(&server)
            .await;

        let api = Api::new(server.uri()).unwrap();
        let functions = api.list_functions().await.unwrap();
        assert_eq!(functions, vec![sample()]);
    }

    #[tokio::test]
    async fn uploads_a_component_on_create() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/functions"))
            .and(body_string_contains("\"name\":\"sample\""))
            .and(body_string_contains("name=\"file\""))
            .respond_with(ResponseTemplate::new(201).set_body_json(sample()))
            .mount(&server)
            .await;

        let mut wasm = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut wasm, b"\0asm").unwrap();
        let api = Api::new(server.uri()).unwrap();
        let created = api
            .create_function(&NewFunction {
                name: "sample".to_string(),
                language: "rust".to_string(),
                wasm: wasm.path().to_path_buf(),
                cpu: "100m".to_string(),
                memory: "128Mi".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(created.name, "sample");
    }

    #[tokio::test]
    async fn keeps_the_current_component_when_updating_metadata() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/functions/sample"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample()))
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/functions/sample"))
            .and(body_string_contains("\"memory\":\"256Mi\""))
            .and(body_string_contains("\"executable\":\"sample.wasm\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(Function {
                memory: "256Mi".to_string(),
                ..sample()
            }))
            .mount(&server)
            .await;

        let api = Api::new(server.uri()).unwrap();
        let updated = api
            .update_function(
                "sample",
                &FunctionPatch {
                    memory: Some("256Mi".to_string()),
                    ..FunctionPatch::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.memory, "256Mi");
    }

    #[tokio::test]
    async fn reports_the_server_status_and_body() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/functions/missing"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Entity not found: missing"))
            .mount(&server)
            .await;

        let api = Api::new(server.uri()).unwrap();
        let error = api.delete_function("missing").await.unwrap_err();
        let message = error.to_string();
        assert!(message.contains("404"));
        assert!(message.contains("missing"));
    }
}
