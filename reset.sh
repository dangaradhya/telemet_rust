#!/bin/bash
echo "🗑️ Wiping telemetry database..."
rm -f telemetry.db telemetry.db-shm telemetry.db-wal
echo "✨ Database reset. The server will recreate it on next boot."