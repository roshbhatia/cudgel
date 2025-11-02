#!/usr/bin/env bash
set -e

# XDG support
XDG_DATA_HOME="${XDG_DATA_HOME:-${HOME}/.local/share}"
PGDATA="${XDG_DATA_HOME}/cudgel/postgres"
PGPORT="${CUDGEL_POSTGRES_PORT:-45678}"
PGLOG="${XDG_DATA_HOME}/cudgel/postgres.log"

# Verify PostgreSQL is available (use system PATH, including Nix)
if ! command -v initdb &> /dev/null; then
    echo "Error: PostgreSQL not found. Please install PostgreSQL 17+ with pgvector:"
    echo "  nix-shell          # Use Nix development environment (recommended)"
    echo "  brew install postgresql@17  # macOS Homebrew"
    echo "  apt install postgresql-17 postgresql-17-pgvector  # Debian/Ubuntu"
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
