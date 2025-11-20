# Agent Development Guide

## Build & Test Commands
- **Build**: `cargo build --release` or `task build`
- **Test all**: `cargo test` or `task test`
- **Test single**: `cargo test test_name` (e.g., `cargo test test_parser_parse_python`)
- **Test lib only**: `cargo test --lib`
- **Test integration**: `cargo test --test integration_tests`
- **Lint**: `cargo clippy --all-targets -- -D warnings` or `task clippy` (zero warnings policy)
- **Format**: `cargo fmt` or `task fmt`
- **Check**: `cargo check` or `task check`
- **Setup**: `task setup` (builds, installs, starts DB, initializes schema)

## Code Style
- **Imports**: Group std → external crates → internal modules (`use crate::{...}`); use explicit paths
- **Error handling**: Use `thiserror` for domain errors (`Error` enum), propagate with `?`; convert to user-friendly messages via `Error::to_user_message()` with troubleshooting steps
- **Types**: Explicit types preferred; `Option<T>` and `Result<T, Error>`; leverage type aliases (`pub type Result<T> = std::result::Result<T, Error>`)
- **Naming**: snake_case (functions/vars), PascalCase (types), SCREAMING_SNAKE_CASE (constants)
- **Async**: `tokio::main` for entry, `Arc<Database>` for shared state, `async fn` for I/O
- **Module docs**: Doc comments (`///`) for public APIs with runnable examples; `//!` module-level docs explain purpose/usage
- **Testing**: Prefix tests `test_`, use `setup_test_db()` helper, return `Option<Arc<Database>>`, skip gracefully when PostgreSQL unavailable
- **Validation**: Early input validation with specific ranges (e.g., 1-1000 for limits), actionable error messages with remediation steps
- **Database**: PostgreSQL-only persistence (port 54321), pgvector for embeddings, use `Arc<Database>` for shared access

## Active Technologies
- Rust 2021 edition (cargo 1.75+) + tokio (async runtime), chrono (time handling), tracing (logging), postgres (database), existing cudgel modules (Indexer, Database) (002-automatic-re-indexing)
- PostgreSQL 15+ with existing `scheduled_tasks` table (port 54321) (002-automatic-re-indexing)
- Rust 2021 edition (cargo 1.75+) + clap (CLI), tokio (async), postgres + pgvector, ort (ONNX), optimum-cli (Python for model export), uv (Python package manager) (004-auto-deps-management)
- PostgreSQL 15+ (port 45678), XDG-compliant directories (~/.local/share/cudgel, ~/.local/state/cudgel) (004-auto-deps-management)
- Rust 2021 edition (cargo 1.75+) + `tokenizers` (0.19), `ort` (2.0.0-rc.10 - ONNX Runtime), existing tree-sitter parsers (001-fallback-tokenization)
- PostgreSQL 15+ with pgvector extension (port 45678) (001-fallback-tokenization)

## Recent Changes
- 002-automatic-re-indexing: Added Rust 2021 edition (cargo 1.75+) + tokio (async runtime), chrono (time handling), tracing (logging), postgres (database), existing cudgel modules (Indexer, Database)
