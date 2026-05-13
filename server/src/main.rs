use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tokio::net::TcpListener;
use std::fs::File;
use std::path::Path;

#[derive(Deserialize, Debug)]
struct LogPayload {
    timestamp: String,
    level: String,
    message: String,
}

#[tokio::main]
async fn main() {
    println!("--- TelemetRust Aggregator Booting ---");

    // 1. Database Initialization
    let db_path = "telemetry.db";
    let db_url = format!("sqlite://{}", db_path);

    // If the database file doesn't exist, create an empty one
    if !Path::new(db_path).exists() {
        File::create(db_path).expect("Failed to create database file");
        println!("Database file created: {}", db_path);
    }

    // 2. The Connection Pool
    // A "Pool" holds multiple open connections to the database. 
    // This allows multiple HTTP requests to write to the DB simultaneously.
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to SQLite");

    // 3. Automated Migrations (Schema Definition)
    // Run a query on startup to ensure our table exists
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            level TEXT NOT NULL,
            message TEXT NOT NULL
        );"
    )
    .execute(&pool)
    .await
    .expect("Failed to create database table");

    println!("Database connected and schema verified.");

    // 4. The Router with State
    // `.with_state(pool)` safely injects our database pool into the Axum web framework
    let app = Router::new()
        .route("/api/logs", post(ingest_log))
        .with_state(pool);

    // 5. Start the Server
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("Listening for incoming telemetry on http://127.0.0.1:8080/api/logs\n");
    axum::serve(listener, app).await.unwrap();
}

// 6. The Handler Function (Now with Database Access)
// Axum automatically extracts the 'State' (our db pool) and the 'Json' payload.
async fn ingest_log(
    State(pool): State<SqlitePool>,
    Json(payload): Json<LogPayload>,
) {
    // Execute an asynchronous INSERT statement
    // We use `?` parameter binding to prevent SQL Injection attacks
    let result = sqlx::query(
        "INSERT INTO logs (timestamp, level, message) VALUES (?, ?, ?)"
    )
    .bind(&payload.timestamp)
    .bind(&payload.level)
    .bind(&payload.message)
    .execute(&pool)
    .await;

    match result {
        Ok(_) => println!("DB SAVED -> [{}] {}: {}", payload.timestamp, payload.level, payload.message),
        Err(e) => println!("CRITICAL DB ERROR: Failed to save log - {}", e),
    }
}