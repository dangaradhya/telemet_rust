use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::{sleep, Duration};

#[derive(serde::Serialize)]
struct LogPayload {
    timestamp: String,
    level: String,
    message: String,
}

#[tokio::main]
async fn main() {
    let log_path = "system.log";
    // Point to our new Axum API endpoint
    let api_endpoint = "http://127.0.0.1:8080/api/logs"; 
    
    println!("--- TelemetRust Live Agent Booting ---");
    
    // Create an HTTP Client once, outside the loop.
    // Creating a new client inside the loop is a major performance anti-pattern.
    let http_client = reqwest::Client::new();

    let file = File::open(log_path).await.expect("CRITICAL: Failed to open system.log");
    let mut reader = BufReader::new(file);
    let mut line_buffer = String::new();

    loop {
        line_buffer.clear();
        let bytes_read = reader.read_line(&mut line_buffer).await.expect("IO Error");

        if bytes_read == 0 {
            sleep(Duration::from_millis(100)).await;
            continue;
        }

        let clean_line = line_buffer.trim(); 
        let (level, message) = if clean_line.starts_with("[ERROR]") {
            ("ERROR", &clean_line[7..])
        } else if clean_line.starts_with("[WARN]") {
            ("WARN", &clean_line[6..])
        } else if clean_line.starts_with("[INFO]") {
            ("INFO", &clean_line[6..])
        } else {
            ("UNKNOWN", clean_line)
        };

        let payload = LogPayload {
            timestamp: chrono::Utc::now().to_rfc3339(), 
            level: level.to_string(),
            message: message.trim().to_string(),
        };

        // Network Transmission using Reqwest
        match http_client.post(api_endpoint)
            .json(&payload) // Automatically serializes the struct to JSON and sets HTTP headers
            .send()
            .await 
        {
            Ok(response) => {
                if response.status().is_success() {
                    println!("Transmitted (HTTP 200 OK): {}", payload.message);
                } else {
                    println!("SERVER REJECTED PAYLOAD: Status {}", response.status());
                }
            }
            Err(e) => println!("NETWORK FAULT: Could not reach Aggregator. Error: {}", e),
        }
    }
}