use crate::application::invocation_service::InvocationService;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use std::sync::Arc;

pub async fn gateway(
    req: HttpRequest,
    body: String,
    service: web::Data<Arc<InvocationService>>,
) -> impl Responder {
    let method = req.method().as_str();
    let path = req.path().strip_prefix("/function").unwrap_or(req.path());

    match service.invoke_http(method, path, &body).await {
        Ok(res) => HttpResponse::Ok().body(res),
        Err(crate::domain::entities::DomainError::NotFound(_)) => {
            HttpResponse::NotFound().body("Function route not found")
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
