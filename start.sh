#!/bin/bash

echo "🚀 Booting TelemetRust Infrastructure..."

# 1. Start the React Dashboard
# Using --prefix tells npm to run the command inside the dashboard folder without us having to cd into it
echo "📡 Starting React Dashboard on port 5173..."
npm run dev --prefix dashboard > /dev/null 2>&1 &
FRONTEND_PID=$!

# 2. Start the Rust Aggregator Server
# We are already in the root workspace, so cargo -p knows exactly what to do
echo "🧠 Starting Axum Aggregator Server on port 8080..."
cargo run -p server > /dev/null 2>&1 &
SERVER_PID=$!

# Wait for the backend to initialize and bind to the port
sleep 3

# 3. Start the Hardware Agent Simulation
echo "⚙️ Starting Hardware Agent Simulation..."
cargo run -p agent &
AGENT_PID=$!

# The Teardown Trap: Catches Ctrl+C and gracefully kills all background processes
trap "echo -e '\n🛑 Shutting down TelemetRust...'; kill $FRONTEND_PID $SERVER_PID $AGENT_PID 2>/dev/null; exit" INT TERM EXIT

echo "✅ All systems running! Press Ctrl+C to safely shut down."

# Keep the script alive while the background processes run
wait