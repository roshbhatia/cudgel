# Implementation Plan: Cudgel Code Intelligence System

**Branch**: `001-init-code-indexing-tool` | **Date**: 2025-10-31 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-init-code-indexing-tool/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Build a local-first codebase intelligence system enabling developers to index git repositories, perform semantic code searches, schedule automatic re-indexing via background daemon, and generate AI-powered knowledge graphs. Uses tree-sitter for AST parsing, Ollama for embeddings and knowledge generation, PostgreSQL with pgvector for storage, all orchestrated through Rust CLI tools following service-oriented architecture with trait-based extensibility.

## Technical Context

**Language/Version**: Rust stable (edition 2021, MSRV 1.75+)
**Primary Dependencies**:
- CLI: `clap` (derive macros), `dialoguer` (interactive prompts)
- Async: `tokio` (full features for runtime, fs, process)
- Database: `sqlx` (async PostgreSQL, compile-time query checking), `pgvector` support
- Parsing: `tree-sitter` + per-language grammars (python, javascript, typescript, rust, go, c, cpp, java)
- Embeddings/LLM: `ollama-rs` (Rust client for Ollama API)
- Config: `config` crate (layered TOML + env vars)
- Logging: `tracing` + `tracing-subscriber`
- Error: `thiserror`, `anyhow`
- Utilities: `walkdir`, `sha2`, `serde`, `serde_json`, `chrono`

**Storage**: PostgreSQL 14+ (local, port 54321) with pgvector extension for vector similarity search
**Testing**: `cargo test` (unit + integration), Docker Compose for PostgreSQL fixtures
**Target Platform**: macOS, Linux (Unix-like systems with XDG support)
**Project Type**: Single Rust project (CLI tool + background daemon)
**Performance Goals**:
- Index throughput: >1000 files/second
- Query latency: <1 second for 100k symbols
- Startup time: <500ms
- Memory: <500MB RSS during indexing

**Constraints**:
- Local-first: All processing localhost-only, no external API calls
- XDG compliant: Data in `~/.local/share/cudgel/`, config in `~/.config/cudgel/`, state in `~/.local/state/cudgel/`
- Offline-capable: Works without internet after initial setup
- PostgreSQL exclusive: Single database for all persistence

**Scale/Scope**:
- Repositories: Up to 100 concurrent scheduled repos
- Files per repo: 10,000+ files supported
- Symbols: 100,000+ indexed symbols per repo
- Concurrent queries: Single-user focused (1-2 concurrent operations)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### I. Rust Idioms First ✅
- **Status**: PASS
- **Application**: Using Rust stable with idiomatic patterns (Result/Option types, trait composition, async/await with Tokio, thiserror for errors)
- **Verification**: Service traits for Parser, Indexer, QueryEngine; owned types with explicit lifetime annotations where needed

### II. Local-First Architecture ✅
- **Status**: PASS
- **Application**: All data persists locally in PostgreSQL on port 54321, Ollama runs locally, tree-sitter parsing is local, no external API calls
- **Verification**: Network operations limited to localhost:54321 (PostgreSQL) and localhost:11434 (Ollama)

### III. XDG Base Directory Compliance ✅
- **Status**: PASS
- **Application**:
  - Data: `~/.local/share/cudgel/postgres/` (database), `~/.local/share/cudgel/models/` (embeddings)
  - Config: `~/.config/cudgel/config.toml`
  - Cache: `~/.cache/cudgel/` (temporary build artifacts)
  - State: `~/.local/state/cudgel/orchestrator.log` (daemon logs, PIDs)
- **Verification**: `config` crate configured with XDG paths, all file operations use XDG directories

### IV. PostgreSQL as Single Source of Truth ✅
- **Status**: PASS
- **Application**: All data in PostgreSQL:
  - Repositories, files, symbols → relational tables
  - Embeddings → pgvector extension
  - Schedules → `scheduled_tasks` table
  - Knowledge graphs → `knowledge_documents` table
  - No file-based persistence except XDG cache
- **Verification**: sqlx for all database operations, schema includes all entities

### V. Self-Documenting Code ✅
- **Status**: PASS
- **Application**: Clear naming (IndexService, QueryEngine, KnowledgeGenerator), minimal comments except doc comments for public API and tree-sitter AST logic
- **Verification**: Code review checklist enforces comment policy

### VI. Nix Flake as Primary Install Method ✅
- **Status**: PASS
- **Application**: `flake.nix` provides:
  - Package output for `cudgel` binary
  - devShell with Rust, PostgreSQL, Ollama, tree-sitter
  - PostgreSQL NixOS module for local service
- **Verification**: CI validates flake build, README lists `nix build` as primary install

### VII. Pre-commit Hooks and High Test Coverage ✅
- **Status**: PASS
- **Application**:
  - Pre-commit: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --lib`
  - Unit tests: In-module tests for all services (target >80% coverage)
  - Integration tests: Docker Compose with PostgreSQL fixtures
  - Fast tests: Unit <1s, integration <10s
- **Verification**: `.pre-commit-config.yaml` configured, CI enforces all checks

### VIII. 12-Factor Principles (Where Applicable) ✅
- **Status**: PASS (selective adoption)
- **Application**:
  - Config: Environment variables via `config` crate, defaults in `config.toml`
  - Dependencies: `Cargo.toml` and `flake.nix`
  - Build/Release/Run: Cargo profiles (dev/release), Nix outputs
  - Logs: `tracing` structured logging to stdout
  - Disposability: Fast startup (<500ms), graceful PostgreSQL shutdown
- **Not Applicable**: Backing services (local only), port binding (CLI), concurrency (single-process async), dev/prod parity (identical)
- **Verification**: Startup checks verify dependencies, logging goes to stdout only

### IX. Service-Oriented, Trait-Extensible Design ✅
- **Status**: PASS
- **Application**:
  - Services: IndexService, QueryEngine, OrchestratorService, KnowledgeGenerator as independent modules
  - Traits: `Parser` (language-specific), `EmbeddingGenerator`, `ScheduleManager`
  - Dependency injection: Trait objects via Arc<dyn Trait> or generics
  - Extension: New languages via `Parser` trait impl, new embedding models via `EmbeddingGenerator` trait
- **Verification**: Module structure in `src/services/`, trait definitions in `src/traits/`

### Gate Evaluation

**Overall Status**: ✅ **PASS** - All constitution principles satisfied

**Justifications**: None required - design adheres to all principles

**Proceed to Phase 0**: Research can begin

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
cudgel/
├── src/
│   ├── main.rs                 # CLI entry point (clap command dispatch)
│   ├── lib.rs                  # Public library interface
│   ├── config.rs               # Configuration (config crate + XDG)
│   ├── error.rs                # Error types (thiserror)
│   │
│   ├── cli/                    # CLI command handlers
│   │   ├── mod.rs
│   │   ├── index.rs            # cudgel index command
│   │   ├── query.rs            # cudgel query command
│   │   ├── knowledge.rs        # cudgel knowledge command
│   │   └── orchestrator.rs     # cudgel orchestrator command
│   │
│   ├── services/               # Core business logic (service-oriented)
│   │   ├── mod.rs
│   │   ├── indexer.rs          # IndexService (orchestrates indexing workflow)
│   │   ├── parser.rs           # ParserService (tree-sitter AST extraction)
│   │   ├── embeddings.rs       # EmbeddingService (Ollama client)
│   │   ├── query.rs            # QueryEngine (pgvector search)
│   │   ├── orchestrator.rs     # OrchestratorService (scheduling daemon)
│   │   └── knowledge.rs        # KnowledgeGenerator (Ollama-powered docs)
│   │
│   ├── db/                     # Database layer
│   │   ├── mod.rs
│   │   ├── schema.rs           # sqlx migrations, table definitions
│   │   ├── repos.rs            # Repository CRUD
│   │   ├── files.rs            # File CRUD
│   │   ├── symbols.rs          # Symbol CRUD + pgvector ops
│   │   ├── schedules.rs        # Schedule CRUD
│   │   └── knowledge.rs        # KnowledgeGraph CRUD
│   │
│   ├── traits/                 # Extension traits for modularity
│   │   ├── mod.rs
│   │   ├── parser.rs           # Parser trait (language-specific impls)
│   │   ├── embeddings.rs       # EmbeddingGenerator trait
│   │   └── scheduler.rs        # ScheduleManager trait
│   │
│   └── utils/                  # Utilities
│       ├── mod.rs
│       ├── xdg.rs              # XDG path helpers
│       ├── git.rs              # Git operations (git2 crate)
│       ├── hash.rs             # SHA256 file hashing
│       └── minifier.rs         # LLM-OpenAPI-minifier logic
│
├── tests/
│   ├── unit/                   # Unit tests (no I/O)
│   │   ├── parser_test.rs
│   │   ├── embeddings_test.rs
│   │   └── minifier_test.rs
│   │
│   ├── integration/            # Integration tests (PostgreSQL fixtures)
│   │   ├── index_test.rs
│   │   ├── query_test.rs
│   │   └── orchestrator_test.rs
│   │
│   └── fixtures/               # Test data
│       ├── sample-repos/
│       └── docker-compose.yml  # PostgreSQL + pgvector test fixture
│
├── migrations/                 # sqlx database migrations
│   ├── 001_init.sql
│   ├── 002_pgvector.sql
│   └── 003_schedules.sql
│
├── flake.nix                   # Nix flake (package + devShell + postgres)
├── Cargo.toml                  # Dependencies
├── Cargo.lock
├── .pre-commit-config.yaml     # Pre-commit hooks
├── README.md
└── CLAUDE.md                   # AI development guide
```

**Structure Decision**: Single Rust project (Option 1) as this is a CLI tool, not a web/mobile app. Service-oriented design with clear separation:
- `cli/` handles user interaction and delegates to services
- `services/` contains core business logic, each service independent
- `db/` abstracts database operations via sqlx
- `traits/` provides extension points for new languages, embedding models, etc.
- `tests/` organized by type (unit/integration) with Docker fixtures for E2E tests

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No violations - table not needed.

---

## Phase Completion Summary

### Phase 0: Research ✅ COMPLETE

**Artifacts Generated**:
- `research.md` - Comprehensive research document covering:
  - Tree-sitter integration patterns for multi-language parsing
  - Ollama integration with llama3.2:8b for embeddings and knowledge generation
  - PostgreSQL + pgvector schema design and best practices
  - Scheduling implementation via polling
  - Configuration management with `config` crate
  - LLM-OpenAPI-minifier output format specification
  - Testing strategy (unit + Docker-based integration)
  - Nix flake structure for reproducible builds

**Key Decisions**:
- ✅ All technical unknowns resolved
- ✅ Library selections validated (tree-sitter, ollama-rs, sqlx, config)
- ✅ Implementation patterns documented with code examples
- ✅ Alternatives evaluated and rationale provided
- ✅ No blockers identified

### Phase 1: Design & Contracts ✅ COMPLETE

**Artifacts Generated**:
- `data-model.md` - Complete database schema with:
  - 6 entities (Repository, File, Symbol, Embedding, ScheduledTask, KnowledgeDocument)
  - ERD diagram showing relationships
  - SQL schema with constraints, indexes, and validation rules
  - Migration strategy using sqlx
  - Data lifecycle documentation

- `contracts/cli-interface.md` - CLI contract specification:
  - 4 main commands: `index`, `query`, `knowledge`, `orchestrator`
  - Detailed usage, options, examples, exit codes
  - Output format specifications (table, JSON, minified)
  - Environment variable overrides
  - Shell completion support

- `quickstart.md` - User onboarding guide:
  - Installation instructions (Nix + Cargo)
  - Initial setup (PostgreSQL, Ollama)
  - 5-minute quick start tour
  - Common use cases (5 scenarios)
  - Configuration and troubleshooting
  - Performance tips

**Agent Context Updated**:
- ✅ CLAUDE.md updated with technology stack from plan.md
- ✅ Preserved manual additions between markers
- ✅ Added Rust stable, PostgreSQL 14+ references

### Phase 2: Post-Design Constitution Re-check ✅ PASS

**Re-evaluated Constitution Compliance**:
- ✅ I. Rust Idioms First - Service traits, owned types, async/await patterns documented
- ✅ II. Local-First Architecture - PostgreSQL + Ollama localhost-only confirmed
- ✅ III. XDG Compliance - All directories mapped in data model and config
- ✅ IV. PostgreSQL Exclusivity - Single database, all entities in schema, no file persistence
- ✅ V. Self-Documenting Code - Clear naming conventions, minimal comment policy
- ✅ VI. Nix Flake Primary - flake.nix structure researched, package + devShell + postgres
- ✅ VII. Pre-commit + Tests - Testing strategy defined (unit, Docker integration)
- ✅ VIII. 12-Factor (Applicable) - Config layering, logging, disposability documented
- ✅ IX. Service-Oriented Traits - Service modules and trait extensibility in structure

**No Violations** - Design fully compliant with constitution

### Next Steps

**Ready for Implementation**:
1. Run `/speckit.tasks` to generate task breakdown from design artifacts
2. Implement in order:
   - Phase 1 (P1): Index and Query Codebase (MVP)
   - Phase 2 (P2): Schedule Automatic Re-indexing
   - Phase 3 (P3): Knowledge Graph Generation
   - Phase 4 (P4): LLM Export Formats

**Artifacts Ready for Task Generation**:
- ✅ spec.md (4 prioritized user stories)
- ✅ plan.md (technical context, structure)
- ✅ research.md (implementation patterns)
- ✅ data-model.md (6 entities, schema)
- ✅ contracts/cli-interface.md (CLI specification)
- ✅ quickstart.md (user documentation)

**Phase 0 + Phase 1 Complete** - Planning finished successfully!
