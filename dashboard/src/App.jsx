// 1. IMPORTS
// 'useState' lets us store data (like memory). 
// 'useEffect' lets us run background tasks (like fetching data).
import { useState, useEffect } from 'react';
import './App.css'; // Pulls in the dark-mode styles we wrote earlier

function App() {
  // 2. STATE (The Component's Memory)
  // The syntax is: const [variableName, setterFunction] = useState(initialValue);
  // Whenever we call a setterFunction (like setLogs), React instantly redraws the screen with the new data.
  const [logs, setLogs] = useState([]); // Starts as an empty array []
  const [loading, setLoading] = useState(true); // Starts as true, so we can show a loading message
  const [connectionError, setConnectionError] = useState(null); // Stores any network errors

  // 3. EFFECTS (Background Tasks)
  // useEffect runs the code inside it as soon as the dashboard loads on the screen.
  useEffect(() => {
    
    // We create an asynchronous function to fetch data from your Rust server without freezing the browser.
    const fetchTelemetry = async () => {
      try {
        // Reach out to the Rust GET endpoint
        const response = await fetch('http://127.0.0.1:8080/api/dashboard');
        
        // If the server returns a 404 or 500 error, throw a fit so the 'catch' block handles it.
        if (!response.ok) throw new Error('Failed to fetch telemetry data');
        
        // Convert the raw JSON text from Rust into a usable JavaScript object/array
        const data = await response.json();
        
        // Save the data to React's memory. This triggers the screen to redraw!
        setLogs(data);
        setConnectionError(null); // Clear any previous errors
      } catch (err) {
        // If the Rust server is off, this catches the failure and updates the error state.
        setConnectionError("Cannot connect to the TelemetRust Aggregator.");
        console.error(err); // Prints the exact error to the browser's hidden developer console
      } finally {
        // Whether it succeeded or failed, we are done loading.
        setLoading(false);
      }
    };

    // Run the fetch function immediately when the page loads
    fetchTelemetry();
    
    // Set up a repeating timer (polling) to fetch fresh data every 3,000 milliseconds (3 seconds)
    const interval = setInterval(fetchTelemetry, 3000);
    
    // Cleanup Function: If the user closes this component, destroy the timer so it doesn't run forever in the background causing memory leaks.
    return () => clearInterval(interval);
  }, []); // The empty array [] tells React: "Only set up this interval ONCE when the app first boots."

  // 4. HELPER FUNCTIONS
  // Takes the ugly UTC timestamp from Rust and formats it to your local time with milliseconds.
  const formatTime = (isoString) => {
    const date = new Date(isoString);
    return date.toLocaleTimeString() + '.' + date.getMilliseconds().toString().padStart(3, '0');
  };

  // 5. THE UI (JSX)
  // This is what actually gets drawn to the screen. It looks like HTML, but it's powered by JavaScript.
  return (
    <div className="dashboard-container">
      <header className="header">
        <h1>TelemetRust Command Center</h1>
      </header>

      {/* CONDITIONAL RENDERING: Logical AND (&&) */}
      {/* If 'connectionError' has text in it, draw the red error box. If it's null, draw nothing. */}
      {connectionError && (
        <div className="log-card error" style={{ marginBottom: '2rem' }}>
          <h3 style={{ margin: 0, color: 'var(--error-red)' }}>System Offline</h3>
          <p>{connectionError}</p>
        </div>
      )}

      {/* CONDITIONAL RENDERING: The Ternary Operator (condition ? true : false) */}
      {/* If loading is true (and no error), show the text. Otherwise (:), draw the log feed. */}
      {loading && !connectionError ? (
        <p>Initializing telemetry stream...</p>
      ) : (
        <div className="log-feed">
          
          {/* MAPPING: Looping through arrays */}
          {/* We take our 'logs' array and use .map() to create a visual HTML card for every single item in the array. */}
          {logs.map((log) => (
            
            // Every item in a list needs a unique 'key' so React can track it efficiently if it moves or deletes.
            // We use template literals (`string ${variable}`) to dynamically inject the log.level (e.g., "error", "warn") as a CSS class name!
            <div key={log.id} className={`log-card ${log.level.toLowerCase()}`}>
              
              <div className="log-header">
                {/* We pass the timestamp to our helper function before drawing it */}
                <span className="timestamp">{formatTime(log.timestamp)}</span>
                <span className="badge">[{log.level}]</span>
              </div>
              
              <p className="log-message">{log.message}</p>

              {/* The Force Multiplier: Conditionally render the AI analysis if it exists. */}
              {/* Because INFO and WARN logs have a null analysis, this completely skips them. */}
              {log.analysis && (
                <div className="ai-triage-panel">
                  <h4>Automated Triage Diagnosis</h4>
                  <p>{log.analysis}</p>
                </div>
              )}
              
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// 6. EXPORT
// Makes this component available so the rest of the Vite app can find it and mount it to the screen.
export default App;