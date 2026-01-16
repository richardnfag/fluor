use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use api::domain::wasm_runtime::WasmRuntime;
use std::sync::Arc;
use tracing::{error, info};

use api::application::{
    auth_service::AuthService, function_service::FunctionService,
    invocation_service::InvocationService, trigger_service::TriggerService,
};
use api::infrastructure::db::sqlite::SqliteRepository;
use api::infrastructure::wasm::runtime::WasmtimeRuntime;
use api::{application, infrastructure};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let port_str = std::env::var("PORT").unwrap_or("8080".to_string());

    // 0. Observability
    infrastructure::telemetry::init_telemetry().expect("Failed to init telemetry");
    info!(
        "Starting Fluor API (Hexagonal) at http://0.0.0.0:{}",
        port_str
    );

    // 1. infrastructure / Adapters
    let pool = infrastructure::db::sqlite::init_db().await;
    let repo = Arc::new(SqliteRepository::new(pool.clone()));

    let clickhouse_url =
        std::env::var("CLICKHOUSE_URL").unwrap_or("http://localhost:8123".to_string());
    let clickhouse_user = std::env::var("CLICKHOUSE_USER").unwrap_or("default".to_string());
    let clickhouse_password =
        std::env::var("CLICKHOUSE_PASSWORD").unwrap_or("password".to_string());
    let clickhouse_db = std::env::var("CLICKHOUSE_DB").unwrap_or("default".to_string());

    let clickhouse_repo = Arc::new(infrastructure::db::clickhouse::ClickHouseRepository::new(
        &clickhouse_url,
        &clickhouse_user,
        &clickhouse_password,
        &clickhouse_db,
    ));

    let runtime = Arc::new(WasmtimeRuntime::new().expect("Failed to init Wasmtime"));
    let wasm_storage_path = std::env::var("WASM_STORAGE_PATH").unwrap_or("wasm_data".to_string());

    let password_pepper = std::env::var("PASSWORD_PEPPER").expect("PASSWORD_PEPPER must be set");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    // 2. Application / Services
    let auth_service = Arc::new(AuthService::new(repo.clone(), password_pepper, jwt_secret));
    let function_service = Arc::new(FunctionService::new(
        repo.clone(),
        runtime.clone(),
        wasm_storage_path,
    ));
    let invocation_service = Arc::new(InvocationService::new(
        repo.clone(),
        repo.clone(),
        runtime.clone(),
    ));
    let trigger_service = Arc::new(TriggerService::new(
        repo.clone(),
        invocation_service.clone(),
    ));
    let telemetry_service =
        application::telemetry_service::TelemetryService::new(clickhouse_repo.clone());

    // 3. Bootstrap (Preload)
    if let Err(e) = invocation_service.load_routes().await {
        error!("Failed to load routes: {}", e);
    }

    if let Ok(funcs) = function_service.list_functions().await {
        for f in funcs {
            if !f.executable.is_empty() {
                if let Err(e) = runtime.load_function(&f.name, &f.executable) {
                    error!("Failed to preload function {}: {}", f.name, e);
                } else {
                    info!("Preloaded function {}", f.name);
                }
            }
        }
    }

    let port = std::env::var("PORT")
        .unwrap_or("8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);
    let host = std::env::var("HOST").unwrap_or("0.0.0.0".to_string());

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .app_data(web::Data::new(auth_service.clone()))
            .app_data(web::Data::new(function_service.clone()))
            .app_data(web::Data::new(trigger_service.clone()))
            .app_data(web::Data::new(invocation_service.clone()))
            .app_data(web::Data::new(telemetry_service.clone()))
            .wrap(cors)
            .configure(infrastructure::http::handlers::auth::config)
            .configure(infrastructure::http::handlers::functions::config)
            .configure(infrastructure::http::handlers::triggers::config)
            .configure(infrastructure::http::handlers::telemetry::config)
            .configure(infrastructure::http::handlers::users::config)
            .service(
                web::scope("/function")
                    .default_service(web::to(infrastructure::http::handlers::gateway::gateway)),
            )
    })
    .bind((host, port))?
    .run()
    .await?;

    infrastructure::telemetry::shutdown_telemetry();
    Ok(())
}
