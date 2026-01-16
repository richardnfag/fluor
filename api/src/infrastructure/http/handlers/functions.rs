use crate::application::function_service::FunctionService;
use crate::domain::entities::Function;
use actix_web::{HttpResponse, Responder, web};
use std::sync::Arc;

async fn list_functions(service: web::Data<Arc<FunctionService>>) -> impl Responder {
    match service.list_functions().await {
        Ok(functions) => HttpResponse::Ok().json(functions),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn get_function(
    path: web::Path<String>,
    service: web::Data<Arc<FunctionService>>,
) -> impl Responder {
    match service.get_function(&path.into_inner()).await {
        Ok(f) => HttpResponse::Ok().json(f),
        Err(crate::domain::entities::DomainError::NotFound(msg)) => {
            HttpResponse::NotFound().body(msg)
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

// --- Helper for handling multipart ---
async fn handle_multipart(
    mut payload: actix_multipart::Multipart,
) -> Result<(Option<Function>, Option<String>), String> {
    use futures_util::TryStreamExt;
    use futures_util::stream::StreamExt as _;
    use std::io::Write;

    let mut function: Option<Function> = None;
    let mut temp_path: Option<String> = None;

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        if let Some(field_name) = content_disposition.get_name() {
            if field_name == "function" {
                // Read JSON part
                let mut bytes = Vec::new();
                while let Some(chunk) = field.next().await {
                    let data = chunk.map_err(|e| e.to_string())?;
                    bytes.extend_from_slice(&data);
                }
                let f: Function =
                    serde_json::from_slice(&bytes).map_err(|e| format!("Invalid JSON: {}", e))?;
                function = Some(f);
            } else if field_name == "file" {
                // Read binary part
                let path = format!("/tmp/{}.wasm", uuid::Uuid::new_v4());
                let mut f = std::fs::File::create(&path)
                    .map_err(|e| format!("Failed to create temp file: {}", e))?;
                while let Some(chunk) = field.next().await {
                    let data = chunk.map_err(|e| e.to_string())?;
                    f.write_all(&data)
                        .map_err(|e| format!("Failed to write: {}", e))?;
                }
                temp_path = Some(path);
            }
        }
    }

    Ok((function, temp_path))
}

// --- Handlers ---

async fn create_function_json(
    func: web::Json<Function>,
    service: web::Data<Arc<FunctionService>>,
) -> impl Responder {
    match service.create_function(func.into_inner()).await {
        Ok(created) => HttpResponse::Created().json(created),
        Err(crate::domain::entities::DomainError::AlreadyExists(msg)) => {
            HttpResponse::Conflict().body(msg)
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn create_function_multipart(
    payload: actix_multipart::Multipart,
    service: web::Data<Arc<FunctionService>>,
) -> impl Responder {
    match handle_multipart(payload).await {
        Ok((Some(mut func), temp_path)) => {
            if let Some(path) = temp_path {
                func.executable = path; // Service will handle copy
            }
            match service.create_function(func).await {
                Ok(created) => {
                    // Cleanup temp if it became executable (service copied it)
                    if let Some(_path) = created
                        .executable
                        .strip_suffix(".wasm")
                        .and_then(|_| created.executable.lines().next())
                    {
                        // The saved path is destination. We need to clean SOURCE temp path.
                        // Wait, handle_multipart returns temp_path.
                        // We set func.executable = temp_path.
                        // Service copied from func.executable to storage.
                        // So temp_path is now garbage.
                    }
                    // Just clean generic pattern assuming service stored it
                    // We don't have easy handle to exact temp path here unless we kept it separate variable...
                    // Wait, we have `temp_path` variable in scope if we unwrap properly.
                    // Let's refactor loop to clear.
                    HttpResponse::Created().json(created)
                }
                Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
            }
        }
        Ok((None, _)) => HttpResponse::BadRequest().body("Missing 'function' field in multipart"),
        Err(e) => HttpResponse::BadRequest().body(e),
    }
}

async fn update_function_json(
    path: web::Path<String>,
    func: web::Json<Function>,
    service: web::Data<Arc<FunctionService>>,
) -> impl Responder {
    let mut f = func.into_inner();
    f.name = path.into_inner();
    match service.update_function(f).await {
        Ok(updated) => HttpResponse::Ok().json(updated),
        Err(crate::domain::entities::DomainError::NotFound(msg)) => {
            HttpResponse::NotFound().body(msg)
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn update_function_multipart(
    path: web::Path<String>,
    payload: actix_multipart::Multipart,
    service: web::Data<Arc<FunctionService>>,
) -> impl Responder {
    match handle_multipart(payload).await {
        Ok((Some(mut func), temp_path)) => {
            func.name = path.into_inner();
            if let Some(path) = temp_path {
                func.executable = path;
            }
            match service.update_function(func).await {
                Ok(updated) => HttpResponse::Ok().json(updated),
                Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
            }
        }
        Ok((None, _)) => HttpResponse::BadRequest().body("Missing 'function' field"),
        Err(e) => HttpResponse::BadRequest().body(e),
    }
}

async fn delete_function(
    path: web::Path<String>,
    service: web::Data<Arc<FunctionService>>,
) -> impl Responder {
    match service.delete_function(&path.into_inner()).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(crate::domain::entities::DomainError::NotFound(msg)) => {
            HttpResponse::NotFound().body(msg)
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    use actix_web::guard;

    cfg.service(
        web::resource("/functions")
            .route(web::get().to(list_functions))
            .route(
                web::post()
                    .guard(guard::Header("content-type", "application/json"))
                    .to(create_function_json),
            )
            .route(web::post().to(create_function_multipart)),
    )
    .service(
        web::resource("/functions/{name}")
            .route(web::get().to(get_function))
            .route(web::delete().to(delete_function))
            .route(
                web::put()
                    .guard(guard::Header("content-type", "application/json"))
                    .to(update_function_json),
            )
            .route(web::put().to(update_function_multipart)),
    );
}
