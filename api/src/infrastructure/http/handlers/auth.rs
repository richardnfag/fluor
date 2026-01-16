use crate::application::auth_service::AuthService;
use actix_web::{HttpResponse, Responder, post, web};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[post("/login")]
pub async fn login(
    req: web::Json<LoginRequest>,
    service: web::Data<Arc<AuthService>>,
) -> impl Responder {
    match service.login(&req.email, &req.password).await {
        Ok(token) => HttpResponse::Ok().json(serde_json::json!({ "token": token })),
        Err(e) => HttpResponse::Unauthorized().body(e.to_string()),
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(login);
}
