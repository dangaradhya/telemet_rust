use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader, AsyncWriteExt}; 
use tokio::net::TcpStream; 
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
    // 1. Define our Central Server Address (Localhost for testing, port 8080)
    let server_address = "127.0.0.1:8080"; 
    
    println!("--- TelemetRust Live Agent Booting ---");
    println!("Attempting to connect to Central Aggregator at {}...", server_address);

    // 2. The Network Handshake (Resilient Design)
    // We try to connect to the server. 'match' handles the Success (Ok) or Failure (Err) gracefully.
    let mut network_stream = match TcpStream::connect(server_address).await {
        Ok(stream) => {
            println!("SUCCESS: TCP Socket established.");
            Some(stream) // Store the active stream
        }
        Err(e) => {
            // If the server is offline, we DO NOT crash. We log a warning and continue.
            println!("WARN: Aggregator offline ({}). Running in Local-Only mode.", e);
            None // No stream available
        }
    };

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

        let json_output = serde_json::to_string(&payload).expect("Failed to serialize");

        // 3. The Transmission Logic
        // We append a newline character (\n) so the receiving server knows where one JSON object ends and the next begins.
        let network_payload = format!("{}\n", json_output);

        // 4. Routing the Data
        // 'if let Some' checks: Do we have an active network connection?
        if let Some(ref mut stream) = network_stream {
            
            // If yes, convert the string to raw bytes and blast it over the TCP socket.
            match stream.write_all(network_payload.as_bytes()).await {
                Ok(_) => println!("Transmitted (TCP): {}", json_output),
                Err(e) => {
                    println!("NETWORK FAULT: Connection lost. Error: {}", e);
                    // In a production system, we would trigger a reconnect sequence here.
                }
            }
        } else {
            // If the network is down, we fall back to printing it locally.
            println!("Local Store (Offline): {}", json_output);
        }
    }
}