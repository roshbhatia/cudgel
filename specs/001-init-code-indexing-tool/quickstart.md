# Quickstart Guide: Cudgel

**Date**: 2025-10-31
**Feature**: 001-init-code-indexing-tool

## What is Cudgel?

Cudgel is a local-first codebase intelligence system that helps you understand and search your code using natural language. It indexes your git repositories, generates semantic embeddings, and enables fast similarity search—all running locally on your machine without sending code to external services.

## Prerequisites

Before using Cudgel, ensure you have:

- **Nix** with flakes enabled (recommended install method)
- **PostgreSQL 14+** (provided by Nix flake)
- **Ollama** with llama3.2:8b model (provided by Nix flake)
- **Git** (for repository tracking)
- **Unix-like OS**: macOS or Linux with XDG support

## Installation

### Method 1: Nix Flake (Recommended)

```bash
# Clone repository
git clone https://github.com/your-org/cudgel.git
cd cudgel

# Enter development shell (includes all dependencies)
nix develop

# Build and install
nix build
sudo cp result/bin/cudgel /usr/local/bin/
```

### Method 2: Cargo Install (Alternative)

```bash
# Install system dependencies first
brew install postgresql@16 ollama  # macOS
# or
apt-get install postgresql-16 ollama  # Debian/Ubuntu

# Install from crates.io
cargo install cudgel

# Or build from source
git clone https://github.com/your-org/cudgel.git
cd cudgel
cargo install --path .
```

## Initial Setup

### 1. Start PostgreSQL

```bash
# Using Nix (automated)
nix develop  # PostgreSQL starts automatically

# Or manually (if not using Nix)
# Create data directory
mkdir -p ~/.local/share/cudgel/postgres
initdb -D ~/.local/share/cudgel/postgres

# Start on custom port to avoid conflicts
postgres -D ~/.local/share/cudgel/postgres -p 54321 &

# Create database
createdb -p 54321 cudgel
```

### 2. Enable pgvector Extension

```bash
# Connect to database
psql -p 54321 -d cudgel

# Enable extension
CREATE EXTENSION IF NOT EXISTS vector;

# Exit
\q
```

### 3. Start Ollama

```bash
# Start Ollama service (runs in background)
ollama serve &

# Pull llama3.2:8b model (one-time download, ~4.7GB)
ollama pull llama3.2:8b

# Verify model is available
ollama list
```

### 4. Verify Installation

```bash
# Check cudgel is installed
cudgel --version

# Check dependencies are running
cudgel doctor  # (fictional command - actual implementation would verify deps)
```

Expected output:
```
Cudgel v0.1.0

✓ PostgreSQL running on localhost:54321
✓ Database 'cudgel' exists
✓ pgvector extension enabled
✓ Ollama service running on localhost:11434
✓ Model 'llama3.2:8b' available
✓ XDG directories configured
```

## Quick Start: 5-Minute Tour

### Step 1: Index Your First Repository

```bash
# Index a local project
cd ~/projects/my-rust-project
cudgel index .

# Or specify path explicitly
cudgel index ~/projects/my-rust-project
```

Expected output:
```
Indexing repository: /Users/you/projects/my-rust-project
Discovering git-tracked files...
Found 128 source files (Python, Rust, JavaScript)

Parsing files...
[========================================] 100% (128/128 files)
Extracted 456 symbols (functions, classes, methods)

Generating embeddings...
[========================================] 100% (456/456 symbols)

✓ Indexed 128 files, 456 symbols in 12.3s
```

### Step 2: Search Your Code

```bash
# Basic semantic search
cudgel query "authentication logic"
```

Expected output:
```
╭──────────────────────────────────────────────────────────────────╮
│ Similarity │ File                │ Line │ Symbol           │
├────────────┼─────────────────────┼──────┼──────────────────┤
│ 0.94       │ src/auth/mod.rs     │ 42   │ authenticate_user│
│ 0.89       │ src/auth/token.rs   │ 89   │ verify_jwt_token │
│ 0.85       │ src/middleware.rs   │ 156  │ auth_middleware  │
╰──────────────────────────────────────────────────────────────────╯

Showing 3 results (0.18s)
```

### Step 3: Schedule Automatic Re-indexing

```bash
# Re-index hourly (useful for actively developed projects)
cudgel index --schedule hourly .

# Start orchestrator daemon to run scheduled tasks
cudgel orchestrator start
```

Expected output:
```
✓ Repository scheduled for hourly re-indexing
✓ Orchestrator daemon started (PID: 12345)

Logs: ~/.local/state/cudgel/orchestrator.log
```

### Step 4: Generate Knowledge Graph

```bash
# Generate AI-powered documentation
cudgel knowledge
```

This opens your `$EDITOR` with a markdown document like:
```markdown
# Repository: my-rust-project

## Design & Architecture

This project implements a web service using the Actix framework with
a service-oriented architecture. The main components are:

- `src/services/`: Business logic (UserService, AuthService, DataService)
- `src/handlers/`: HTTP request handlers
- `src/models/`: Database models (SQLx)
- `src/middleware/`: Authentication and logging middleware

The architecture follows a layered pattern where handlers depend on
services, and services depend on database models.

## Dependencies

Core dependencies:
- `actix-web` (4.4.0): Web framework
- `sqlx` (0.7.0): Async PostgreSQL client
- `tokio` (1.35.0): Async runtime
- `serde` (1.0): Serialization

## Build Process

```bash
# Development build
cargo build

# Run tests
cargo test

# Production release
cargo build --release

# Run locally
cargo run --bin my-rust-project
```

## Licensing

MIT License - permissive open source license allowing commercial use,
modification, and distribution with attribution.
```

## Common Use Cases

### Use Case 1: Find Related Code

You're working on a feature and need to find similar implementations:

```bash
# Find functions related to database migrations
cudgel query "database schema migration" --type function

# Limit to specific language
cudgel query "error handling" --language rust --limit 10
```

### Use Case 2: Onboarding New Developers

Help new team members understand the codebase:

```bash
# Generate comprehensive knowledge graph
cudgel knowledge --output ONBOARDING.md --no-editor

# Search for architectural patterns
cudgel query "dependency injection pattern"

# Find entry points
cudgel query "main function application startup"
```

### Use Case 3: LLM Context for Code Review

Provide code context to AI assistants without copy-pasting:

```bash
# Get minified output for token efficiency
cudgel query "user authentication flow" --minified > auth_context.json

# Pipe to LLM (example with hypothetical CLI)
cudgel query "payment processing" --minified | llm prompt "Review this code for security issues:"
```

### Use Case 4: Cross-Repository Search

Search across multiple indexed repositories:

```bash
# Index multiple projects
cudgel index ~/projects/frontend
cudgel index ~/projects/backend
cudgel index ~/projects/shared-lib

# Search returns results from all repos
cudgel query "API client implementation"
```

### Use Case 5: Automated Re-indexing for Active Projects

Keep your most-used repositories always up-to-date:

```bash
# Schedule hourly re-indexing for active project
cudgel index --schedule hourly ~/projects/main-app

# Schedule daily for less active projects
cudgel index --schedule daily ~/projects/legacy-system

# Custom interval (every 6 hours)
cudgel index --schedule 6 ~/projects/library

# Start orchestrator to run schedules
cudgel orchestrator start

# Check what's scheduled
cudgel orchestrator status
```

## Configuration

### Default Configuration

Cudgel works out-of-the-box with zero configuration. Default settings:

- **Database**: PostgreSQL on `localhost:54321`, database `cudgel`
- **Ollama**: `http://localhost:11434`, model `llama3.2:8b`
- **Orchestrator**: Poll every 60 seconds
- **Query**: Return top 50 results
- **Logging**: Info level

### Custom Configuration

Create `~/.config/cudgel/config.toml` to override defaults:

```toml
[database]
host = "localhost"
port = 54321
database = "cudgel"
user = "your-username"

[ollama]
url = "http://localhost:11434"
model = "llama3.2:8b"
timeout_secs = 120

[orchestrator]
poll_interval_secs = 60
max_concurrent_tasks = 5

[indexer]
max_file_size_bytes = 10485760  # 10MB
ignored_patterns = [".git", "node_modules", "target", "dist"]

[query]
default_limit = 50
max_limit = 1000

[logging]
level = "info"  # error, warn, info, debug, trace
```

### Environment Variables

Override any setting via environment variables:

```bash
# Database configuration
export CUDGEL_DATABASE__PORT=5432

# Ollama configuration
export CUDGEL_OLLAMA__MODEL=llama3.2:latest

# Logging level
export CUDGEL_LOGGING__LEVEL=debug

# Run commands with overrides
cudgel index /path/to/repo
```

## Troubleshooting

### PostgreSQL Not Running

**Error**: `PostgreSQL is not running on port 54321`

**Solution**:
```bash
# Check if running
ps aux | grep postgres

# Start manually
postgres -D ~/.local/share/cudgel/postgres -p 54321 &

# Or use Nix (starts automatically)
nix develop
```

### Ollama Not Running

**Error**: `Ollama service not available at localhost:11434`

**Solution**:
```bash
# Check if running
curl http://localhost:11434/api/tags

# Start Ollama
ollama serve &

# Verify model is available
ollama list
# If llama3.2:8b not listed:
ollama pull llama3.2:8b
```

### Repository Not a Git Repo

**Error**: `Not a git repository or git not found`

**Solution**:
```bash
# Ensure repository has .git directory
cd /path/to/repo
git status

# If not a git repo, initialize it
git init
git add .
git commit -m "Initial commit"

# Then index
cudgel index .
```

### Out of Memory During Indexing

**Error**: `process killed (OOM)` or slow indexing

**Solution**:
```bash
# Reduce batch size via environment variable
export CUDGEL_INDEXER__BATCH_SIZE=50  # default: 100

# Or increase system memory available to Ollama
# Edit Ollama config to use less VRAM
```

### Embedding Generation Timeout

**Error**: `Timeout generating embedding after 30s`

**Solution**:
```bash
# Increase timeout in config.toml
[ollama]
timeout_secs = 180  # 3 minutes instead of 2

# Or via environment variable
export CUDGEL_OLLAMA__TIMEOUT_SECS=180

cudgel index /path/to/repo
```

## Performance Tips

### Faster Indexing

```bash
# Use --schedule to index incrementally (only changed files)
cudgel index --schedule hourly .

# Skip large files (configure in config.toml)
[indexer]
max_file_size_bytes = 5242880  # 5MB instead of default 10MB
```

### Faster Queries

```bash
# Limit results for quick searches
cudgel query "search term" --limit 10

# Use specific filters to reduce search space
cudgel query "parser" --language rust --type function
```

### Reduce Memory Usage

```bash
# Use smaller Ollama model (less accurate but faster)
export CUDGEL_OLLAMA__MODEL=llama3.2:3b  # 3B instead of 8B

# Limit concurrent indexing tasks
[indexer]
max_concurrent_files = 5  # default: 10
```

## Next Steps

### Learn More

- Read the full [CLI Interface Contract](./contracts/cli-interface.md)
- Understand the [Data Model](./data-model.md)
- Review [Research and Best Practices](./research.md)

### Integrate with Your Workflow

```bash
# Add to shell startup (~/.bashrc or ~/.zshrc)
# Start orchestrator on login
cudgel orchestrator start > /dev/null 2>&1

# Create aliases for common queries
alias find-auth='cudgel query "authentication" --type function'
alias find-tests='cudgel query "test" --type function'

# Add shell completion
cudgel --completions bash > /usr/local/share/bash-completion/completions/cudgel
```

### Advanced Usage

- **Scripting**: Use `--json` output with `jq` for automation
- **CI/CD Integration**: Index on deploy, generate knowledge graphs for docs
- **Editor Integration**: Create vim/emacs/vscode plugins using CLI interface

## Support

- **Issues**: https://github.com/your-org/cudgel/issues
- **Discussions**: https://github.com/your-org/cudgel/discussions
- **Documentation**: https://docs.cudgel.dev

## Summary

You've learned to:
- ✅ Install Cudgel with Nix or Cargo
- ✅ Set up PostgreSQL and Ollama dependencies
- ✅ Index your first repository
- ✅ Search code with semantic queries
- ✅ Schedule automatic re-indexing
- ✅ Generate AI-powered knowledge graphs

Cudgel is now ready to help you understand and navigate your codebase faster!
