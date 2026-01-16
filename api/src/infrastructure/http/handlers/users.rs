use crate::application::auth_service::AuthService;
use crate::domain::entities::User;
use actix_web::{HttpResponse, Responder, get, web, HttpRequest};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct UserResponse {
    id: i64,
    name: String,
    email: String,
    role: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            name: user.name,
            email: user.email,
            role: user.role,
        }
    }
}

#[get("/me")]
pub async fn me(
    req: HttpRequest,
    service: web::Data<Arc<AuthService>>,
) -> impl Responder {
    let auth_header = match req.headers().get("Authorization") {
        Some(h) => h,
        None => return HttpResponse::Unauthorized().body("Missing Authorization header"),
    };

    let token_str = match auth_header.to_str() {
        Ok(s) => s,
        Err(_) => return HttpResponse::Unauthorized().body("Invalid Authorization header"),
    };

    // Support "Bearer <token>" or just "<token>"
    let token = token_str.strip_prefix("Bearer ").unwrap_or(token_str).trim();

    match service.get_current_user(token).await {
        Ok(user) => HttpResponse::Ok().json(UserResponse::from(user)),
        Err(e) => HttpResponse::Unauthorized().body(e.to_string()),
    }
}

#[derive(serde::Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[actix_web::put("/me")]
pub async fn update_me(
    req: HttpRequest,
    body: web::Json<UpdateUserRequest>,
    service: web::Data<Arc<AuthService>>,
) -> impl Responder {
    let auth_header = match req.headers().get("Authorization") {
        Some(h) => h,
        None => return HttpResponse::Unauthorized().body("Missing Authorization header"),
    };

    let token_str = match auth_header.to_str() {
        Ok(s) => s,
        Err(_) => return HttpResponse::Unauthorized().body("Invalid Authorization header"),
    };

    let token = token_str.strip_prefix("Bearer ").unwrap_or(token_str).trim();

    match service.update_user(token, body.name.clone(), body.email.clone()).await {
        Ok(user) => HttpResponse::Ok().json(UserResponse::from(user)),
        Err(e) => match e {
            crate::domain::entities::DomainError::NotFound(_) => HttpResponse::NotFound().body(e.to_string()),
            crate::domain::entities::DomainError::ValidationError(_) => HttpResponse::BadRequest().body(e.to_string()),
            crate::domain::entities::DomainError::AlreadyExists(_) => HttpResponse::Conflict().body(e.to_string()),
            _ => HttpResponse::InternalServerError().body(e.to_string()),
        },
    }
}

#[derive(serde::Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[actix_web::put("/me/password")]
pub async fn change_password(
    req: HttpRequest,
    body: web::Json<ChangePasswordRequest>,
    service: web::Data<Arc<AuthService>>,
) -> impl Responder {
    let auth_header = match req.headers().get("Authorization") {
        Some(h) => h,
        None => return HttpResponse::Unauthorized().body("Missing Authorization header"),
    };

    let token_str = match auth_header.to_str() {
        Ok(s) => s,
        Err(_) => return HttpResponse::Unauthorized().body("Invalid Authorization header"),
    };

    let token = token_str.strip_prefix("Bearer ").unwrap_or(token_str).trim();

    match service.change_password(token, &body.current_password, &body.new_password).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "success": true })),
        Err(e) => match e {
            crate::domain::entities::DomainError::NotFound(_) => HttpResponse::NotFound().body(e.to_string()),
            crate::domain::entities::DomainError::ValidationError(_) => HttpResponse::BadRequest().body(e.to_string()),
            _ => HttpResponse::InternalServerError().body(e.to_string()),
        },
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(me);
    cfg.service(update_me);
    cfg.service(change_password);
}
