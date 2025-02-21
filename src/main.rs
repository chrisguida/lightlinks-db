use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use sqlx::{Pool, Sqlite, SqlitePool};
use std::sync::Arc;
use std::env;
use std::fs;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct Store {
    db: Arc<Pool<Sqlite>>,
}

#[tokio::main]
async fn main() {
    // Ensure the database file exists
    let db_path = "store.db";
    if !std::path::Path::new(db_path).exists() {
        fs::File::create(db_path).unwrap();
    }

    // Ensure the directory for the database file exists
    let db_dir = std::path::Path::new(db_path).parent().unwrap_or(std::path::Path::new("."));
    fs::create_dir_all(db_dir).unwrap();

    // Set the DATABASE_URL environment variable in Rust
    env::set_var("DATABASE_URL", format!("sqlite://{}", db_path));

    let database_url = env::var("DATABASE_URL").unwrap();
    let pool = SqlitePool::connect(&database_url).await.unwrap();
    sqlx::query("CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT)")
        .execute(&pool)
        .await
        .unwrap();

    let store = Store {
        db: Arc::new(pool),
    };

    let app = Router::new()
        .route("/store/:key", get(get_value))
        .route("/store", post(set_value))
        .layer(CorsLayer::new().allow_origin(Any))
        .with_state(store);

    let addr: std::net::SocketAddr = "0.0.0.0:3000".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_value(State(store): State<Store>, Path(key): Path<String>) -> Json<Option<String>> {
    let result = sqlx::query_scalar("SELECT value FROM kv WHERE key = ?")
        .bind(key)
        .fetch_optional(&*store.db)
        .await
        .unwrap();
    Json(result)
}

#[derive(Deserialize)]
struct SetValueRequest {
    key: String,
    value: String,
}

async fn set_value(State(store): State<Store>, Json(payload): Json<SetValueRequest>) -> StatusCode {
    sqlx::query("INSERT INTO kv (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(payload.key)
        .bind(payload.value)
        .execute(&*store.db)
        .await
        .unwrap();
    StatusCode::OK
}
