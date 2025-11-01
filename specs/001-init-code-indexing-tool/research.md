# Research: Cudgel Code Intelligence System

**Date**: 2025-10-31
**Feature**: 001-init-code-indexing-tool

## Overview

This document consolidates research findings for implementing Cudgel, a local-first codebase intelligence system. Research focuses on library selection, best practices, integration patterns, and performance optimization for the Rust-based CLI tool.

## 1. Tree-sitter Integration for Multi-Language Parsing

### Decision: Use tree-sitter with per-language grammar crates

**Chosen Approach**:
- Main crate: `tree-sitter = "0.22"`
- Language grammars as separate dependencies:
  - `tree-sitter-python = "0.21"`
  - `tree-sitter-javascript = "0.21"`
  - `tree-sitter-typescript = "0.21"`
  - `tree-sitter-rust = "0.21"`
  - `tree-sitter-go = "0.21"`
  - `tree-sitter-c = "0.21"`
  - `tree-sitter-cpp = "0.22"`
  - `tree-sitter-java = "0.21"`

**Rationale**:
- Tree-sitter provides incremental, error-tolerant parsing with concrete syntax trees
- Each language grammar is maintained separately, allowing independent updates
- Rust bindings are zero-cost wrappers around C libraries
- Query DSL enables extracting specific node types (functions, classes) without manual traversal

**Implementation Pattern**:
```rust
// Parser trait for extensibility
pub trait LanguageParser: Send + Sync {
    fn language(&self) -> tree_sitter::Language;
    fn extract_symbols(&self, tree: &Tree, source: &str) -> Vec<Symbol>;
}

// Example: Python parser
pub struct PythonParser;
impl LanguageParser for PythonParser {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_python::language()
    }

    fn extract_symbols(&self, tree: &Tree, source: &str) -> Vec<Symbol> {
        // Use tree-sitter query:
        // (function_definition name: (identifier) @func.name)
        // (class_definition name: (identifier) @class.name)
    }
}
```

**Alternatives Considered**:
- **syn** (Rust-only): Rejected because limited to Rust; we need multi-language support
- **ANTLR4 Rust target**: Rejected due to slower parsing, higher memory overhead, complex grammar maintenance
- **libclang for C/C++**: Rejected to maintain consistency with single parser (tree-sitter handles C/C++)

**Best Practices**:
- Cache `Parser` instances per thread (tree-sitter is not Send/Sync)
- Use thread-local storage for parser reuse: `thread_local! { static PARSER: RefCell<Parser> = ... }`
- Extract only needed symbol types via queries, don't traverse entire AST
- Limit recursion depth for deeply nested code to prevent stack overflow

## 2. Ollama Integration for Embeddings and Knowledge Generation

### Decision: Use `ollama-rs` crate with llama3.2:8b model

**Chosen Approach**:
- Crate: `ollama-rs = "0.2"` (async Rust client for Ollama API)
- Model: `llama3.2:8b` (8 billion parameter model, good balance of quality and speed)
- API endpoint: `http://localhost:11434` (default Ollama server)

**Rationale**:
- Ollama provides local LLM inference, aligning with local-first principle
- llama3.2:8b offers strong code understanding without requiring massive hardware
- 8B model fits in 16GB RAM, runs on M1/M2 Macs and commodity GPUs
- `ollama-rs` is actively maintained, supports streaming responses, embeddings API

**Implementation Pattern**:
```rust
use ollama_rs::{Ollama, generation::embeddings::request::GenerateEmbeddingsRequest};

pub struct OllamaEmbeddingService {
    client: Ollama,
    model: String,
}

impl OllamaEmbeddingService {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Ollama::default(), // localhost:11434
            model: "llama3.2:8b".to_string(),
        })
    }

    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let request = GenerateEmbeddingsRequest::new(
            self.model.clone(),
            text.to_string().into(),
        );
        let response = self.client.generate_embeddings(request).await?;
        Ok(response.embeddings[0].clone())
    }
}
```

**Alternatives Considered**:
- **ONNX Runtime with sentence-transformers**: Rejected because requires managing ONNX models manually, less flexibility for knowledge generation (embedding-only)
- **OpenAI API**: Rejected - violates local-first principle, requires internet, costs money
- **llama.cpp bindings**: Considered but rejected - Ollama provides better API abstraction, model management

**Best Practices**:
- Check Ollama service availability on startup (`/api/tags` endpoint)
- Use connection pooling for concurrent embedding generation (tokio semaphore)
- Batch embedding requests when possible (Ollama supports batch embeddings)
- Cache embeddings for unchanged symbols (check SHA256 hash)
- Set timeouts for LLM calls (30s for embeddings, 120s for knowledge generation)

**Knowledge Generation Strategy**:
- Analyze indexed symbols to identify patterns:
  - Dependency analysis: Extract imports/requires from AST
  - Architecture detection: Identify MVC, microservices patterns from directory structure
  - Build process: Parse `Cargo.toml`, `package.json`, `Makefile`
  - Licensing: Extract license headers, detect SPDX identifiers
- Use few-shot prompting with llama3.2:8b to generate structured markdown
- Store generated content with metadata (generated timestamp, model version) for reproducibility

## 3. PostgreSQL with pgvector for Vector Similarity Search

### Decision: Use sqlx with pgvector extension for all persistence

**Chosen Approach**:
- Database client: `sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "macros"] }`
- Vector extension: `pgvector` installed via PostgreSQL extension system
- Vector type: `vector(384)` for embeddings (llama3.2 embedding dimension)
- Index: `HNSW` index for approximate nearest neighbor search

**Rationale**:
- sqlx provides compile-time query verification, preventing SQL injection and typos
- Async operations integrate cleanly with Tokio runtime
- pgvector extension adds vector similarity operators (<->, <#>, <=>)
- HNSW index offers best tradeoff for accuracy/speed (90%+ recall at 10x speedup vs exact search)
- Single database simplifies transactions (ACID for repos + files + symbols + embeddings)

**Schema Design**:
```sql
-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Repositories table
CREATE TABLE repositories (
    id SERIAL PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    last_indexed_at TIMESTAMPTZ,
    file_count INTEGER DEFAULT 0,
    symbol_count INTEGER DEFAULT 0,
    status TEXT DEFAULT 'pending'
);

-- Files table
CREATE TABLE files (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    language TEXT NOT NULL,
    last_parsed_at TIMESTAMPTZ,
    symbol_count INTEGER DEFAULT 0,
    UNIQUE(repo_id, path)
);

-- Symbols table
CREATE TABLE symbols (
    id SERIAL PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    symbol_type TEXT NOT NULL, -- function, class, method, variable
    line_number INTEGER NOT NULL,
    code_snippet TEXT,
    documentation TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Embeddings table (pgvector)
CREATE TABLE embeddings (
    id SERIAL PRIMARY KEY,
    symbol_id INTEGER NOT NULL UNIQUE REFERENCES symbols(id) ON DELETE CASCADE,
    vector vector(384) NOT NULL, -- llama3.2 embedding dimension
    generated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create HNSW index for fast similarity search
CREATE INDEX ON embeddings USING hnsw (vector vector_cosine_ops);

-- Scheduled tasks table
CREATE TABLE scheduled_tasks (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    interval_hours INTEGER NOT NULL,
    next_run_at TIMESTAMPTZ NOT NULL,
    last_run_at TIMESTAMPTZ,
    status TEXT DEFAULT 'active',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Knowledge documents table
CREATE TABLE knowledge_documents (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER NOT NULL UNIQUE REFERENCES repositories(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    generated_at TIMESTAMPTZ DEFAULT NOW(),
    last_edited_at TIMESTAMPTZ,
    version INTEGER DEFAULT 1
);
```

**Implementation Pattern**:
```rust
use sqlx::{PgPool, FromRow};
use pgvector::Vector;

#[derive(FromRow)]
pub struct Embedding {
    pub id: i32,
    pub symbol_id: i32,
    pub vector: Vector,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn search_similar(pool: &PgPool, query_vector: Vec<f32>, limit: i64) -> Result<Vec<Symbol>> {
    let query_vec = Vector::from(query_vector);
    sqlx::query_as!(
        Symbol,
        r#"
        SELECT s.id, s.name, s.symbol_type, s.line_number, s.code_snippet,
               e.vector <=> $1 as distance
        FROM symbols s
        JOIN embeddings e ON e.symbol_id = s.id
        ORDER BY e.vector <=> $1
        LIMIT $2
        "#,
        query_vec as Vector,
        limit
    )
    .fetch_all(pool)
    .await
}
```

**Alternatives Considered**:
- **diesel**: Rejected - synchronous only, no async support for high-throughput indexing
- **tokio-postgres**: Considered but rejected - sqlx provides compile-time checks and better ergonomics
- **Qdrant/Milvus**: Rejected - introduces second database, violates PostgreSQL exclusivity principle

**Best Practices**:
- Use connection pooling: `PgPool::connect_with(config)` with min/max connections
- Run migrations on startup: `sqlx::migrate!().run(&pool).await`
- Use prepared statements (sqlx does this automatically)
- Batch insert embeddings (PostgreSQL COPY or multi-row INSERT)
- Monitor index size: HNSW index grows with data, may need reindexing for large datasets
- Use `ANALYZE` after bulk inserts to update query planner statistics

## 4. Scheduling with PostgreSQL Polling

### Decision: Poll `scheduled_tasks` table every 60 seconds in background daemon

**Chosen Approach**:
- Daemon: Separate `cudgel orchestrator` command (not a system service)
- Polling interval: 60 seconds (configurable via `CUDGEL_POLL_INTERVAL_SECS`)
- Task execution: Spawn tokio tasks for each due job
- Locking: PostgreSQL row-level locks (`SELECT ... FOR UPDATE SKIP LOCKED`)

**Rationale**:
- Simple, reliable, requires no external job queue (Redis, RabbitMQ)
- PostgreSQL provides ACID guarantees for schedule updates
- Row-level locking prevents duplicate execution
- Polling is acceptable for hourly/daily schedules (60s latency negligible)

**Implementation Pattern**:
```rust
pub struct OrchestratorService {
    pool: PgPool,
    poll_interval: Duration,
}

impl OrchestratorService {
    pub async fn run(&self) -> Result<()> {
        let mut interval = tokio::time::interval(self.poll_interval);
        loop {
            interval.tick().await;
            if let Err(e) = self.process_due_tasks().await {
                tracing::error!("Failed to process tasks: {}", e);
            }
        }
    }

    async fn process_due_tasks(&self) -> Result<()> {
        let tasks = sqlx::query_as!(
            ScheduledTask,
            r#"
            SELECT * FROM scheduled_tasks
            WHERE next_run_at <= NOW() AND status = 'active'
            ORDER BY next_run_at
            FOR UPDATE SKIP LOCKED
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        for task in tasks {
            let pool = self.pool.clone();
            tokio::spawn(async move {
                if let Err(e) = execute_task(&pool, task).await {
                    tracing::error!("Task {} failed: {}", task.id, e);
                }
            });
        }

        Ok(())
    }
}
```

**Alternatives Considered**:
- **tokio-cron-scheduler**: Rejected - in-memory only, lost on restart, no persistence
- **celery-rs**: Rejected - requires Redis, too heavyweight for local-first tool
- **systemd timers**: Rejected - platform-specific (no macOS support), requires root for installation

**Best Practices**:
- Update `next_run_at` immediately after starting task (not after completion) to prevent drift
- Store orchestrator PID in `~/.local/state/cudgel/orchestrator.pid` for status checks
- Implement graceful shutdown: catch SIGTERM/SIGINT, wait for running tasks
- Log execution start/completion/errors to `~/.local/state/cudgel/orchestrator.log`
- Limit concurrent tasks with semaphore: `Arc<Semaphore>` with max permits

## 5. Configuration Management with config Crate

### Decision: Use `config` crate with layered TOML + environment variables

**Chosen Approach**:
- Crate: `config = "0.13"`
- Config file: `~/.config/cudgel/config.toml` (XDG_CONFIG_HOME)
- Override hierarchy: Environment variables > Config file > Defaults

**Rationale**:
- `config` crate supports multiple sources (files, env vars) with precedence
- TOML format is human-readable, Rust ecosystem standard
- Layering enables per-deployment customization without editing files

**Configuration Structure**:
```toml
# ~/.config/cudgel/config.toml

[database]
host = "localhost"
port = 54321
user = "cudgel"
password = ""
database = "cudgel"

[ollama]
url = "http://localhost:11434"
model = "llama3.2:8b"
timeout_secs = 120

[orchestrator]
poll_interval_secs = 60
max_concurrent_tasks = 5

[indexer]
max_file_size_bytes = 10485760  # 10MB
ignored_patterns = [".git", "node_modules", "target"]

[query]
default_limit = 50
max_limit = 1000

[logging]
level = "info"
```

**Implementation Pattern**:
```rust
use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub database: DatabaseConfig,
    pub ollama: OllamaConfig,
    pub orchestrator: OrchestratorConfig,
    pub indexer: IndexerConfig,
    pub query: QueryConfig,
    pub logging: LoggingConfig,
}

impl Settings {
    pub fn new() -> Result<Self> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix("cudgel")?;
        let config_path = xdg_dirs.place_config_file("config.toml")?;

        let settings = Config::builder()
            .add_source(config::File::from(config_path).required(false))
            .add_source(Environment::with_prefix("CUDGEL").separator("__"))
            .build()?;

        settings.try_deserialize()
    }
}
```

**Environment Variable Examples**:
- `CUDGEL_DATABASE__PORT=5432` → overrides `database.port`
- `CUDGEL_OLLAMA__URL=http://localhost:8080` → overrides `ollama.url`
- `CUDGEL_LOGGING__LEVEL=debug` → overrides `logging.level`

**Alternatives Considered**:
- **figment**: Similar feature set, less mature than `config`
- **Hardcoded values**: Rejected - inflexible, requires recompilation for changes
- **Command-line flags only**: Rejected - repetitive for common settings

**Best Practices**:
- Validate config on load (e.g., port in valid range 1-65535)
- Provide sane defaults for all settings
- Log final merged config on startup (debug level) for troubleshooting
- Create config directory if missing: `std::fs::create_dir_all(xdg_dirs.get_config_home())`

## 6. LLM-OpenAPI-Minifier Output Format

### Decision: Implement custom minifier logic for token-efficient output

**Chosen Approach**:
- Remove all unnecessary whitespace (compact JSON)
- Strip metadata fields not needed for LLM context (IDs, timestamps)
- Abbreviate keys: `file_path` → `path`, `symbol_name` → `name`, `code_snippet` → `code`
- Omit null/empty fields

**Rationale**:
- LLMs have token limits; reducing tokens allows more context
- Minified format reduces 40%+ tokens compared to pretty JSON (per spec SC-008)
- Preserves semantic information while optimizing for machine consumption

**Implementation Pattern**:
```rust
use serde_json::{json, Value};

pub struct Minifier;

impl Minifier {
    pub fn minify_query_results(results: Vec<Symbol>) -> String {
        let minified: Vec<Value> = results.iter().map(|s| {
            json!({
                "p": s.file_path,        // path
                "l": s.line_number,      // line
                "n": s.name,             // name
                "t": s.symbol_type,      // type
                "c": s.code_snippet,     // code
                // Omit: id, timestamp, documentation if empty
            })
        }).collect();

        serde_json::to_string(&minified).unwrap()
    }
}
```

**Example Output**:
```json
[{"p":"src/main.rs","l":42,"n":"main","t":"function","c":"fn main() { ... }"},{"p":"src/lib.rs","l":10,"n":"Config","t":"struct","c":"pub struct Config { ... }"}]
```

**Alternatives Considered**:
- **gzip compression**: Rejected - LLMs need readable text, not binary
- **Protobuf/MessagePack**: Rejected - not human-readable, requires deserialization
- **Standard JSON**: Works but wastes tokens on formatting

**Best Practices**:
- Document minified format in README for LLM integration
- Provide schema example for prompt engineering
- Add `--format` flag: `table` (default), `json`, `json-pretty`, `minified`

## 7. Testing Strategy

### Decision: Unit tests + Docker-based integration tests with PostgreSQL fixtures

**Chosen Approach**:
- Unit tests: In-module tests for pure logic (parsers, minifier, utils)
- Integration tests: `tests/integration/` with Docker Compose spinning up PostgreSQL + pgvector
- Test fixtures: Sample repositories in `tests/fixtures/sample-repos/`
- CI: GitHub Actions running both test suites

**Rationale**:
- Unit tests provide fast feedback (<1s), no external dependencies
- Integration tests verify database schema, migrations, end-to-end workflows
- Docker ensures consistent test environment across local dev and CI
- Fixtures enable realistic testing with actual code repositories

**Docker Compose Fixture**:
```yaml
# tests/fixtures/docker-compose.yml
version: '3.8'
services:
  postgres:
    image: pgvector/pgvector:pg16
    environment:
      POSTGRES_USER: cudgel_test
      POSTGRES_PASSWORD: test
      POSTGRES_DB: cudgel_test
    ports:
      - "54322:5432"
    volumes:
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql
```

**Test Organization**:
```rust
// tests/integration/index_test.rs
#[tokio::test]
async fn test_index_repository() {
    let pool = setup_test_db().await;
    let indexer = IndexService::new(pool.clone());

    let repo_path = PathBuf::from("tests/fixtures/sample-repos/rust-project");
    let result = indexer.index_repository(&repo_path).await;

    assert!(result.is_ok());
    let stats = result.unwrap();
    assert_eq!(stats.file_count, 5);
    assert_eq!(stats.symbol_count, 23);

    teardown_test_db(pool).await;
}
```

**Alternatives Considered**:
- **Mocked database**: Rejected - doesn't test real SQL, migrations, pgvector behavior
- **Shared test database**: Rejected - tests interfere with each other, cleanup complexity
- **Testcontainers**: Considered but Docker Compose is simpler for single-service case

**Best Practices**:
- Use separate test database (`cudgel_test`) on different port (54322)
- Run migrations in `setup_test_db()` helper
- Clean up data after each test: `TRUNCATE` or drop/recreate database
- Mark integration tests with `#[ignore]` by default, run explicitly in CI
- Use `cargo test --lib` for unit tests (fast), `cargo test --all` for full suite

## 8. Nix Flake Structure

### Decision: Provide flake.nix with package output, devShell, and PostgreSQL service

**Chosen Approach**:
- Package: `nix build` produces `cudgel` binary
- devShell: `nix develop` provides Rust, PostgreSQL, Ollama, sqlx-cli, tree-sitter CLI
- PostgreSQL service: NixOS module configuration for local postgres instance

**Flake Structure**:
```nix
{
  description = "Cudgel - Local-first codebase intelligence";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default;
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "cudgel";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.postgresql ];
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.postgresql_16
            pkgs.sqlx-cli
            pkgs.ollama
            pkgs.tree-sitter
            pkgs.cargo-watch
            pkgs.cargo-nextest
          ];

          shellHook = ''
            export PGDATA="$PWD/.pgdata"
            export CUDGEL_DATABASE__PORT=54321
          '';
        };
      }
    );
}
```

**Rationale**:
- Nix flake provides reproducible builds, pins exact dependency versions
- devShell includes all development tools (no manual installation)
- PostgreSQL service ensures correct version (16+) with pgvector support

**Alternatives Considered**:
- **Docker-based dev environment**: Rejected - less ergonomic for local development, Nix provides better shell integration
- **Manual installation docs**: Rejected - error-prone, version mismatches, violates constitution principle VI

**Best Practices**:
- Pin Rust toolchain to specific version: `pkgs.rust-bin.stable."1.75.0".default`
- Include `direnv` support: `.envrc` with `use flake`
- Document flake usage in README: `nix develop`, `nix build`, `nix run`

## Summary

All research confirms technical feasibility with mature, well-supported libraries. No blockers identified. Key findings:

1. **Tree-sitter** provides robust multi-language parsing with extensible trait design
2. **Ollama + llama3.2:8b** enables local LLM inference for embeddings and knowledge generation
3. **PostgreSQL + pgvector** consolidates all persistence with HNSW-indexed vector search
4. **sqlx** offers compile-time safety for async database operations
5. **Polling-based scheduling** is simple, reliable, requires no external dependencies
6. **config crate** provides layered configuration matching 12-factor principles
7. **Custom minifier** optimizes LLM token usage while preserving semantics
8. **Docker + Nix** enable reproducible testing and development environments

All decisions align with Cudgel constitution principles (local-first, XDG compliance, PostgreSQL exclusivity, Rust idioms, Nix flake, testing discipline).

**Next Steps**: Proceed to Phase 1 (data model, contracts, quickstart).
