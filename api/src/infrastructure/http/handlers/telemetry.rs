use crate::application::telemetry_service::TelemetryService;
use actix_web::{HttpResponse, web};
use serde_json::json;

pub async fn get_function_metrics(
    service: web::Data<TelemetryService>,
    path: web::Path<String>,
) -> HttpResponse {
    let function_name = path.into_inner();

    match service.get_function_metrics(&function_name).await {
        Ok(metrics) => HttpResponse::Ok().json(metrics),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e })),
    }
}

pub async fn get_function_logs(
    service: web::Data<TelemetryService>,
    path: web::Path<String>,
) -> HttpResponse {
    let function_name = path.into_inner();

    match service.get_function_logs(&function_name).await {
        Ok(logs) => HttpResponse::Ok().json(logs),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e })),
    }
}

pub async fn get_recent_logs(service: web::Data<TelemetryService>) -> HttpResponse {
    match service.get_recent_logs().await {
        Ok(logs) => HttpResponse::Ok().json(logs),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e })),
    }
}

pub async fn get_overall_metrics(service: web::Data<TelemetryService>) -> HttpResponse {
    match service.get_overall_metrics().await {
        Ok(metrics) => HttpResponse::Ok().json(metrics),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e })),
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/telemetry")
            .route(
                "/functions/{name}/metrics",
                web::get().to(get_function_metrics),
            )
            .route("/metrics/overall", web::get().to(get_overall_metrics))
            .route("/functions/{name}/logs", web::get().to(get_function_logs))
            .route("/logs", web::get().to(get_recent_logs)),
    );
}
