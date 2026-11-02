wit_bindgen::generate!({
    path: "wit",
    world: "function",
});

struct Component;

impl exports::fluor::fun::handler::Guest for Component {
    fn handle(
        request: exports::fluor::fun::handler::Request,
    ) -> Result<exports::fluor::fun::handler::Response, exports::fluor::fun::handler::Error> {
        if request.path != "/healthz" {
            return Err(exports::fluor::fun::handler::Error {
                code: exports::fluor::fun::handler::ErrorCode::NotFound,
                message: format!("Unknown route: {}", request.path),
            });
        }

        let headers = vec![exports::fluor::fun::handler::Header {
            name: "content-type".to_string(),
            value: "application/json".to_string(),
        }];

        Ok(exports::fluor::fun::handler::Response {
            status: 200,
            headers,
            body: r#"{"status":"ok"}"#.to_string(),
        })
    }
}

export!(Component);
