# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Cudgel is a streamlined code indexing tool combining tree-sitter parsing, PostgreSQL/pgvector embeddings, and semantic search. **CLI-only, service-oriented design** optimized for developer UX.

## Quick Start

```bash
# One-time setup
task setup

# Index a repository (incremental - only re-parses changed files)
cudgel index /path/to/repo

# Query indexed code
cudgel query "authentication handler"

# Explore code relationships
cudgel graph authenticate_user --depth 2
```

## Development Commands

### Task Automation

```bash
task setup       # Complete setup: build, install, start PostgreSQL
task build       # Build release binary
task install     # Install globally
task db-start    # Start PostgreSQL on port 54321
task db-stop     # Stop PostgreSQL
task db-status   # Check if running
task fmt         # Format code
task clippy      # Run linter
task test        # Run tests
```

### Cargo Commands

```bash
cargo build --release          # Build optimized binary
cargo test                     # Run test suite
cargo clippy -- -D warnings    # Lint (must pass)
cargo fmt                      # Format code
cargo install --path .         # Install globally
```

## Architecture

### Service-Oriented Design

```
CLI Layer (main.rs)
  ↓
Core Services
  ├─ Indexer      → Orchestrates incremental indexing
  ├─ QueryEngine  → Semantic search
  └─ GraphQuery   → Code relationship traversal
  ↓
Data Layer
  ├─ Parser       → Tree-sitter AST extraction
  ├─ Embeddings   → ONNX-based vector generation
  └─ Database     → PostgreSQL + pgvector
```

### Core Components

1. **Parser** (`src/parser.rs`): Multi-language AST parsing with tree-sitter
2. **Indexer** (`src/indexer.rs`): Incremental indexing with hash-based change detection
3. **Database** (`src/database.rs`): PostgreSQL + pgvector with connection pooling
4. **Query Engine** (`src/query.rs`): Semantic vector search
5. **Graph Query** (`src/graph.rs`): Code relationship analysis
6. **Embeddings** (`src/embeddings.rs`): ONNX Runtime sentence-transformers
7. **CLI** (`src/main.rs`): Command-line interface

### Database

- **PostgreSQL 17** on port **54321** (non-standard to avoid conflicts)
- **Data location**: `~/.local/share/cudgel/postgres` (XDG compliant)
- **Extensions**: pgvector for vector similarity search
- **Schema**: Auto-initialized on first connection
- **Indexing**: IVFFlat index for approximate nearest neighbor search

**Tables**:
- `repositories`: Indexed code repositories
- `files`: Source files with content hashing for incremental indexing
- `ast_nodes`: Tree-sitter AST nodes
- `symbols`: Extracted code symbols (functions, classes, methods) with embeddings
- `references`: Symbol relationships (calls, imports, extends)
- `code_chunks`: Semantic code chunks for search
- `scheduled_tasks`: Automatic re-indexing schedules (added for User Story 2)
- `knowledge_documents`: AI-generated repository documentation (added for User Story 3)

**Vector Indexes**:
- **HNSW** (Hierarchical Navigable Small World) indexes on `symbols.embedding` and `code_chunks.embedding`
- HNSW provides exact nearest neighbor search with excellent recall
- Better than IVFFlat for small-medium datasets (< 1M vectors)
- Optimal for production workloads from 100s to 100Ks of symbols

### Language Support

Supported via tree-sitter grammars:
- Python (`.py`, `.pyw`)
- JavaScript/JSX (`.js`, `.jsx`, `.mjs`)
- TypeScript/TSX (`.ts`, `.tsx`)
- Rust (`.rs`)
- Go (`.go`)
- C (`.c`, `.h`)
- C++ (`.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh`, `.hxx`)
- Java (`.java`)

To add a new language:
1. Add tree-sitter grammar to `Cargo.toml`
2. Update `detect_language()` in `src/parser.rs`
3. Update `get_language()` in `src/parser.rs`
4. Add symbol extraction rules in `extract_symbols_recursive()`

### Configuration

All settings in `src/config.rs` with XDG Base Directory specification support:
- PostgreSQL: `localhost:54321`
- Database: `cudgel`
- User: System username
- Embedding model: `$XDG_DATA_HOME/cudgel/models/all-MiniLM-L6-v2` (falls back to `./models/`)

**XDG Directory Structure** (checks environment variables first):
- Data: `$XDG_DATA_HOME/cudgel` (default: `~/.local/share/cudgel`)
- State: `$XDG_STATE_HOME/cudgel` (default: `~/.local/state/cudgel`)
- Cache: `$XDG_CACHE_HOME/cudgel` (default: `~/.cache/cudgel`)
- Config: `$XDG_CONFIG_HOME/cudgel` (default: `~/.config/cudgel`)

## CLI Commands

### Index Repository
```bash
# Basic indexing
cudgel index /path/to/repo          # Full index
cudgel index /path/to/repo          # Re-run: only processes changed files
cudgel index . --dry-run            # Preview what would be indexed

# Multiple paths with glob patterns (Go-style)
cudgel index ./...                  # Index current directory recursively
cudgel index src/... tests/...      # Index multiple directories

# Include/exclude patterns (glob syntax)
cudgel index . --include "**/*.rs"                    # Only Rust files
cudgel index . --include "src/**/*.rs,tests/**/*.rs"  # Multiple patterns (comma-separated)
cudgel index . --exclude "**/target/**,**/node_modules/**"  # Exclude directories
cudgel index . --include "**/*.rs" --exclude "**/*test*.rs"  # Combine include and exclude

# Language filtering
cudgel index . --languages rust,python              # Only Rust and Python files
cudgel index . --languages go,typescript            # Only Go and TypeScript files

# Combined filtering
cudgel index . \
  --include "src/**" \
  --exclude "**/*test*" \
  --languages rust,python \
  --dry-run                         # Preview filtered results
```

### Query Code
```bash
cudgel query "function that handles authentication"
cudgel query "parse configuration file" --limit 5
cudgel query "user login" --json     # JSON output
```

### Graph Relationships
```bash
cudgel graph authenticate_user
cudgel graph process_request --depth 2
cudgel graph main --json             # JSON output
```

### Database Management
```bash
cudgel init-db           # Initialize schema
cudgel init-db --reset   # Drop and recreate (deletes all data)
```

## Key Implementation Details

### Incremental Re-Indexing

Cudgel automatically detects file changes during re-indexing:
- Compares SHA256 hashes of file contents
- Skips unchanged files (no re-parsing, no re-embedding)
- Deletes old symbols and re-parses changed files
- Significantly faster for incremental updates

**Implementation** (`src/indexer.rs:index_file`):
```rust
let existing_hash = db.get_file_hash(repo_id, &path).await?;
if let Some(old_hash) = existing_hash {
    if old_hash == hash {
        return Ok(file_id);  // Skip unchanged file
    }
    db.delete_file_symbols(file_id).await?;  // Clear old symbols
}
```

### Async Runtime
Uses Tokio throughout. All database operations are async with connection pooling via `deadpool-postgres`.

### Error Handling
Simplified error types in `src/error.rs` using `thiserror`. All public functions return `Result<T, Error>`. Errors are concise and actionable.

### Symbol Extraction
Language-specific symbol extraction in `src/parser.rs:extract_symbols_recursive()`. Each language has specific node types (e.g., Python: `function_definition`, `class_definition`).

### File Filtering

**Overview**: `IndexFilter` (`src/indexer.rs`) provides powerful glob-based pattern matching and language filtering for selective indexing.

**Features**:
- **Glob patterns**: Include/exclude files using standard glob syntax (`*`, `**`, `?`, `[...]`)
- **Language filtering**: Index only specific programming languages
- **Go-style paths**: Use `./...` syntax for recursive directory indexing
- **Validation**: Patterns and languages are validated before indexing

**Implementation** (`src/indexer.rs:IndexFilter`):
```rust
let filter = IndexFilter::new()
    .with_include_patterns(vec!["src/**/*.rs".to_string()])
    .with_exclude_patterns(vec!["**/target/**".to_string()])
    .with_languages(vec!["rust".to_string()]);

filter.validate()?;  // Validates patterns and language names

if filter.should_index_file(path) {
    // File passes all filters
}
```

**Pattern Precedence**:
1. Language filter (cheapest check, applied first)
2. Exclude patterns (take precedence over includes)
3. Include patterns (if specified, file must match at least one)

**Supported Languages**:
- `python`, `javascript`, `typescript`, `rust`, `go`, `c`, `cpp`, `java`

**Common Patterns**:
```bash
# Exclude build artifacts
--exclude "**/target/**,**/node_modules/**,**/dist/**"

# Include only source directories
--include "src/**,lib/**,tests/**"

# Language-specific indexing
--languages rust,python,go

# Monorepo filtering
cudgel index . --include "services/api/**" --languages typescript
```

**Performance**: Filtering is applied during file discovery, before parsing or embedding, minimizing wasted work.

### Embeddings
`src/embeddings.rs` uses ONNX Runtime with sentence-transformers/all-MiniLM-L6-v2:
- 384-dimensional vectors
- Mean pooling over token embeddings
- Cached tokenizer for efficiency

**Setup**:
```bash
uv venv .venv && source .venv/bin/activate
uv pip install 'optimum[onnxruntime]'
optimum-cli export onnx --model sentence-transformers/all-MiniLM-L6-v2 ./models/all-MiniLM-L6-v2
```

### Database Connection Pooling
Uses `deadpool-postgres` with automatic connection management. Pool configured in `src/database.rs:Database::new()`.

### CLI Structure
Main CLI in `src/main.rs` uses `clap` with derive macros. Each command has a dedicated `cmd_*` async function.

## Testing

```bash
cargo test                          # Run all tests
cargo test test_database_connection # Run specific test
cargo test -- --nocapture           # Show output
```

Tests require PostgreSQL running on port 54321. Run `task db-start` first.

## Important Notes

- **CLI-only design**: Rust CLI is the sole entrypoint
- **Native PostgreSQL**: Runs natively on port 54321 (not Docker)
- **XDG compliant**: Data stored in `~/.local/share/cudgel/postgres`
- **Auto-initialization**: Database schema initializes automatically
- **Incremental indexing**: Only re-processes changed files
- **Self-documenting code**: Minimal comments, clear naming
- **Service-oriented**: Clear separation of concerns
- All CLI output uses `colored` and `comfy-table` for formatting
- Release builds use aggressive optimizations (LTO, single codegen unit)

## Active Technologies
- Rust stable (edition 2021, MSRV 1.75+) (001-init-code-indexing-tool)
- PostgreSQL 14+ (local, port 54321) with pgvector extension for vector similarity search (001-init-code-indexing-tool)

## Implementation Status (Feature 001-init-code-indexing-tool)

### ✅ Completed (Functional MVP)

**User Story 1: Index and Query Codebase (P1) - COMPLETE**
- ✅ CLI commands: `cudgel index`, `cudgel query`, `cudgel graph`
- ✅ Multi-language parsing with tree-sitter (8 languages)
- ✅ ONNX-based embeddings (sentence-transformers/all-MiniLM-L6-v2)
- ✅ Incremental indexing with SHA256 hash-based change detection
- ✅ pgvector semantic search with HNSW indexing
- ✅ Table and JSON output formats
- ✅ Graph relationship traversal
- ✅ Advanced file filtering (glob patterns, include/exclude, language filtering)
- ✅ Go-style recursive path syntax (`./...`)
- ✅ Dry-run mode with filter preview

**Infrastructure**:
- ✅ PostgreSQL schema with auto-initialization
- ✅ Connection pooling (deadpool-postgres)
- ✅ Error handling with thiserror
- ✅ Taskfile.yml for common operations
- ✅ XDG-compliant data directories
- ✅ Comprehensive CLI with clap

**User Story 2: Schedule Automatic Re-indexing (P2) - COMPLETE**
- ✅ Database table `scheduled_tasks` created
- ✅ Database operations (create/delete/get scheduled tasks)
- ✅ CLI: `cudgel index --schedule hourly|daily|weekly|<hours> /path/to/repo`
- ✅ CLI: `cudgel index --unschedule /path/to/repo`
- ✅ CLI: `cudgel orchestrator start|stop|status|restart`
- ✅ Orchestrator daemon with polling loop (60s interval)
- ✅ Graceful shutdown (SIGTERM/SIGINT handling)
- ✅ PID file management (~/.local/state/cudgel/orchestrator.pid)
- ✅ Logging to ~/.local/state/cudgel/orchestrator.log
- ✅ Integration tests for orchestrator and scheduling (11 new tests)

### 🚧 In Progress / Planned

**User Story 3: Generate Knowledge Graph Documentation (P3) - TABLES READY**
- ✅ Database table `knowledge_documents` created
- ✅ ollama-rs dependency added to Cargo.toml
- ⏳ CLI: `cudgel knowledge` command
- ⏳ Ollama integration for LLM-powered documentation
- ⏳ $EDITOR integration for manual editing
- ⏳ Dependency extraction from manifest files
- ⏳ Architecture pattern detection
- ⏳ Build process documentation
- ⏳ License detection

**User Story 4: Export Query Results for LLM Consumption (P4)**
- ⏳ JSON minifier (LLM-OpenAPI-minifier format)
- ⏳ CLI flags: `--json`, `--json-pretty`, `--minified`
- ⏳ Key abbreviation (file_path → p, line_number → l, etc.)
- ⏳ Null/empty field omission

**Polish & Documentation**:
- ⏳ README.md with installation and quickstart
- ⏳ CONTRIBUTING.md with development guidelines
- ⏳ flake.nix for Nix-based installation
- ⏳ .pre-commit-config.yaml for pre-commit hooks
- ⏳ Shell completion scripts (bash, zsh, fish)
- ⏳ CHANGELOG.md for version tracking
- ⏳ LICENSE file (MIT)

### 📋 Next Steps for Implementation

**To complete User Story 3** (Knowledge Graph):
1. Implement `src/knowledge.rs` with Ollama client
2. Add `cudgel knowledge` subcommand in `src/main.rs`
3. Implement dependency extraction from Cargo.toml, package.json, etc.
4. Implement $EDITOR integration

**To complete User Story 4** (LLM Export):
1. Implement `src/minifier.rs` with token-efficient JSON format
2. Add `--minified` flag to `cudgel query` command
3. Update query output to support multiple formats

## Recent Changes
- 002-automatic-re-indexing: **Completed User Story 2** - Automatic re-indexing with orchestrator daemon
- 002-automatic-re-indexing: Added 11 comprehensive integration tests for scheduling and orchestrator
- 002-automatic-re-indexing: Removed all backwards compatibility code (Config::from_env())
- 002-automatic-re-indexing: XDG-strict configuration - no fallback paths
- 001-init-code-indexing-tool: **Added advanced file filtering** - glob patterns, include/exclude, language filtering
- 001-init-code-indexing-tool: **Go-style recursive path syntax** (`./...`) for multi-path indexing
- 001-init-code-indexing-tool: **IndexFilter** implementation with validation and pattern precedence
- 001-init-code-indexing-tool: Comprehensive test suite (16 new tests for filtering)
- 001-init-code-indexing-tool: Updated CLI to support multiple paths and filtering options
- 001-init-code-indexing-tool: **Enforced XDG Base Directory specification** - all paths check environment variables first
- 001-init-code-indexing-tool: Added scheduled_tasks and knowledge_documents tables for US2/US3
- 001-init-code-indexing-tool: Implemented User Story 1 (Index and Query) - COMPLETE
