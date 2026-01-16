use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};

use crate::domain::entities::{DomainError, Function, Language, Trigger, User};
use crate::domain::ports::{FunctionRepository, TriggerRepository, UserRepository};
use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHasher, SaltString},
};
use rand_core::OsRng;
use std::env;
use std::str::FromStr;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Clone)]
pub struct SqliteRepository {
    pool: SqlitePool,
}

impl SqliteRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

pub async fn create_pool(mut database_url: String) -> SqlitePool {
    // Sanitize URL from legacy parameters that cause panic
    if database_url.contains("_journal_mode") || database_url.contains("_busy_timeout") {
        warn!("Sanitizing invalid DATABASE_URL parameters...");
        if let Some(idx) = database_url.find('?') {
            database_url = database_url[..idx].to_string();
        }
        database_url.push_str("?mode=rwc");
    }

    let connection_options = SqliteConnectOptions::from_str(&database_url)
        .expect("Failed to parse database URL")
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_millis(5000))
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(connection_options)
        .await
        .expect("Failed to connect to DB");

    // Create Schema
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL DEFAULT '',
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user'
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create users table");

    info!("Users table initialized");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS functions (
            name TEXT PRIMARY KEY,
            language TEXT NOT NULL,
            executable TEXT NOT NULL,
            cpu TEXT NOT NULL,

            memory TEXT NOT NULL,
            readonly BOOLEAN NOT NULL DEFAULT FALSE
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create functions table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS triggers (
            name TEXT PRIMARY KEY,
            method TEXT NOT NULL,
            path TEXT NOT NULL,
            function TEXT NOT NULL,
            function_name TEXT, 
            readonly BOOLEAN NOT NULL DEFAULT FALSE,
            FOREIGN KEY (function) REFERENCES functions(name)
        )", // Note: I noticed `function` column in triggers table in previous view, but Trigger struct has function_name.
            // In seed data: `function TEXT NOT NULL`.
            // I will keep it as is, but ensuring schema matches what was there.
            // Wait, checking original file line 83: `function TEXT NOT NULL`.
            // Line 246 in original struct From impl maps `row.function` to `Trigger.function_name`.
            // So database column is `function`.
    )
    .execute(&pool)
    .await
    .expect("Failed to create triggers table");

    // Attempt to add 'readonly' column if it doesn't exist (for existing DBs)
    let _ = sqlx::query("ALTER TABLE functions ADD COLUMN readonly BOOLEAN NOT NULL DEFAULT FALSE")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE triggers ADD COLUMN readonly BOOLEAN NOT NULL DEFAULT FALSE")
        .execute(&pool)
        .await;

    pool
}

pub async fn init_db() -> SqlitePool {
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:fluor.db?mode=rwc".to_string());
    let pool = create_pool(database_url).await;

    seed_data(&pool).await;

    let admin_email = env::var("ADMIN_EMAIL").expect("ADMIN_EMAIL must be set");
    let admin_password = env::var("ADMIN_PASSWORD").expect("ADMIN_PASSWORD must be set");
    let pepper = env::var("PASSWORD_PEPPER").expect("PASSWORD_PEPPER must be set");
    seed_admin(&pool, &admin_email, &admin_password, &pepper).await;

    pool
}

pub async fn seed_data(pool: &SqlitePool) {

    // Ensure healthz always exists
    let healthz_exists: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM functions WHERE name = 'healthz'")
            .fetch_one(pool)
            .await
            .unwrap_or((0,));

    if healthz_exists.0 == 0 {
        sqlx::query("INSERT INTO functions (name, language, executable, cpu, memory, readonly) VALUES 
            ('healthz', 'rust', 'modules/healthz/target/wasm32-wasip1/release/healthz.wasm', '0.1', '128', TRUE)
        ")
        .execute(pool)
        .await
        .expect("Failed to seed healthz function"); // Ignore if fails (e.g. concurrent)

        sqlx::query(
            "INSERT INTO triggers (name, method, path, function, readonly) VALUES 
            ('health-check', 'GET', '/healthz', 'healthz', TRUE)
        ",
        )
        .execute(pool)
        .await
        .expect("Failed to seed healthz trigger");

        info!("Seeded healthz function");
    }
}

pub async fn seed_admin(pool: &SqlitePool, admin_email: &str, admin_password: &str, pepper: &str) {
    // Env vars removed from here

    let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email = ?")
        .bind(admin_email)
        .fetch_one(pool)
        .await
        .expect("Failed to check for existing admin user");

    if exists.0 == 0 {
        let salt = SaltString::generate(&mut OsRng);
        let password_with_pepper = format!("{}{}", admin_password, pepper);

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default());

        let password_hash = argon2
            .hash_password(password_with_pepper.as_bytes(), &salt)
            .unwrap()
            .to_string();

        sqlx::query("INSERT INTO users (email, password_hash) VALUES (?, ?)")
            .bind(admin_email)
            .bind(password_hash)
            .execute(pool)
            .await
            .expect("Failed to seed admin user");

        info!("Seeded admin user");
    }
}

// DTOs
#[derive(sqlx::FromRow)]
struct UserRow {
    id: i64,
    name: String,
    email: String,
    password_hash: String,
    role: String,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        User {
            id: row.id,
            name: row.name,
            email: row.email,
            password_hash: row.password_hash,
            role: row.role,
        }
    }
}

#[derive(sqlx::FromRow)]
struct FunctionRow {
    name: String,
    language: String,
    executable: String,
    cpu: String,
    memory: String,
    readonly: bool,
}

impl From<FunctionRow> for Function {
    fn from(row: FunctionRow) -> Self {
        let lang = match row.language.as_str() {
            "python" | "Python" => Language::Python,
            "rust" | "Rust" => Language::Rust,
            "go" | "Go" => Language::Go,
            _ => Language::Python, // Default or error
        };
        Function {
            name: row.name,
            language: lang,
            executable: row.executable,
            cpu: row.cpu,
            memory: row.memory,
            runtime: None,
            readonly: row.readonly,
        }
    }
}

#[derive(sqlx::FromRow)]
struct TriggerRow {
    name: String,
    method: String,
    path: String,
    function: String,
    readonly: bool,
}

impl From<TriggerRow> for Trigger {
    fn from(row: TriggerRow) -> Self {
        Trigger {
            name: row.name,
            method: row.method,
            path: row.path,
            function_name: row.function,
            readonly: row.readonly,
        }
    }
}

#[async_trait]
impl UserRepository for SqliteRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError> {
        let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE email = ?")
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(Into::into))
    }

    async fn save(&self, user: &User) -> Result<User, DomainError> {
        // ID is autoincrement, logic would be fetch after insert or assume logic
        // Simplified for this task
        Ok(user.clone())
    }

    async fn update(&self, user: &User) -> Result<User, DomainError> {
        let result = sqlx::query("UPDATE users SET name = ?, email = ?, password_hash = ? WHERE id = ?")
            .bind(&user.name)
            .bind(&user.email)
            .bind(&user.password_hash)
            .bind(user.id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    DomainError::AlreadyExists(user.email.clone())
                } else {
                    DomainError::Internal(e.to_string())
                }
            })?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("User with id {}", user.id)));
        }

        Ok(user.clone())
    }
}

#[async_trait]
impl FunctionRepository for SqliteRepository {
    async fn find_all(&self) -> Result<Vec<Function>, DomainError> {
        let rows = sqlx::query_as::<_, FunctionRow>("SELECT * FROM functions")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Function>, DomainError> {
        let row = sqlx::query_as::<_, FunctionRow>("SELECT * FROM functions WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(Into::into))
    }

    async fn save(&self, f: &Function) -> Result<Function, DomainError> {
        let lang_str = format!("{:?}", f.language); // Debug format is usually Capitalized
        sqlx::query("INSERT INTO functions (name, language, executable, cpu, memory, readonly) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&f.name)
            .bind(lang_str)
            .bind(&f.executable)
            .bind(&f.cpu)
            .bind(&f.memory)
            .bind(f.readonly)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    DomainError::AlreadyExists(f.name.clone())
                } else {
                    DomainError::Internal(e.to_string())
                }
            })?;
        Ok(f.clone())
    }

    async fn update(&self, f: &Function) -> Result<Function, DomainError> {
        // Check if readonly
        let current = self.find_by_name(&f.name).await?;
        if let Some(current) = current {
            if current.readonly {
                 return Err(DomainError::ValidationError(format!("Function '{}' is readonly", f.name)));
            }
        } else {
             return Err(DomainError::NotFound(f.name.clone()));
        }

        let lang_str = format!("{:?}", f.language);
        let result = sqlx::query(
            "UPDATE functions SET language=?, executable=?, cpu=?, memory=? WHERE name=?",
        )
        .bind(lang_str)
        .bind(&f.executable)
        .bind(&f.cpu)
        .bind(&f.memory)
        .bind(&f.name)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(f.name.clone()));
        }

        Ok(f.clone())
    }

    async fn delete(&self, name: &str) -> Result<(), DomainError> {
        // Check if readonly
        let current = self.find_by_name(name).await?;
         if let Some(current) = current {
            if current.readonly {
                 return Err(DomainError::ValidationError(format!("Function '{}' is readonly", name)));
            }
        } else {
            return Err(DomainError::NotFound(name.to_string()));
        }

        let result = sqlx::query("DELETE FROM functions WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(name.to_string()));
        }
        Ok(())
    }
}

#[async_trait]
impl TriggerRepository for SqliteRepository {
    async fn find_all(&self) -> Result<Vec<Trigger>, DomainError> {
        let rows = sqlx::query_as::<_, TriggerRow>("SELECT * FROM triggers")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn save(&self, t: &Trigger) -> Result<Trigger, DomainError> {
        sqlx::query("INSERT INTO triggers (name, method, path, function, readonly) VALUES (?, ?, ?, ?, ?)")
            .bind(&t.name)
            .bind(&t.method)
            .bind(&t.path)
            .bind(&t.function_name)
            .bind(t.readonly)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    DomainError::AlreadyExists(t.name.clone())
                } else {
                    DomainError::Internal(e.to_string())
                }
            })?;
        Ok(t.clone())
    }

    async fn delete(&self, name: &str) -> Result<(), DomainError> {
        // Check if readonly
        // We need find_by_name for trigger but we haven't implemented it in repo trait yet?
        // Wait, TriggerRepository trait usually has find_by_name. Let me check ports.rs or infer from context.
        // Actually, looking at sqlite.rs, find_all is there, delete is there, but save is there.
        // I need to check if find_by_name exists in TriggerRepository.
        // Based on previous file content, it wasn't visible in the view.
        // I will assume I need to fetch it manually here if not available in trait.
        // But let's check ports.rs first? No I can just check sqlite.rs again.
        // TriggerRepository impl usually follows FunctionRepository.
        // sqlite.rs impl has find_all, save, delete. It MISSES find_by_name.
        // I should stick to `sqlx::query_as` locally here.
        
        let row = sqlx::query_as::<_, TriggerRow>("SELECT * FROM triggers WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        if let Some(row) = row {
            if row.readonly {
                return Err(DomainError::ValidationError(format!("Trigger '{}' is readonly", name)));
            }
        } else {
             return Err(DomainError::NotFound(name.to_string()));
        }

        let result = sqlx::query("DELETE FROM triggers WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(name.to_string()));
        }
        Ok(())
    }
}


