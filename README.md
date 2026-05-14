# TelemetRust: AI-Powered Telemetry Command Center

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![React](https://img.shields.io/badge/react-%2320232a.svg?style=for-the-badge&logo=react&logoColor=%2361DAFB)
![SQLite](https://img.shields.io/badge/sqlite-%2307405e.svg?style=for-the-badge&logo=sqlite&logoColor=white)
![Gemini AI](https://img.shields.io/badge/Gemini%20AI-%238E75B2.svg?style=for-the-badge&logo=googlebard&logoColor=white)

A distributed, full-stack telemetry aggregation system designed to simulate embedded hardware monitoring. This project ingests real-time system logs, persists them to a local SQLite database, and utilizes an asynchronous background worker to perform automated root-cause analysis on critical faults using the Google Gemini AI API.

## 🏗️ System Architecture

The project is built as a monorepo consisting of three distinct microservices:

1. **The Hardware Agent (`/agent`)**: A Rust-based simulation engine that mimics an embedded system (e.g., a vehicle control unit) transmitting high-frequency system logs and error codes via HTTP POST requests.
2. **The Aggregator Server (`/server`)**: A high-performance, asynchronous Rust backend utilizing `Axum` and `Tokio`. It ingests logs, manages connection pools to an SQLite database, and spawns non-blocking background threads to query the Gemini LLM for automated triage of `[ERROR]` level events.
3. **The Command Center UI (`/dashboard`)**: A modern, dark-mode React application built with Vite. It polls the server's REST API to provide a live, color-coded feed of system events and displays the AI's diagnostic reports in real-time.

## ✨ Core Features

* **Asynchronous AI Triage:** Critical system errors trigger a fire-and-forget background thread that queries the Gemini 3.1 Flash Lite model for rapid root-cause analysis and recommended fixes without blocking the main telemetry ingestion pipeline.
* **Robust Error Handling:** The backend is designed to gracefully handle upstream API rate limits, network timeouts, and JSON parsing failures, ensuring the core database remains uncorrupted during cloud outages.
* **Relational Persistence:** Utilizes `sqlx` for compile-time verified SQL queries, maintaining a relational link between raw hardware logs and generated AI reports.
* **Live-Updating UI:** The React dashboard implements efficient API polling to instantly visualize incoming hardware faults and their corresponding AI diagnosis.

## 🚀 Getting Started

### Prerequisites
* [Rust & Cargo](https://rustup.rs/)
* [Node.js & npm](https://nodejs.org/)
* [Google AI Studio API Key](https://aistudio.google.com/)

### Installation & Setup

**1. Clone the repository**
```bash
git clone https://github.com/yourusername/telemet_rust.git
cd telemet_rust
```

**2. Configure Environment Variables**
Create a `.env` file in the root directory and add your Gemini API key:
```env
GEMINI_API_KEY=your_api_key_here
```

**3. Install Frontend Dependencies**
```bash
cd dashboard
npm install
cd ..
```

### Running the System

The project includes an orchestration script that handles process lifecycle management. You can boot the entire infrastructure (Agent, Server, and Dashboard) with a single command:

```bash
# Make the script executable (first time only)
chmod +x start.sh

# Boot the system
./start.sh
```

Once running, navigate to `http://localhost:5173` in your browser to view the live Command Center. To gracefully shut down all microservices, simply press `Ctrl+C` in the terminal. There is also a dashboard that displays the raw JSON data: `http://127.0.0.1:8080/api/dashboard`

## 🛠️ Tech Stack Details

* **Backend Environment:** Rust, Cargo Workspaces
* **Web Framework:** Axum, Tower (CORS)
* **Async Runtime:** Tokio
* **Database:** SQLite, SQLx (Compile-time checked queries)
* **HTTP Client:** Reqwest (for external LLM API calls)
* **Frontend:** React 18, Vite, standard CSS
* **AI Integration:** Google Gemini API (`gemini-3.1-flash-lite`)