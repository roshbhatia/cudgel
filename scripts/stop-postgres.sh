#!/usr/bin/env bash

# XDG support
XDG_DATA_HOME="${XDG_DATA_HOME:-${HOME}/.local/share}"
PGDATA="${XDG_DATA_HOME}/cudgel/postgres"
PGPORT="${CUDGEL_POSTGRES_PORT:-45678}"

if ! pg_isready -p $PGPORT -h localhost > /dev/null 2>&1; then
    echo "PostgreSQL is not running"
    exit 0
fi

echo "Stopping PostgreSQL..."
pg_ctl -D "$PGDATA" stop
echo "PostgreSQL stopped"
