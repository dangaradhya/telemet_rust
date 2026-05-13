use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::{sleep, Duration};

// 1. Define our strongly-typed Data Structure
#[derive(serde::Serialize)]
struct LogPayload {
    timestamp: String,
    level: String,
    message: String,
}

#[tokio::main]
async fn main() {
    let log_path = "system.log";
    println!("--- TelemetRust Live Agent Booting ---");
    println!("Watching {} for real-time events...", log_path);

    let file = File::open(log_path).await.expect("CRITICAL: Failed to open system.log");
    let mut reader = BufReader::new(file);
    
    // We allocate a single String buffer in memory once. 
    // We will clear and reuse this exact memory address for every line, 
    // which prevents the garbage-collection lag found in languages like Python.
    let mut line_buffer = String::new();

    // 2. The Infinite Agent Loop
    loop {
        line_buffer.clear(); // Empty the buffer without freeing the memory capacity

        // Read bytes directly into our buffer
        let bytes_read = reader.read_line(&mut line_buffer).await.expect("IO Error");

        if bytes_read == 0 {
            // EOF (End of File) Reached. 
            // Yield the CPU for 100ms before checking for new data.
            // This ensures our background agent uses ~0.01% CPU while waiting.
            sleep(Duration::from_millis(100)).await;
            continue; // Skip the rest of the loop and start over
        }

        // 3. Parsing and Structuring the Data
        // Remove the trailing newline character (\n)
        let clean_line = line_buffer.trim(); 

        // Basic parser: Check if the line starts with a severity bracket
        let (level, message) = if clean_line.starts_with("[ERROR]") {
            ("ERROR", &clean_line[7..])
        } else if clean_line.starts_with("[WARN]") {
            ("WARN", &clean_line[6..])
        } else if clean_line.starts_with("[INFO]") {
            ("INFO", &clean_line[6..])
        } else {
            ("UNKNOWN", clean_line)
        };

        // 4. Instantiate our Struct
        let payload = LogPayload {
            // Generate an exact ISO 8601 timestamp at the moment of reading
            timestamp: chrono::Utc::now().to_rfc3339(),
            level: level.to_string(),
            message: message.trim().to_string(),
        };

        // 5. Serialize to JSON
        let json_output = serde_json::to_string(&payload).expect("Failed to serialize");

        println!("Transmitting: {}", json_output);
    }
}