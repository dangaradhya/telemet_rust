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

// Used to parse the JSON response from the LLM
#[derive(Deserialize, Debug)]
struct OpenAiResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: OpenAiMessage,
}

#[derive(Deserialize, Debug)]
struct OpenAiMessage {
    content: String,
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
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            level TEXT NOT NULL,
            message TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS triage_reports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            log_id INTEGER NOT NULL,
            analysis TEXT NOT NULL,
            FOREIGN KEY(log_id) REFERENCES logs(id)
        );"
    )
    .execute(&pool)
    .await
    .expect("Failed to create database tables");

    println!("Database tables connected and schema verified.");

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

// The HTTP Handler for Ingesting Logs
async fn ingest_log(
    State(pool): State<SqlitePool>,
    Json(payload): Json<LogPayload>,
) {
    // 1. Save the incoming log to the database
    let result = sqlx::query(
        "INSERT INTO logs (timestamp, level, message) VALUES (?, ?, ?)"
    )
    .bind(&payload.timestamp)
    .bind(&payload.level)
    .bind(&payload.message)
    .execute(&pool)
    .await;

    // 2. Log the result to the console for visibility
    match result {
        Ok(db_result) => {
            println!("DB SAVED -> [{}] {}: {}", payload.timestamp, payload.level, payload.message);
            
            // THE FORCE MULTIPLIER: Trigger the AI Triage in the background
            // Only trigger the AI for ERROR logs to save resources. 
            if payload.level == "ERROR" {
                // We need the log_id to link the AI's analysis back to the original log entry in the database
                let log_id = db_result.last_insert_rowid();
                // Cloning the pool and message for the async task. This is necessary because the async task will outlive the scope of this HTTP request handler.
                let pool_clone = pool.clone(); 
                // Cloning the message string to move it into the async task. This avoids ownership issues since the original payload will be dropped after this function returns.
                let msg_clone = payload.message.clone();

                // tokio::spawn detaches this task from the main thread. 
                // The server immediately finishes the HTTP request without waiting for the AI!
                tokio::spawn(async move {
                    run_ai_triage(pool_clone, log_id, msg_clone).await;
                });
            }
        }
        Err(e) => println!("CRITICAL DB ERROR: Failed to save log - {}", e),
    }
}

// The Fire-and-Forget AI Worker
async fn run_ai_triage(pool: SqlitePool, log_id: i64, error_message: String) {
    println!(">> [AI Worker Started] Triaging Log ID: {}", log_id);

    // Grab the API key from your Linux environment variables
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        println!(">> [AI Worker Aborted] No OPENAI_API_KEY found in environment.");
        return;
    }

    // Construct the prompt for the LLM. This is where you can get creative with how you want to instruct the AI to analyze the error message.
    let client = reqwest::Client::new();
    let prompt = format!(
        "You are an expert embedded systems engineer. Analyze this system log error and provide a 2-sentence probable root cause and a 1-sentence recommended fix. Error: '{}'",
        error_message
    );

    // The body of the request to OpenAI's Chat Completion API. This is where you specify the model, the messages (prompt), and any parameters like temperature. 
    // This is supposed to be a simple JSON payload that the OpenAI API expects. 
    let body = serde_json::json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.2
    });

    // Make the HTTP POST request to OpenAI's API. This is an asynchronous network call that will wait for the AI's response without blocking the main server thread.
    match client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await 
    {
        Ok(response) => {
            // Parse the JSON response from OpenAI into our OpenAiResponse struct. This allows us to easily extract the AI's analysis from the response.
            // The OpenAI API returns a structured JSON response that includes an array of "choices". Each choice contains a "message" with the AI's content.
            // We take the first choice and extract the content for our analysis.
            if let Ok(json) = response.json::<OpenAiResponse>().await {
                if let Some(choice) = json.choices.first() {
                    let analysis = &choice.message.content;
                    
                    // Save the AI's analysis to the database
                    let _ = sqlx::query("INSERT INTO triage_reports (log_id, analysis) VALUES (?, ?)")
                        .bind(log_id)
                        .bind(analysis)
                        .execute(&pool)
                        .await;

                    println!(">> [AI Triage Complete] Saved analysis for Log ID: {}", log_id);
                }
            }
        }
        Err(e) => println!(">> [AI Worker Failed] Network error: {}", e),
    }
}