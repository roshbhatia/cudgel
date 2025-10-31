# Cudgel Quick Start Guide

Get started with Cudgel in under 5 minutes!

## Prerequisites

- **Rust 1.70+** - [Install Rust](https://rustup.rs/)
- **Homebrew** - [Install Homebrew](https://brew.sh) (macOS only)
- **Docker** - [Install Docker](https://docs.docker.com/get-docker/) (for Temporal)

## Installation

```bash
# Clone the repository
git clone https://github.com/roshbhatia/cudgel.git
cd cudgel

# Build and install
cargo install --path .
```

## One-Time Setup

Run the setup command once to configure services:

```bash
cudgel setup
```

This will:
1. Install PostgreSQL 16 via Homebrew (if not already installed)
2. Create macOS Launch Agents for PostgreSQL and Temporal
3. Initialize the cudgel database with pgvector extension
4. Configure services to start automatically on login

**That's it!** Services will now start automatically whenever you log in.

## Index Your First Repository

Now you can index your code:

```bash
# Index the current directory
cudgel index .

# Or index a specific repository
cudgel index /path/to/your/repo
```

You'll see a progress bar and statistics:
```
Indexing repository...
Path: /path/to/repo

Found 150 files to index
[00:00:45] ===================> 150/150 Indexing: src/main.rs

Successfully indexed repository with ID: 1

Indexing Statistics:
 Files: 150 total, 148 indexed, 2 failed
 Symbols: 423 total

 Files by language:
 rust: 45
 python: 50
 javascript: 53

 Symbols by kind:
 function: 285
 class: 95
 method: 43
```

## Service Management

Manage your services with the `services` command:

```bash
# Check service status
cudgel services status

# Start services manually (if stopped)
cudgel services start

# Stop services
cudgel services stop

# Remove Launch Agents
cudgel services remove
```

## Query Your Code

Now you can search your code using natural language:

```bash
# Find authentication functions
cudgel query "function that handles user authentication"

# Find database connections
cudgel query "database connection code"

# Find error handling
cudgel query "error handling functions"
```

## Explore Code Relationships

```bash
# See what references a function
cudgel graph authenticate_user

# See what a function calls (2 levels deep)
cudgel graph process_request --depth 2
```

## Scheduled Re-Indexing

Keep your index fresh with automatic re-indexing:

```bash
# Re-index every hour
cudgel index . --schedule hourly

# Re-index every day
cudgel index /path/to/repo --schedule daily

# Re-index every 6 hours
cudgel index /path/to/repo --schedule 6
```

Cudgel uses Temporal workflows (auto-started) to schedule periodic indexing.

## Advanced Usage

### Multiple Repositories

```bash
cudgel index ~/projects/project1
cudgel index ~/projects/project2
cudgel index ~/work/important-app
```

All repositories are stored in the same database for cross-project search.

### Advanced Queries

```bash
# Limit results
cudgel query "authentication" --limit 5

# Filter by repository
cudgel query "error handling" --repo /path/to/repo

# JSON output for scripting
cudgel query "database" --json
```

### Graph Queries

```bash
# Deep traversal (3 levels)
cudgel graph main --depth 3

# JSON output
cudgel graph UserService --json
```

## Supported Languages

Cudgel automatically detects and indexes these languages:
- Python (`.py`)
- JavaScript/JSX (`.js`, `.jsx`)
- TypeScript/TSX (`.ts`, `.tsx`)
- Rust (`.rs`)
- Go (`.go`)
- C/C++ (`.c`, `.cpp`, `.h`, `.hpp`)
- Java (`.java`)

## How It Works

```

 ‚ CLI ‚ Your commands (cudgel index ., cudgel query "...")
 ¬ ˜
 ‚
 ¼

 ‚ Auto-Service Manager ‚
 ‚ ‚
 ‚ ‚ Docker Compose ‚ ‚
 ‚ ‚ PostgreSQL + pgvector ‚ ‚
 ‚ ‚ Temporal (for scheduling) ‚ ‚
 ‚ ˜ ‚
 ¬ ˜
 ‚
 ¼

 ‚ Cudgel Core Engine ‚
 ‚ ‚
 ‚ ‚ Tree- ‚ ‚ Indexer ‚ ‚
 ‚ ‚ sitter ‚ ¬ ˜ ‚
 ‚ ˜ ‚ ‚
 ‚ ¼ ‚
 ‚ ‚Embedding ‚ ‚ Database ‚ ‚
 ‚ ‚Generator ‚ ‚ (auto- ‚ ‚
 ‚ ‚ ‚ ‚ init) ‚ ‚
 ‚ ˜ ¬ ˜ ‚
 ‚ ‚ ‚
 ‚ ¼ ‚
 ‚ ‚ Query ‚ ‚ Graph ‚ ‚
 ‚ ‚ Engine ‚ ‚ Query ‚ ‚
 ‚ ˜ ˜ ‚
 ˜
```

**Key Features:**
- **Zero config**: No env files, no manual database setup
- **Auto-start**: Services start automatically when needed
- **Auto-init**: Database schema initializes on first connection
- **Auto-schedule**: Use `--schedule` flag for periodic re-indexing

## Troubleshooting

### Services won't start

```bash
# Check Docker is running
docker --version
docker ps

# Manually check services
docker ps | grep cudgel
```

If Docker isn't running, start Docker Desktop and try again.

### Reset everything

```bash
# Stop all services
docker compose -p cudgel down -v

# Try again
cudgel index .
```

### Check service logs

```bash
# View PostgreSQL logs
docker logs cudgel-postgres-1

# View Temporal logs
docker logs cudgel-temporal-1
```

## Getting Help

```bash
# See all commands
cudgel --help

# Get help for specific command
cudgel index --help
cudgel query --help
```

## What's Next?

- Read the [full README](README.md) for architecture details
- See [CLAUDE.md](CLAUDE.md) for development guide
- Check out the source code to understand the internals

Happy indexing!
