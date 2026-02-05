use actix_web::{App, test, web};
use api::application::auth_service::AuthService;
use api::application::function_service::FunctionService;
use api::application::invocation_service::InvocationService;
use api::application::telemetry_service::TelemetryService;
use api::application::trigger_service::TriggerService;
use api::domain::wasm_runtime::WasmRuntime;
use api::infrastructure::db::clickhouse::ClickHouseRepository;
use api::infrastructure::db::sqlite::{SqliteRepository, create_pool};
use api::infrastructure::http::handlers;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

// Mock Runtime for Integration Tests
#[derive(Debug)]
pub struct TestRuntime {
    functions: Mutex<HashMap<String, String>>, // name -> path
}

impl TestRuntime {
    pub fn new() -> Self {
        Self {
            functions: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl WasmRuntime for TestRuntime {
    fn load_function(&self, name: &str, wasm_path: &str) -> anyhow::Result<()> {
        let mut functions = self.functions.lock().unwrap();
        functions.insert(name.to_string(), wasm_path.to_string());
        Ok(())
    }

    async fn invoke(&self, name: &str, _params: &str) -> anyhow::Result<String> {
        let functions = self.functions.lock().unwrap();
        if functions.contains_key(name) {
            let resp = serde_json::json!({ "message": format!("Hello from {}", name) });
            Ok(serde_json::to_string(&resp)?)
        } else {
            Err(anyhow::anyhow!("Function {} not loaded", name))
        }
    }
}

async fn spawn_app() -> (
    impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    tempfile::TempDir,
) {
    // 1. Setup Data
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap());

    let pool = create_pool(db_url).await;
    let repo = Arc::new(SqliteRepository::new(pool.clone()));

    // Dummy ClickHouse (won't be used for critical path)
    let clickhouse_repo = Arc::new(ClickHouseRepository::new(
        "http://localhost:8123",
        "default",
        "password",
        "default",
    ));

    // Use Mock Runtime
    let runtime = Arc::new(TestRuntime::new());
    let wasm_storage_path = temp_dir.path().join("wasm").to_str().unwrap().to_string();
    std::fs::create_dir_all(&wasm_storage_path).unwrap();

    // 2. Services
    let auth_service = Arc::new(AuthService::new(
        repo.clone(),
        "secret".to_string(),
        "jwt_secret".to_string(),
    ));
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
    let telemetry_service = TelemetryService::new(clickhouse_repo);

    // 3. Init Service
    // Call seed functions
    api::infrastructure::db::sqlite::seed_data(&pool).await;
    api::infrastructure::db::sqlite::seed_admin(&pool, "admin@fluor.com", "admin", "secret").await; // New signature

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(auth_service))
            .app_data(web::Data::new(function_service))
            .app_data(web::Data::new(trigger_service))
            .app_data(web::Data::new(invocation_service))
            .app_data(web::Data::new(telemetry_service))
            .configure(handlers::auth::config)
            .configure(handlers::functions::config)
            .configure(handlers::triggers::config)
            .configure(handlers::telemetry::config)
            .service(web::scope("/function").default_service(web::to(handlers::gateway::gateway))),
    )
    .await;

    (app, temp_dir)
}

#[actix_rt::test]
async fn test_auth_flow() {
    let (app, _td) = spawn_app().await;

    // Login with seeded admin
    let payload = serde_json::json!({
        "email": "admin@fluor.com",
        "password": "admin"
    });

    let req = test::TestRequest::post()
        .uri("/login")
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    if !resp.status().is_success() {
        println!("Auth failed: {:?}", resp.status());
    }
    assert!(resp.status().is_success());
}

#[actix_rt::test]
async fn test_functions_full_crud() {
    let (app, _td) = spawn_app().await;

    // Create
    let payload = serde_json::json!({
        "name": "crud-func",
        "language": "python",
        "executable": "",
        "cpu": "0.1",
        "memory": "128"
    });
    let req = test::TestRequest::post()
        .uri("/functions")
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    eprintln!("Create Function (CRUD) status: {:?}", resp.status());
    if !resp.status().is_success() {
        // println!("Create Function failed: {:?}", resp.status());
    }
    assert!(resp.status().is_success());

    // Update
    let update_payload = serde_json::json!({
        "name": "crud-func", // ID
        "language": "python",
        "executable": "",
        "cpu": "0.2", // Changed
        "memory": "256"
    });
    let req = test::TestRequest::put()
        .uri("/functions/crud-func")
        .set_json(&update_payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Get
    let req = test::TestRequest::get()
        .uri("/functions/crud-func")
        .to_request();
    let resp: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(resp["cpu"], "0.2");

    // Delete
    let req = test::TestRequest::delete()
        .uri("/functions/crud-func")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Verify Gone
    let req = test::TestRequest::get()
        .uri("/functions/crud-func")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn test_invocation() {
    let (app, _td) = spawn_app().await;

    // Create a persistent dummy wasm file
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join(format!("test-{}.wasm", uuid::Uuid::new_v4()));
    std::fs::write(&path, "dummy wasm content").unwrap();
    let path_str = path.to_str().unwrap().to_string();

    // 1. Create Function (this triggers runtime.load_function in mock)
    let payload = serde_json::json!({
        "name": "invoke-func",
        "language": "rust",
        "executable": path_str,
        "cpu": "0.1",
        "memory": "128"
    });
    let req = test::TestRequest::post()
        .uri("/functions")
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // 2. Create Trigger to register route
    let trig_payload = serde_json::json!({
        "name": "invoke-trig",
        "function": "invoke-func",
        "method": "POST",
        "path": "/my-func"
    });
    let req = test::TestRequest::post()
        .uri("/triggers")
        .set_json(&trig_payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // 3. Invoke via Gateway
    let req = test::TestRequest::post()
        .uri("/function/my-func")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body_bytes = test::read_body(resp).await;
    let resp: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp["message"], "Hello from invoke-func");
}

#[actix_rt::test]
async fn test_telemetry() {
    let (app, _td) = spawn_app().await;

    // Test endpoint reachability (Might return 500 if ClickHouse is down, but shouldn't be 404)
    let req = test::TestRequest::get()
        .uri("/telemetry/functions/any-func/metrics")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_ne!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);

    let req = test::TestRequest::get().uri("/telemetry/logs").to_request();
    let resp = test::call_service(&app, req).await;
    assert_ne!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
}
