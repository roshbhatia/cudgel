<!--
Sync Impact Report - Constitution Update
==========================================
Version Change: Template → 1.0.0
Modified Principles: 
  - Added: I. Local-First Architecture
  - Added: II. Test-Driven Development (NON-NEGOTIABLE)
  - Added: III. Performance & Efficiency
  - Added: IV. Semantic Intelligence
  - Added: V. Incremental Processing
Added Sections:
  - Technology Stack Requirements
  - Development Workflow
  - Governance
Removed Sections: None
Templates Requiring Updates:
  ✅ plan-template.md - Constitution Check section aligns with principles
  ✅ spec-template.md - User story structure supports independent testing (aligns with TDD)
  ✅ tasks-template.md - Task organization by user story supports incremental delivery
  ⚠️  AGENTS.md - References TDD but could strengthen alignment with constitution principles
Follow-up TODOs:
  - Ratification date set to 2025-11-19 (constitution creation date)
  - Consider adding explicit observability/logging principle in future amendment
  - Consider adding explicit CLI-first principle in future amendment
-->

# Cudgel Constitution

## Core Principles

### I. Local-First Architecture

All functionality MUST run locally without external service dependencies (except optional Ollama for knowledge graph features). Data MUST be stored in local PostgreSQL database. Privacy MUST be preserved—no code leaves the machine except by explicit user action.

**Rationale**: Users need full control over their code and confidence that sensitive intellectual property remains on their local system. This enables trust, offline usage, and compliance with security policies.

### II. Test-Driven Development (NON-NEGOTIABLE)

Tests MUST be written first, shown to fail, then implementation proceeds. Every feature MUST follow the Red-Green-Refactor cycle. Integration tests MUST verify contract compliance before feature completion. Unit tests MUST verify individual component behavior.

**Rationale**: TDD ensures correctness, prevents regressions, and documents expected behavior. Given the complexity of multi-language parsing, embeddings, and vector search, upfront testing is essential to maintain quality.

### III. Performance & Efficiency

Indexing operations MUST complete within defined time bounds (5 minutes for 10k files). Memory footprint MUST remain under 500MB RSS during active operations. Query results MUST return in under 1 second for repositories with up to 100k symbols. Incremental re-indexing MUST process only changed files (90% time reduction for <10% file changes).

**Rationale**: Developers need fast feedback loops. Slow indexing or queries break flow state and reduce tool adoption. Memory efficiency prevents resource contention on developer machines.

### IV. Semantic Intelligence

Code search MUST use vector embeddings via ONNX (sentence-transformers/all-MiniLM-L6-v2) and pgvector for semantic similarity, not just text matching. Query results MUST be ranked by relevance (similarity score). Symbol extraction MUST use tree-sitter AST parsing for accurate language understanding.

**Rationale**: Simple text search misses semantic relationships. Developers think in concepts, not exact string matches. AST-based parsing provides structural understanding that regex cannot achieve.

### V. Incremental Processing

File changes MUST be detected via SHA256 content hashing. Re-indexing MUST skip unchanged files. Database schema MUST support efficient delta updates. Operations MUST be idempotent—safe to re-run without corruption.

**Rationale**: Re-parsing entire codebases on every change wastes developer time and compute resources. Smart incremental updates enable frequent re-indexing without performance penalty.

## Technology Stack Requirements

**Language**: Rust 2021 edition (cargo 1.75+)

**Core Dependencies**:
- `tree-sitter` family for multi-language AST parsing
- `tokio` for async runtime
- `postgres` + `pgvector` for vector storage (port 45678)
- `ort` (ONNX Runtime) for embedding generation
- `clap` for CLI interface
- `thiserror` + `anyhow` for error handling

**Development Tools**:
- `cargo test` for test execution (32+ tests required coverage)
- `cargo clippy` with `-D warnings` (zero warnings policy)
- `cargo fmt` for code formatting
- `task` (Taskfile) for standardized build commands

**Platform Support**: macOS, Linux (x86_64, ARM64)

**External Services** (local only):
- PostgreSQL 15+ with pgvector extension (REQUIRED)
- Ollama with llama3.2:8b (OPTIONAL, knowledge graph only)

All dependencies MUST be offline-capable after initial download. No runtime internet requirements except Ollama inference (optional feature).

## Development Workflow

**Error Handling**: Use `thiserror` for domain errors, `anyhow` for context propagation. Convert errors to user-friendly messages via `Error::to_user_message()`. Validation MUST happen early with actionable error messages and troubleshooting steps.

**Naming Conventions**: snake_case for functions/variables, PascalCase for types, SCREAMING_SNAKE_CASE for constants. Module names reflect functionality (`parser`, `indexer`, `query`, `embeddings`, `orchestrator`).

**Testing Strategy**:
- Unit tests via `cargo test --lib`
- Integration tests via `cargo test --test integration_tests`
- Test naming: `test_<module>_<function>_<scenario>`
- Database tests use `setup_test_db()` helper
- Skip tests gracefully when PostgreSQL unavailable (`#[ignore]` + env check)

**Code Review Gates**:
- All tests pass (`cargo test`)
- Zero clippy warnings (`cargo clippy --all-targets -- -D warnings`)
- Code formatted (`cargo fmt --check`)
- Constitution principles verified (see Constitution Check in templates)

**Commit Standards**: Atomic commits per logical change. Commit messages follow conventional format: `<type>: <description>` (feat, fix, refactor, test, docs, chore).

## Governance

**Amendment Process**: Constitution changes require documentation in this file with version bump (see semantic versioning below). All amendments MUST include:
1. Sync Impact Report (prepended as HTML comment)
2. Rationale for change
3. Migration plan for existing code/templates if applicable
4. Updated version number and Last Amended date

**Versioning Policy** (Semantic Versioning):
- **MAJOR**: Backward-incompatible governance/principle removals or redefinitions
- **MINOR**: New principle/section added or materially expanded guidance
- **PATCH**: Clarifications, wording, typo fixes, non-semantic refinements

**Compliance Review**: All PRs MUST verify adherence to constitution principles. Features violating principles MUST document justification in plan.md Complexity Tracking section. Unjustified violations MUST be rejected.

**Simplicity Mandate**: Complexity MUST be justified against simpler alternatives. New abstractions (patterns, frameworks, indirections) MUST solve concrete problems documented in spec.md or plan.md. YAGNI principle applies—do not add features for hypothetical future needs.

**Guidance Documents**: This constitution supersedes all other practices. For runtime development guidance, refer to `AGENTS.md` (build commands, code style, testing patterns). For feature development workflow, refer to `.specify/templates/*.md` (spec, plan, tasks).

**Version**: 1.0.0 | **Ratified**: 2025-11-19 | **Last Amended**: 2025-11-19
