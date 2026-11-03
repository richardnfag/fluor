mod client;
mod output;
mod templates;

pub use client::{Api, FunctionPatch, Invocation, NewFunction, NewTrigger};
pub use output::{
    function_text, functions_text, template_text, templates_text, trigger_text, triggers_text,
};
pub use templates::{CreatedTemplate, Template, Templates, created_output, templates_output};

pub fn canonical_language(value: &str) -> Result<String, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "python" => Ok("python".to_string()),
        "rust" => Ok("rust".to_string()),
        "go" => Ok("go".to_string()),
        "csharp" | "c#" => Ok("csharp".to_string()),
        "javascript" | "js" => Ok("javascript".to_string()),
        _ => Err(format!(
            "unsupported language '{value}'. expected python, rust, go, csharp, or javascript"
        )),
    }
}

pub fn gateway_url(server: &str, path: &str, query: Option<&str>) -> String {
    let base = server.trim_end_matches('/');
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let mut url = format!("{base}/function{path}");
    if let Some(query) = query.map(str::trim).filter(|query| !query.is_empty()) {
        let query = query.trim_start_matches('?');
        url.push('?');
        url.push_str(query);
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_server_language_names() {
        assert_eq!(canonical_language("Rust").unwrap(), "rust");
        assert_eq!(canonical_language("C#").unwrap(), "csharp");
        assert_eq!(canonical_language("js").unwrap(), "javascript");
        assert!(canonical_language("moonbit").is_err());
    }

    #[test]
    fn builds_a_gateway_url() {
        assert_eq!(
            gateway_url(
                "http://127.0.0.1:8080/",
                "examples/rust",
                Some("?name=pencil")
            ),
            "http://127.0.0.1:8080/function/examples/rust?name=pencil"
        );
    }
}
