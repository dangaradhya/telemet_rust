use axum::{
    routing::post,
    Router,
    Json,
};
use serde::Deserialize;
use tokio::net::TcpListener;

// 1. The Data Contract
// Using Deserialize here. The Agent "Serializes" (turns struct -> JSON). 
// The Server "Deserializes" (turns incoming JSON -> struct).
#[derive(Deserialize, Debug)]
struct LogPayload {
    timestamp: String,
    level: String,
    message: String,
}

#[tokio::main]
async fn main() {
    // 2. The Router
    // We map the URL "/api/logs" to a specific function called `ingest_log`.
    // We strictly define this as a POST route. If someone sends a GET request, Axum rejects it automatically.
    let app = Router::new().route("/api/logs", post(ingest_log));

    // 3. The TCP Listener
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    
    println!("--- TelemetRust Aggregator Booting ---");
    println!("Listening for incoming telemetry on http://127.0.0.1:8080/api/logs");

    // 4. Start the Server
    axum::serve(listener, app).await.unwrap();
}

// 5. The Handler Function
// By defining the parameter as `Json<LogPayload>`,
// Axum will automatically read the HTTP body, check if it's valid JSON, 
// ensure it matches our exact struct fields, and pass it to this function.
// If the agent sends malformed data, Axum automatically returns a 400 Bad Request to the agent.
async fn ingest_log(Json(payload): Json<LogPayload>) {
    println!("RECEIVED -> [{}] {}: {}", payload.timestamp, payload.level, payload.message);
}