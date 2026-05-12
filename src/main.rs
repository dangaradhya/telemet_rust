// We are replacing 'std::fs' with 'tokio::fs' for non-blocking file access.
use tokio::fs::File;
// We need AsyncBufReadExt to read files line-by-line asynchronously.
use tokio::io::{AsyncBufReadExt, BufReader};

// PRINCIPLE 1: The Tokio Macro
// A standard Rust program must start with a synchronous 'main' function.
// The #[tokio::main] macro automatically writes a hidden synchronous main function
// that boots up the Tokio runtime, which then executes our async code.
#[tokio::main]
async fn main() {
    let log_path = "system.log";
    println!("--- TelemetRust Async Agent Booting ---");

    // PRINCIPLE 2: Async File Opening
    // File::open returns a 'Future'. 
    // We add '.await' to pause here until the OS actually opens the file.
    // If it fails, '.expect()' crashes the program gracefully.
    let file = File::open(log_path).await.expect("CRITICAL: Failed to open system.log");

    // PRINCIPLE 3: Buffered Reading
    // Reading directly from a file byte-by-byte is incredibly slow.
    // BufReader grabs a large chunk of the file into RAM all at once.
    let reader = BufReader::new(file);

    // .lines() creates an asynchronous stream that yields one line at a time.
    let mut lines = reader.lines();

    // PRINCIPLE 4: The Async Loop
    // Because we are fetching data asynchronously, we can't use a standard 'for' loop.
    // We use a 'while let' loop. It asks: "Wait for the next line. If it exists, bind it to 'line'."
    while let Some(line) = lines.next_line().await.expect("Error reading line") {
        
        if line.contains("[ERROR]") {
            // In a real system, this is where we would asynchronously fire a network packet
            // to our central server without stopping the file reading process.
            println!("URGENT ALARM (Async): {}", line);
        } else {
            println!("Processed (Async): {}", line);
        }
    }

    println!("--- End of File Reached ---");
}