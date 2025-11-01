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
- All data persists locally (PostgreSQL on port 54321)
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

### VI. Nix Flake as Primary Install Method

The canonical installation path MUST be via Nix flake:
- `flake.nix` provides reproducible development shell
- Pins exact versions of all system dependencies (Rust, PostgreSQL, ONNX Runtime)
- Alternative methods (cargo install, binary releases) are secondary
- CI/CD MUST validate flake builds

**Rationale**: Nix guarantees reproducible builds across machines, eliminating "works on my machine" issues. Flakes provide hermetic environments for both development and production.

### VII. Pre-commit Hooks and High Test Coverage

Quality gates MUST enforce correctness before code reaches main:
- Pre-commit hooks run: `cargo fmt`, `cargo clippy -- -D warnings`
- Unit tests MUST cover all business logic (target: >80% coverage)
- Integration tests MUST run in Docker with PostgreSQL fixtures
- Tests MUST be fast (unit: <1s total, integration: <10s total)
- CI MUST fail on formatting, linting, or test failures

**Rationale**: Automated quality checks catch regressions immediately. Fast tests enable TDD. Docker integration tests ensure database schema correctness.

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

- Config file format: TOML (human-readable, Rust ecosystem standard)
- Location: `~/.config/cudgel/config.toml`
- Sane defaults MUST allow zero-config operation
- Overrides: Environment variables > Config file > Hardcoded defaults
- Required defaults:
  - PostgreSQL: `localhost:54321`, user/db from `$USER`
  - Embedding model path: `~/.local/share/cudgel/models/all-MiniLM-L6-v2`
  - Log level: `info`

### Dependency Startup Checks

On every `cudgel` invocation, MUST verify:
- PostgreSQL running on configured port (fail fast with actionable error)
- Database schema initialized (auto-init if missing, versioned migrations)
- Embedding models present at expected path (download hint if missing)
- XDG directories exist and are writable

**Rationale**: Explicit dependency checks provide immediate, actionable feedback rather than cryptic downstream failures.

### Language and Toolchain

- **Rust Edition**: 2021 or later
- **MSRV (Minimum Supported Rust Version)**: 1.75+
- **Async Runtime**: Tokio (default features disabled, opt-in to needed features)
- **Database**: PostgreSQL 14+ (pgvector extension required)
- **Embeddings**: ONNX Runtime with sentence-transformers models

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

- **Unit tests**: In-module or `tests/` directory, no external dependencies
- **Integration tests**: Docker Compose fixture with PostgreSQL, full end-to-end
- **Contract tests**: Public API surface (CLI commands, service traits)
- **Test organization**:
  - `tests/unit/` - Pure logic, no I/O
  - `tests/integration/` - Full stack with real PostgreSQL
  - `tests/fixtures/` - Sample code repositories for indexing tests

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

**Version**: 1.0.0 | **Ratified**: 2025-10-31 | **Last Amended**: 2025-10-31
