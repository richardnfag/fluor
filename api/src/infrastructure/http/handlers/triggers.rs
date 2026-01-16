use crate::application::trigger_service::TriggerService;
use crate::domain::entities::Trigger;
use actix_web::{HttpResponse, Responder, delete, get, post, web};
use std::sync::Arc;

#[get("/triggers")]
async fn list_triggers(service: web::Data<Arc<TriggerService>>) -> impl Responder {
    match service.list_triggers().await {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[post("/triggers")]
async fn create_trigger(
    trigger: web::Json<Trigger>,
    service: web::Data<Arc<TriggerService>>,
) -> impl Responder {
    match service.create_trigger(trigger.into_inner()).await {
        Ok(created) => HttpResponse::Created().json(created),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[delete("/triggers/{name}")]
async fn delete_trigger(
    path: web::Path<String>,
    service: web::Data<Arc<TriggerService>>,
) -> impl Responder {
    match service.delete_trigger(&path.into_inner()).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(crate::domain::entities::DomainError::NotFound(msg)) => {
            HttpResponse::NotFound().body(msg)
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(list_triggers)
        .service(create_trigger)
        .service(delete_trigger);
}
