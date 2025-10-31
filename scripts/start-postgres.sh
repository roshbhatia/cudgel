#!/usr/bin/env bash
set -e

# XDG support
XDG_DATA_HOME="${XDG_DATA_HOME:-${HOME}/.local/share}"
PGDATA="${XDG_DATA_HOME}/cudgel/postgres"
PGPORT=54321
PGLOG="${XDG_DATA_HOME}/cudgel/postgres.log"

# Find PostgreSQL binaries (try 17, then 16, then PATH)
if [ -d "/opt/homebrew/opt/postgresql@17/bin" ]; then
    export PATH="/opt/homebrew/opt/postgresql@17/bin:$PATH"
elif [ -d "/usr/local/opt/postgresql@17/bin" ]; then
    export PATH="/usr/local/opt/postgresql@17/bin:$PATH"
elif [ -d "/opt/homebrew/opt/postgresql@16/bin" ]; then
    export PATH="/opt/homebrew/opt/postgresql@16/bin:$PATH"
elif [ -d "/usr/local/opt/postgresql@16/bin" ]; then
    export PATH="/usr/local/opt/postgresql@16/bin:$PATH"
fi

# Verify commands exist
if ! command -v initdb &> /dev/null; then
    echo "Error: PostgreSQL not found. Please install PostgreSQL:"
    echo "  brew install postgresql@17  # Recommended (has pgvector support)"
    echo "  brew install postgresql@16"
    echo "Or enter nix-shell to use the Nix-provided PostgreSQL"
    exit 1
fi

# Create data directory if needed
if [ ! -d "$PGDATA" ]; then
    echo "Initializing PostgreSQL data directory..."
    mkdir -p "$PGDATA"
    initdb -D "$PGDATA" --username="$USER" --auth=trust

    # Configure for local use
    cat >> "$PGDATA/postgresql.conf" <<EOF

# Cudgel-specific settings
port = $PGPORT
unix_socket_directories = '/tmp'
EOF
fi

# Check if already running
if pg_isready -p $PGPORT -h localhost > /dev/null 2>&1; then
    echo "PostgreSQL already running on port $PGPORT"
    exit 0
fi

# Start PostgreSQL
echo "Starting PostgreSQL on port $PGPORT..."
mkdir -p "$(dirname "$PGLOG")"
pg_ctl -D "$PGDATA" -l "$PGLOG" start

# Wait for startup
echo "Waiting for PostgreSQL..."
for i in {1..30}; do
    if pg_isready -p $PGPORT -h localhost > /dev/null 2>&1; then
        echo "PostgreSQL is ready!"

        # Create database and extension
        createdb -p $PGPORT -h localhost cudgel 2>/dev/null || true
        psql -p $PGPORT -h localhost -d cudgel -c "CREATE EXTENSION IF NOT EXISTS vector;" 2>/dev/null || true

        echo "PostgreSQL running on port $PGPORT"
        echo "Logs: $PGLOG"
        exit 0
    fi
    sleep 1
done

echo "Failed to start PostgreSQL"
exit 1
