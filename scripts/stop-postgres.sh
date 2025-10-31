#!/usr/bin/env bash

# XDG support
XDG_DATA_HOME="${XDG_DATA_HOME:-${HOME}/.local/share}"
PGDATA="${XDG_DATA_HOME}/cudgel/postgres"
PGPORT=54321

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

if ! pg_isready -p $PGPORT -h localhost > /dev/null 2>&1; then
    echo "PostgreSQL is not running"
    exit 0
fi

echo "Stopping PostgreSQL..."
pg_ctl -D "$PGDATA" stop
echo "PostgreSQL stopped"
