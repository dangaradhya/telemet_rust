// We must explicitly import the File System (fs) module from the standard library
use std::fs;

fn main() {
    // PRINCIPLE 1: Immutability by Default
    // In Rust, variables cannot be changed once assigned unless you use 'mut'.
    // We are binding the string literal to 'log_path'. 
    let log_path = "system.log";

    // PRINCIPLE 2: Error Handling without Exceptions
    // Rust doesn't use try/catch blocks. Functions that can fail return a 'Result' type.
    // .expect() says: "If this Result is an error, crash the program with this message. 
    // If it's successful, give me the file contents."
    let file_contents = fs::read_to_string(log_path)
        .expect("CRITICAL: Failed to read the log file. Check the path.");

    println!("--- TelemetRust Agent Initialized ---");

    // PRINCIPLE 3: Borrowing and Iteration
    // file_contents.lines() creates an iterator over the string.
    for line in file_contents.lines() {
        // We use the '&' symbol here. This is a "Reference". 
        // It means we are "borrowing" the line to look at it, but we don't "own" it.
        // This is how Rust achieves memory safety without a garbage collector.
        if line.contains("[ERROR]") {
            // The ! in println! means it's a "Macro" (code that writes other code), not a function.
            println!("URGENT ALARM: {}", line);
        } else {
            println!("Processed: {}", line);
        }
    }
}