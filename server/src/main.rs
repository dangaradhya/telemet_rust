use axum::{
    extract::State,
    routing::{get, post},
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

// Used to parse the JSON response from the Gemini API
#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: GeminiContent,
}

#[derive(Deserialize, Debug)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Deserialize, Debug)]
struct GeminiPart {
    text: String,
}

// For the Frontend UI Dashboard
#[derive(serde::Serialize, sqlx::FromRow)]
struct LogResponse {
    id: i64,
    timestamp: String,
    level: String,
    message: String,
    analysis: Option<String>, // Optional because not all logs will have an AI analysis (only ERROR logs do)
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

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
    // Create the logs table first
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
    .expect("Failed to create logs table");

    // Create the triage_reports table SECOND
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS triage_reports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            log_id INTEGER NOT NULL,
            analysis TEXT NOT NULL,
            FOREIGN KEY(log_id) REFERENCES logs(id)
        );"
    )
    .execute(&pool)
    .await
    .expect("Failed to create triage_reports table");

    println!("Database tables connected and schema verified.");

    // 4. The Router with State along with the GET route for the Dashboard API
    let app = Router::new()
        .route("/api/logs", post(ingest_log))
        .route("/api/dashboard", get(fetch_logs)) 
        .with_state(pool);

    // 5. Start the Server
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("Listening for incoming telemetry on http://127.0.0.1:8080/api/logs");
    println!("Dashboard API available at http://127.0.0.1:8080/api/dashboard\n");
    axum::serve(listener, app).await.unwrap();
}

// --- API ENDPOINTS ---

// 6. The POST Endpoint (Used by the Agent to send data)
async fn ingest_log(
    State(pool): State<SqlitePool>,
    Json(payload): Json<LogPayload>,
) {
    // Save the incoming log to the database
    let result = sqlx::query(
        "INSERT INTO logs (timestamp, level, message) VALUES (?, ?, ?)"
    )
    .bind(&payload.timestamp)
    .bind(&payload.level)
    .bind(&payload.message)
    .execute(&pool)
    .await;

    // Log the result to the console for visibility
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

// 7. The GET Endpoint (Used by the UI Dashboard to read data)
// The function takes the shared database pool as input and returns a JSON response containing a vector of LogResponse structs.
async fn fetch_logs(State(pool): State<SqlitePool>) -> axum::Json<Vec<LogResponse>> {
    // This SQL query joins the logs and triage_reports tables to fetch the latest 50 logs along with any AI analysis if it exists.
    let records = sqlx::query_as::<_, LogResponse>(
        "SELECT logs.id, logs.timestamp, logs.level, logs.message, triage_reports.analysis 
         FROM logs 
         LEFT JOIN triage_reports ON logs.id = triage_reports.log_id 
         ORDER BY logs.id DESC 
         LIMIT 50"
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    // Return the records as JSON to the frontend dashboard.
    // The frontend can then display this data in a table, showing the original log message and the AI's analysis side by side.
    axum::Json(records)
}

// --- BACKGROUND WORKERS ---

// 8. The Fire-and-Forget AI Worker 
async fn run_ai_triage(pool: SqlitePool, log_id: i64, error_message: String) {
    println!(">> [AI Worker Started] Triaging Log ID: {}", log_id);

    // Grab the API key from your Linux environment variables
    let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        println!(">> [AI Worker Aborted] No GEMINI_API_KEY found in environment.");
        return;
    }

    // Construct the prompt for the Gemini API. This is where you can get creative with how you ask the AI to analyze the error message.
    let client = reqwest::Client::new();
    let prompt = format!(
        "You are an expert embedded systems engineer. Analyze this system log error and provide a 2-sentence probable root cause and a 1-sentence recommended fix. Error: '{}'",
        error_message
    );

    // The Gemini API expects a specific JSON structure for the request body. 
    let body = serde_json::json!({
        "contents": [{
            "parts": [{"text": prompt}]
        }]
    });

    // Make the POST request to the Gemini API. This is an asynchronous network call that will not block the main server thread.
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.1-flash-lite:generateContent?key={}",
        api_key
    );

    // Handle the response from the Gemini API. We need to parse the JSON response to extract the AI's analysis.
    match client.post(&url).json(&body).send().await {
        Ok(response) => {
            let status = response.status();
            
            // Grab the raw text response from Google first
            if let Ok(raw_text) = response.text().await {
                
                // Check if Google accepted the request (HTTP 200 OK)
                if status.is_success() {
                    // Try to parse the successful text into our Rust struct
                    // If parsing fails, we log the error and the raw response for debugging. 
                    match serde_json::from_str::<GeminiResponse>(&raw_text) {
                        Ok(json) => {
                            // Extract the AI's analysis from the parsed JSON. The structure of the response is based on the Gemini API's documentation. 
                            // We navigate through the nested fields to get to the actual text content of the AI's response. 
                            if let Some(candidate) = json.candidates.first() {
                                if let Some(part) = candidate.content.parts.first() {
                                    let analysis = &part.text;
                                    
                                    // Save the AI's analysis to the database and actually check for errors!
                                    let db_result = sqlx::query("INSERT INTO triage_reports (log_id, analysis) VALUES (?, ?)")
                                        .bind(log_id)
                                        .bind(analysis)
                                        .execute(&pool)
                                        .await;
                                        
                                    match db_result {
                                        Ok(_) => println!(">> [AI Triage Complete] Saved analysis for Log ID: {}", log_id),
                                        Err(e) => println!(">> [CRITICAL DB ERROR] AI succeeded, but DB save failed: {}", e),
                                    }
                                }
                            }
                        }
                        Err(e) => println!(">> [AI Worker Parse Error] Failed to parse: {}. Raw: {}", e, raw_text),
                    }
                } else {
                    // If Google sent an error, print the EXACT reason
                    println!(">> [AI Worker API Error] HTTP {}: {}", status, raw_text);
                }
            }
        }
        Err(e) => println!(">> [AI Worker Failed] Network error: {}", e),
    }
}
