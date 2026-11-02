use crate::application::invocation_service::InvocationService;
use crate::domain::wasm_runtime::{WasmFunctionRequest, WasmHeader};
use actix_web::http::{StatusCode, header::HeaderName, header::HeaderValue};
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use std::sync::Arc;

pub async fn gateway(
    req: HttpRequest,
    body: String,
    service: web::Data<Arc<InvocationService>>,
) -> impl Responder {
    let path = req.path().strip_prefix("/function").unwrap_or(req.path());
    let query = (!req.query_string().is_empty()).then(|| req.query_string().to_string());
    let headers = req
        .headers()
        .iter()
        .map(|(name, value)| WasmHeader {
            name: name.as_str().to_string(),
            value: String::from_utf8_lossy(value.as_bytes()).into_owned(),
        })
        .collect();

    let invocation = WasmFunctionRequest {
        method: req.method().as_str().to_string(),
        path: path.to_string(),
        query,
        headers,
        body,
    };

    match service.invoke_http(invocation).await {
        Ok(Ok(res)) => {
            let status =
                StatusCode::from_u16(res.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let mut builder = HttpResponse::build(status);

            for header in res.headers {
                let Ok(name) = HeaderName::try_from(header.name) else {
                    continue;
                };
                let Ok(value) = HeaderValue::try_from(header.value) else {
                    continue;
                };
                builder.append_header((name, value));
            }

            builder.body(res.body)
        }
        Ok(Err(err)) => {
            let status = StatusCode::from_u16(err.http_status())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            HttpResponse::build(status).json(err)
        }
        Err(crate::domain::entities::DomainError::NotFound(_)) => {
            HttpResponse::NotFound().body("Function route not found")
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
