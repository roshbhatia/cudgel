# CLI Interface Contract: Cudgel

**Date**: 2025-10-31
**Feature**: 001-init-code-indexing-tool

## Overview

This document defines the command-line interface contract for Cudgel. All commands follow Unix conventions (exit codes, stderr for errors, stdout for output, flags with `-`/`--` prefixes).

## Global Options

Available for all commands:

```
--config <PATH>      Path to config file (default: ~/.config/cudgel/config.toml)
--log-level <LEVEL>  Logging verbosity: error, warn, info, debug, trace (default: info)
--help, -h           Show help message
--version, -V        Show version information
```

## Commands

### 1. `cudgel index`

Index a git repository for semantic search.

**Usage**:
```bash
cudgel index [OPTIONS] <PATH>
```

**Arguments**:
- `<PATH>` (required): Absolute or relative path to git repository root

**Options**:
- `--schedule <INTERVAL>`: Schedule automatic re-indexing
  - Values: `hourly`, `daily`, or integer hours (e.g., `6` for every 6 hours)
  - Example: `--schedule hourly`
- `--unschedule`: Remove existing schedule for this repository
- `--force`: Force re-index all files (ignore content hash checks)
- `--dry-run`: Show what would be indexed without performing indexing

**Output** (stdout):
```
Indexing repository: /path/to/repo
[========================================] 100% (1234/1234 files)
Indexed 1234 files, extracted 5678 symbols in 45.2s
```

**JSON Output** (with `--json` flag):
```json
{
  "repo_path": "/path/to/repo",
  "file_count": 1234,
  "symbol_count": 5678,
  "duration_secs": 45.2,
  "schedule": "hourly"
}
```

**Exit Codes**:
- `0`: Success
- `1`: Invalid path or not a git repository
- `2`: Database connection failed
- `3`: Ollama service not available
- `4`: Permission denied

**Examples**:
```bash
# Basic indexing
cudgel index /path/to/my-project

# Index with hourly re-indexing
cudgel index --schedule hourly /path/to/my-project

# Index every 6 hours
cudgel index --schedule 6 /path/to/my-project

# Remove schedule
cudgel index --unschedule /path/to/my-project

# Dry run to preview
cudgel index --dry-run /path/to/my-project
```

---

### 2. `cudgel query`

Search indexed code using semantic similarity.

**Usage**:
```bash
cudgel query [OPTIONS] <SEARCH_TERM>
```

**Arguments**:
- `<SEARCH_TERM>` (required): Natural language search query (quote if contains spaces)

**Options**:
- `--limit <N>`: Maximum number of results (default: 50, max: 1000)
- `--repo <PATH>`: Limit search to specific repository
- `--language <LANG>`: Filter by language (python, rust, javascript, etc.)
- `--type <TYPE>`: Filter by symbol type (function, class, method, etc.)
- `--json`: Output results as compact JSON
- `--json-pretty`: Output results as formatted JSON
- `--minified`: Output results in LLM-optimized minified format

**Output** (stdout, default table format):
```
╭─────────────────────────────────────────────────────────────────────────╮
│ Similarity │ Repository      │ File             │ Line │ Symbol         │
├────────────┼─────────────────┼──────────────────┼──────┼────────────────┤
│ 0.95       │ my-project      │ src/auth.rs      │ 42   │ authenticate   │
│ 0.88       │ my-project      │ src/user.rs      │ 128  │ verify_token   │
│ 0.82       │ other-project   │ lib/auth.py      │ 67   │ check_password │
╰─────────────────────────────────────────────────────────────────────────╯

Showing 3 results (0.15s)
```

**JSON Output** (with `--json` flag):
```json
[
  {
    "similarity": 0.95,
    "repo_path": "/path/to/my-project",
    "file_path": "src/auth.rs",
    "line_number": 42,
    "symbol_name": "authenticate",
    "symbol_type": "function",
    "code_snippet": "fn authenticate(user: &User, password: &str) -> Result<Token> { ... }",
    "documentation": "Authenticates a user with username and password"
  }
]
```

**Minified Output** (with `--minified` flag):
```json
[{"s":0.95,"r":"my-project","f":"src/auth.rs","l":42,"n":"authenticate","t":"function","c":"fn authenticate(user: &User, password: &str) -> Result<Token> { ... }"}]
```

**Exit Codes**:
- `0`: Success (results found)
- `1`: No results found
- `2`: Database connection failed
- `3`: Ollama service not available (embedding generation failed)
- `4`: Invalid query (empty string)

**Examples**:
```bash
# Basic search
cudgel query "authentication logic"

# Search with filters
cudgel query "parse configuration" --language rust --type function

# Limit results
cudgel query "error handling" --limit 10

# JSON output for piping
cudgel query "http client" --json | jq '.[] | .file_path'

# Minified for LLM context
cudgel query "database operations" --minified > context.json
```

---

### 3. `cudgel knowledge`

Generate or edit AI-powered knowledge graph documentation.

**Usage**:
```bash
cudgel knowledge [OPTIONS] [PATH]
```

**Arguments**:
- `[PATH]` (optional): Path to repository (default: current directory)

**Options**:
- `--edit`: Open existing knowledge document for editing
- `--refresh`: Update auto-generated sections while preserving manual edits
- `--replace`: Completely regenerate knowledge graph (discards edits)
- `--output <FILE>`: Write to file instead of opening in editor
- `--no-editor`: Print to stdout instead of opening editor

**Output**:
Opens `$EDITOR` with markdown document:
```markdown
# Repository: my-project

## Design & Architecture

This project follows a service-oriented architecture with...

## Dependencies

- clap: CLI argument parsing
- tokio: Async runtime
- sqlx: PostgreSQL database client
...

## Build Process

```bash
cargo build --release
cargo test
```

## Licensing

MIT License - see LICENSE file for details
```

**JSON Output** (with `--json --no-editor` flags):
```json
{
  "repo_path": "/path/to/my-project",
  "content": "# Repository: my-project\n\n...",
  "generated_at": "2025-10-31T12:00:00Z",
  "last_edited_at": null,
  "version": 1
}
```

**Exit Codes**:
- `0`: Success
- `1`: Repository not indexed
- `2`: Database connection failed
- `3`: Ollama service not available
- `4`: $EDITOR not set and no fallback available

**Examples**:
```bash
# Generate knowledge graph for current directory
cudgel knowledge

# Generate for specific repository
cudgel knowledge /path/to/my-project

# Edit existing knowledge document
cudgel knowledge --edit

# Refresh auto-generated sections
cudgel knowledge --refresh

# Completely regenerate (lose edits)
cudgel knowledge --replace

# Save to file without editor
cudgel knowledge --output knowledge.md --no-editor
```

---

### 4. `cudgel orchestrator`

Manage background daemon for scheduled indexing.

**Usage**:
```bash
cudgel orchestrator <SUBCOMMAND>
```

**Subcommands**:

#### `cudgel orchestrator start`

Start the orchestrator daemon in the background.

**Options**:
- `--foreground`: Run in foreground (don't daemonize)
- `--poll-interval <SECS>`: Polling interval in seconds (default: 60)

**Output** (stdout):
```
Orchestrator daemon started (PID: 12345)
Log file: ~/.local/state/cudgel/orchestrator.log
```

**Exit Codes**:
- `0`: Daemon started successfully
- `1`: Daemon already running
- `2`: Database connection failed

#### `cudgel orchestrator stop`

Stop the running orchestrator daemon.

**Output** (stdout):
```
Orchestrator daemon stopped (PID: 12345)
```

**Exit Codes**:
- `0`: Daemon stopped successfully
- `1`: Daemon not running

#### `cudgel orchestrator status`

Check daemon status and list scheduled tasks.

**Output** (stdout, table format):
```
Orchestrator Status: Running (PID: 12345)
Uptime: 2h 34m
Last poll: 2025-10-31 12:34:56

Scheduled Tasks:
╭────────────────────────────────────────────────────────────────────╮
│ Repository           │ Interval │ Next Run            │ Status   │
├──────────────────────┼──────────┼─────────────────────┼──────────┤
│ /path/to/my-project  │ 1 hour   │ 2025-10-31 13:00:00 │ active   │
│ /path/to/other-proj  │ 6 hours  │ 2025-10-31 18:00:00 │ active   │
╰────────────────────────────────────────────────────────────────────╯
```

**JSON Output** (with `--json` flag):
```json
{
  "status": "running",
  "pid": 12345,
  "uptime_secs": 9240,
  "last_poll_at": "2025-10-31T12:34:56Z",
  "scheduled_tasks": [
    {
      "repo_path": "/path/to/my-project",
      "interval_hours": 1,
      "next_run_at": "2025-10-31T13:00:00Z",
      "status": "active"
    }
  ]
}
```

**Exit Codes**:
- `0`: Status retrieved successfully
- `1`: Daemon not running
- `2`: Database connection failed

#### `cudgel orchestrator restart`

Restart the orchestrator daemon.

**Output** (stdout):
```
Stopping orchestrator daemon (PID: 12345)
Starting orchestrator daemon (PID: 67890)
```

**Exit Codes**:
- `0`: Daemon restarted successfully
- `1`: Failed to stop old daemon
- `2`: Failed to start new daemon

**Examples**:
```bash
# Start daemon
cudgel orchestrator start

# Check status
cudgel orchestrator status

# Run in foreground for debugging
cudgel orchestrator start --foreground

# Stop daemon
cudgel orchestrator stop

# Restart daemon
cudgel orchestrator restart
```

---

## Common Patterns

### Environment Variables

All config file settings can be overridden via environment variables with `CUDGEL_` prefix:

```bash
# Database configuration
export CUDGEL_DATABASE__PORT=5432
export CUDGEL_DATABASE__HOST=localhost

# Ollama configuration
export CUDGEL_OLLAMA__URL=http://localhost:11434
export CUDGEL_OLLAMA__MODEL=llama3.2:8b

# Logging
export CUDGEL_LOGGING__LEVEL=debug

# Run command with overrides
cudgel index /path/to/repo
```

### Piping and Composition

Cudgel commands support Unix pipelines:

```bash
# Search and extract file paths
cudgel query "database migrations" --json | jq -r '.[].file_path'

# Count symbols by type
cudgel query "all functions" --json | jq '[.[].symbol_type] | group_by(.) | map({type: .[0], count: length})'

# Feed query results to LLM
cudgel query "authentication" --minified | llm prompt "Explain this code:"
```

### Error Handling

All commands write errors to stderr:

```bash
# Capture errors
cudgel index /nonexistent 2> errors.log

# Ignore errors
cudgel index /path/to/repo 2> /dev/null

# Check exit code
if ! cudgel query "search term" > /dev/null 2>&1; then
    echo "Query failed"
fi
```

### Progress and Verbosity

Control output verbosity:

```bash
# Quiet mode (errors only)
cudgel --log-level error index /path/to/repo

# Verbose mode (debug info)
cudgel --log-level debug index /path/to/repo

# Trace mode (very detailed)
cudgel --log-level trace query "search term"
```

## Shell Completion

Cudgel provides shell completion scripts for bash, zsh, fish:

```bash
# Bash
cudgel --completions bash > /usr/local/share/bash-completion/completions/cudgel

# Zsh
cudgel --completions zsh > ~/.zsh/completions/_cudgel

# Fish
cudgel --completions fish > ~/.config/fish/completions/cudgel.fish
```

## Contract Validation

All CLI behaviors are tested via integration tests:

```bash
# Test basic commands
cargo test --test cli_index_test
cargo test --test cli_query_test
cargo test --test cli_knowledge_test
cargo test --test cli_orchestrator_test

# Test error cases
cargo test --test cli_errors_test

# Test output formats
cargo test --test cli_json_output_test
```

## Summary

The CLI contract defines:
- 4 main commands: `index`, `query`, `knowledge`, `orchestrator`
- Consistent flag naming (`--json`, `--limit`, etc.)
- Standard exit codes (0 = success, 1-4 = specific errors)
- Multiple output formats (table, JSON, minified)
- Unix-friendly behavior (stderr for errors, pipeable stdout)
- Environment variable overrides for all settings

All commands follow constitution principles (local-first, fail-fast with actionable errors, structured logging).
