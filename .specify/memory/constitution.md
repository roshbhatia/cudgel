# Cudgel Constitution

<!--
==================== SYNC IMPACT REPORT ====================
Version Change: N/A → 1.0.0 (Initial Constitution)

Modified Principles:
- Initial creation - all principles are new

Added Sections:
- Core Principles (9 principles)
- Technical Constraints
- Development Workflow
- Governance

Removed Sections:
- None (initial version)

Templates Requiring Updates:
- ✅ plan-template.md - Constitution Check section validated
- ✅ spec-template.md - Requirements alignment validated
- ✅ tasks-template.md - Task categorization validated
- ✅ agent-file-template.md - Development guidelines validated
- ✅ checklist-template.md - Quality gates validated

Follow-up TODOs:
- None - all placeholders filled

Version Bump Rationale:
- 1.0.0: Initial ratification of Cudgel project constitution
============================================================
-->

## Core Principles

### I. Rust Idioms First

All code MUST follow Rust best practices and idiomatic patterns:
- Leverage the type system for correctness (Result<T, E>, Option<T>)
- Use trait composition over inheritance
- Prefer owned types and explicit lifetimes when borrowing is necessary
- Zero-cost abstractions via generics and monomorphization
- Error handling via `thiserror` for library code, propagating with `?`
- Async everywhere via Tokio runtime (no blocking operations in async contexts)

**Rationale**: Rust's compiler guarantees and ecosystem patterns prevent entire classes of bugs at compile time. Following idiomatic patterns ensures maintainability and correctness.

### II. Local-First Architecture

Cudgel MUST operate entirely on local infrastructure with zero external service dependencies:
- All data persists locally (PostgreSQL on port 45678)
- All processing happens locally (tree-sitter parsing, ONNX embeddings)
- No cloud services, APIs, or telemetry
- Network operations limited to localhost only
- Deterministic behavior independent of network availability

**Rationale**: Users retain complete control over their code and data. Local-first ensures privacy, predictability, and reliability without external failure modes.

### III. XDG Base Directory Compliance

All persistent data MUST follow XDG Base Directory specification:
- Data: `~/.local/share/cudgel/` (PostgreSQL database, indexes, models)
- Config: `~/.config/cudgel/` (configuration files, user preferences)
- Cache: `~/.cache/cudgel/` (temporary embeddings, build artifacts)
- State: `~/.local/state/cudgel/` (logs, runtime state, PIDs)

**Rationale**: Adhering to XDG standards ensures proper integration with Unix-like systems, predictable file locations, and respect for user filesystem organization.

### IV. PostgreSQL as Single Source of Truth

PostgreSQL MUST be the exclusive persistence layer for ALL data:
- Repositories, files, symbols → relational tables
- Embeddings → pgvector extension
- Code relationships (call graphs, references) → graph stored in tables
- Task scheduling metadata → PostgreSQL rows
- No secondary databases, no file-based persistence (except XDG-compliant caching)

**Rationale**: Single database simplifies operations, enables ACID transactions across all data types, leverages PostgreSQL's mature ecosystem, and eliminates synchronization complexity.

### V. Self-Documenting Code

Code MUST be self-explanatory through clear naming and structure; comments are permitted ONLY when:
- Explaining non-obvious algorithmic choices
- Documenting public API surface (doc comments)
- Clarifying complex tree-sitter AST traversal logic
- Noting workarounds for upstream bugs (with issue links)

Comments MUST NOT:
- Restate what the code obviously does
- Serve as version history (use git)
- Contain TODOs or FIXMEs (use issue tracker)

**Rationale**: Clear code is easier to maintain than commented code. The compiler enforces correctness; humans should read intent from structure and names, not English prose.

### VI. Nix Shell as Primary Development Environment

The canonical development environment MUST be via Nix shell:
- `shell.nix` provides reproducible development shell with instant loading
- Pins exact versions of all system dependencies (Rust, PostgreSQL 17+, ONNX Runtime, uv, ollama)
- shellHook MUST NOT block on slow operations (downloads moved to explicit commands)
- Alternative methods (cargo install, binary releases) are secondary
- Development tools accessible via `task` commands (task health, task install-models, etc.)
- CI/CD MUST validate builds work in Nix environment

**Rationale**: Nix guarantees reproducible builds across machines, eliminating "works on my machine" issues. Instant shell loading ensures developer productivity.

### VII. Pre-commit Hooks and High Test Coverage

Quality gates MUST enforce correctness before code reaches main:
- Pre-commit hooks run: `cargo fmt`, `cargo clippy -- -D warnings`
- Unit tests MUST cover all business logic (current: 48 unit tests covering config, database, orchestrator, parser)
- Integration tests MUST run with native PostgreSQL on port 45678 (current: 42 integration tests)
- Tests MUST be fast (unit: <1s total, integration: <15s total)
- CI MUST fail on formatting, linting, or test failures
- Current coverage: 90 tests total (48 unit + 42 integration)

**Rationale**: Automated quality checks catch regressions immediately. Fast tests enable TDD. Native PostgreSQL integration tests ensure database schema correctness.

### VIII. 12-Factor Principles (Where Applicable)

Apply 12-factor methodology where it aligns with local-first constraints:
- **Config**: Environment variables with sane defaults (PostgreSQL port, log level)
- **Dependencies**: Explicitly declared in Cargo.toml and flake.nix
- **Build/Release/Run**: Strict separation via cargo profiles and Nix outputs
- **Logs**: Structured logging to stdout via `tracing` (never files directly)
- **Disposability**: Fast startup, graceful shutdown (clean PostgreSQL connections)

12-factor principles NOT applicable to Cudgel:
- **Backing Services**: PostgreSQL is local, not "attached" via URL
- **Port Binding**: CLI tool, not a web service
- **Concurrency**: Single-process async runtime, not horizontal scaling
- **Dev/Prod Parity**: Identical (local-first means dev IS prod)

**Rationale**: 12-factor guidelines promote clean separation of concerns and operational simplicity. We adopt what fits our CLI/local-first model and skip what doesn't.

### IX. Service-Oriented, Trait-Extensible Design

Architecture MUST follow service-oriented patterns with trait-based extension points:
- Core services (Indexer, QueryEngine, GraphQuery) as independent modules
- Service traits define contracts (e.g., `Parser` trait for language support)
- Dependency injection via trait objects or generics
- Services communicate via async message passing or direct trait method calls
- New features extend via implementing traits, not modifying core services

**Rationale**: Service-oriented design enables independent testing, parallel development, and feature extension without modifying stable code. Traits provide compile-time polymorphism for zero-cost abstractions.

## Technical Constraints

### Configuration Management

- Config profiles via code: `Config::local()`, `Config::test()`, `Config::ci()`
- Configuration constants defined in `DatabaseConfig` (single source of truth)
- Environment variables MUST use `CUDGEL_*` prefix for isolation from system PostgreSQL
- Validation: `Config::local()` returns `Result<Config, Error>` (fail-fast on invalid config)
- Overrides: CUDGEL_* env vars > Hardcoded defaults (no config files)
- Required defaults (via `DatabaseConfig` constants):
  - PostgreSQL: `localhost:45678` (DEFAULT_PORT), user from `$USER`, database `cudgel` (DEFAULT_DATABASE)
  - Embedding model path: `~/.local/share/cudgel/models/all-MiniLM-L6-v2`
  - Ollama: `localhost:11434`, model `llama3.2:8b` (for knowledge generation)
  - Log level: `info`
- Environment variables:
  - `CUDGEL_POSTGRES_HOST` - Database host (default: localhost)
  - `CUDGEL_POSTGRES_PORT` - Database port (default: 45678)
  - `CUDGEL_POSTGRES_DATABASE` - Database name (default: cudgel)
  - `CUDGEL_POSTGRES_USER` - Database user (default: $USER)
  - `CUDGEL_POSTGRES_PASSWORD` - Database password (default: cudgel)

### Dependency Startup Checks

On every `cudgel` invocation, MUST verify:
- PostgreSQL running on configured port (fail fast with actionable error)
- Database schema initialized (auto-init if missing, versioned migrations)
- Configuration validation (Config::local() validates on construction)
- Embedding models present at expected path (download hint if missing, use `task install-models`)
- Ollama service available for knowledge generation (fail gracefully for knowledge command only)
- XDG directories exist and are writable

Development environment health check available via: `task health`

**Rationale**: Explicit dependency checks provide immediate, actionable feedback rather than cryptic downstream failures. Fail-fast validation prevents runtime errors.

### Language and Toolchain

- **Rust Edition**: 2021 or later
- **MSRV (Minimum Supported Rust Version)**: 1.75+
- **Async Runtime**: Tokio (default features disabled, opt-in to needed features)
- **Database**: PostgreSQL 14+ (pgvector extension required)
- **Embeddings**: ONNX Runtime with sentence-transformers models
- **Knowledge Generation**: Ollama with llama3.2:8b model

### Performance and Scale Targets

- Index throughput: >1000 files/second on commodity hardware
- Query latency: <100ms p95 for semantic search (10k indexed symbols)
- Memory footprint: <500MB RSS during indexing
- Startup time: <500ms (excluding PostgreSQL connection establishment)

## Development Workflow

### Git Hooks (Pre-commit)

Mandatory checks before commit:
1. `cargo fmt --check` (formatting)
2. `cargo clippy -- -D warnings` (linting, warnings as errors)
3. `cargo test --lib` (unit tests only, fast feedback)

On push to remote:
4. `cargo test --all` (full test suite including integration tests)

### Testing Discipline

- **Unit tests**: In-module `#[cfg(test)]` blocks, no external dependencies (48 tests)
  - `config.rs`: 23 tests (validation, XDG paths, config profiles)
  - `database.rs`: 3 tests (connection strings, scheduled tasks)
  - `orchestrator.rs`: 6 tests (PID management, daemon lifecycle)
  - `parser.rs`: 18 tests (language detection, hashing, tree-sitter)
- **Integration tests**: Native PostgreSQL on port 45678, full end-to-end (42 tests)
- **Contract tests**: Public API surface (CLI commands, service traits)
- **Test organization**:
  - Unit tests: In-module via `#[cfg(test)]`
  - Integration tests: `tests/integration_tests.rs`
  - Fixtures: Sample code in integration test setup

### Code Review Requirements

All changes MUST:
- Pass CI (fmt, clippy, tests)
- Include tests for new functionality
- Update relevant documentation (README, CLAUDE.md, docstrings)
- Demonstrate adherence to constitution principles

Complexity additions (new services, architecture changes) MUST:
- Justify deviation from simplicity in commit message or PR description
- Propose simpler alternatives and explain rejection reasoning

### Release Process

1. Update `Cargo.toml` version (semantic versioning)
2. Update `CHANGELOG.md` with user-facing changes
3. Tag release: `git tag v<version>`
4. CI builds and publishes:
   - Nix flake output
   - Cargo crate to crates.io
   - Binary releases for Linux/macOS (GitHub Releases)

## Governance

### Constitution Authority

This constitution is the HIGHEST authority on design and implementation decisions. When in conflict:
- Constitution > Team preferences
- Constitution > Convenience
- Constitution > External "best practices" (unless adopted into constitution)

### Amendment Process

To amend this constitution:
1. Propose change via issue or pull request
2. Document rationale and impact on existing principles
3. Update all dependent templates (plan, spec, tasks, agent-file)
4. Increment `CONSTITUTION_VERSION` per semantic versioning
5. Update `LAST_AMENDED_DATE`
6. Obtain approval from project maintainers

### Version Semantics

- **MAJOR (X.0.0)**: Backward-incompatible principle removal or redefinition
- **MINOR (0.X.0)**: New principle added or existing principle materially expanded
- **PATCH (0.0.X)**: Clarifications, wording fixes, typo corrections

### Compliance Review

Every feature specification MUST include a "Constitution Check" section verifying:
- Which principles apply to this feature
- How the design upholds each applicable principle
- Justification for any principle violations (with approval requirement)

Every pull request MUST:
- Pass pre-commit hook validation (enforces coding standards)
- Demonstrate test coverage (enforces testing discipline)
- Reference applicable constitution principles in description

### Principle Enforcement

Violations of NON-NEGOTIABLE principles (Rust idioms, local-first, PostgreSQL exclusivity, pre-commit hooks) MUST be rejected unless:
- Upstream bug workaround (temporary, with removal plan)
- Explicit constitution amendment approved

Violations of SHOULD principles (12-factor applicability) MAY be accepted with justification documented in commit message.

### Living Document

This constitution is a living document. As Cudgel evolves:
- New principles MAY be added via MINOR version bump
- Existing principles MAY be clarified via PATCH version bump
- Principles MUST NOT be removed without MAJOR version bump and migration guide

---

**Version**: 1.1.0 | **Ratified**: 2025-10-31 | **Last Amended**: 2025-11-02

## Changelog

### 1.1.0 (2025-11-02) - Configuration & DevOps Improvements

**Modified Principles:**
- II. Local-First Architecture: Updated PostgreSQL port 54321 → 45678
- VI. Nix as Primary Method: Updated to focus on shell.nix with instant loading
- VII. Pre-commit Hooks: Updated test counts (90 total: 48 unit + 42 integration)

**Technical Constraints Updates:**
- Configuration Management: Added CUDGEL_* environment variables, Config profiles, DatabaseConfig constants
- Dependency Startup Checks: Added Config validation, task health command
- Testing Discipline: Updated with actual test counts and organization

**Rationale:**
- Port 45678 is a better default (less likely to conflict than 54321)
- CUDGEL_* prefix isolates configuration from system PostgreSQL
- Config::local() returning Result enables fail-fast validation
- Configuration constants provide single source of truth
- shell.nix improvements: instant loading, explicit model installation via task command
