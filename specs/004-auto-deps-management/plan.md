# Implementation Plan: Automatic Dependency Management

**Branch**: `004-auto-deps-management` | **Date**: 2025-11-19 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/004-auto-deps-management/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Replace manual setup instructions throughout cudgel codebase with automated `cudgel deps` command that handles model downloads, database startup, schema initialization, and dependency validation. Respects XDG Base Directory specification for all data storage.

## Technical Context

**Language/Version**: Rust 2021 edition (cargo 1.75+)  
**Primary Dependencies**: clap (CLI), tokio (async), postgres + pgvector, ort (ONNX), hf-hub (model download)  
**Storage**: PostgreSQL 15+ (port 45678), XDG-compliant directories (~/.local/share/cudgel, ~/.local/state/cudgel)  
**Testing**: cargo test (unit + integration), setup_test_db() helper for DB tests  
**Target Platform**: macOS, Linux (x86_64, ARM64)  
**Project Type**: Single CLI application with async runtime  
**Performance Goals**: Model download <5 min (reasonable connection), dependency check <2 sec, idempotent operations  
**Constraints**: <500MB memory during operations, offline-capable after initial setup, respects XDG environment variables  
**Scale/Scope**: Single-user developer tool, ~100MB model download, schema initialization for existing tables

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### ✅ I. Local-First Architecture

- Models downloaded to local XDG directories (no external service runtime dependencies)
- PostgreSQL runs locally on custom port (45678)
- No cloud API calls except initial HuggingFace model download
- **Status**: COMPLIANT

### ✅ II. Test-Driven Development

- Will write tests first for dependency validation, model download, database checks
- Integration tests verify contract: `cudgel deps` succeeds when dependencies satisfied
- Unit tests verify individual validators (model exists, DB running, schema initialized)
- **Status**: COMPLIANT - TDD workflow will be followed

### ✅ III. Performance & Efficiency

- Dependency check target: <2 seconds (validates without downloads)
- Model download progress indicators to maintain user awareness
- Idempotent operations - safe to re-run without performance penalty
- **Status**: COMPLIANT - aligns with SC-003

### ✅ IV. Semantic Intelligence

- No impact on semantic search functionality
- Ensures embedding models are available for downstream semantic operations
- **Status**: COMPLIANT - enables, not modifies

### ✅ V. Incremental Processing

- Checks existing model files before downloading (skip if present)
- Verifies database already running before attempting start
- Schema initialization checks existing tables (idempotent CREATE IF NOT EXISTS)
- **Status**: COMPLIANT - implements incremental validation

### Technology Stack Alignment

- ✅ Rust 2021 + cargo 1.75+
- ✅ tokio async runtime (existing)
- ✅ postgres + pgvector (existing, port 45678)
- ✅ clap CLI framework (existing)
- ✅ thiserror for error handling (existing pattern)
- ✅ Platform support: macOS, Linux

### Development Workflow Alignment

- ✅ Error handling via `Error::to_user_message()` with troubleshooting steps
- ✅ snake_case naming conventions
- ✅ Test naming: `test_deps_<scenario>`
- ✅ Zero clippy warnings policy
- ✅ Database tests use `setup_test_db()` helper

**Overall Status**: ✅ PASS - No constitution violations. Feature aligns with local-first, TDD, and efficiency principles.

## Project Structure

### Documentation (this feature)

```text
specs/004-auto-deps-management/
├── plan.md              # This file
├── research.md          # Phase 0 output (model download methods, XDG patterns)
├── data-model.md        # Phase 1 output (Dependency, ModelArtifact, DatabaseInstance entities)
├── quickstart.md        # Phase 1 output (usage examples for cudgel deps)
├── contracts/           # Phase 1 output (CLI interface contract)
│   └── cli-interface.md
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created yet)
```

### Source Code (repository root)

```text
src/
├── deps.rs              # NEW: Dependency management module
│   ├── mod.rs           # Public API: validate(), install(), clean()
│   ├── model.rs         # Model download orchestration
│   ├── database.rs      # PostgreSQL lifecycle management
│   ├── schema.rs        # Schema initialization
│   └── checker.rs       # Dependency validation
├── config.rs            # EXISTING: Already has XDG helpers
├── database.rs          # EXISTING: Database connection
├── embeddings.rs        # UPDATE: Replace manual instructions with "run: cudgel deps"
├── error.rs             # UPDATE: Add DependencyError variants
├── main.rs              # UPDATE: Add Deps subcommand
└── lib.rs               # UPDATE: Export deps module

tests/
├── integration_tests.rs # UPDATE: Add deps integration tests
└── unit/                # NEW: Unit tests for deps module
    ├── test_checker.rs
    ├── test_model.rs
    └── test_database.rs

scripts/
├── start-postgres.sh    # EXISTING: Will be called by deps module
└── stop-postgres.sh     # EXISTING: Will be called by deps clean
```

**Structure Decision**: Single project structure (Option 1). Cudgel is a monolithic CLI tool. New `deps` module added alongside existing modules (indexer, parser, query, embeddings). Follows existing patterns in codebase.

## Complexity Tracking

> No Constitution Check violations - this section is empty.

---

## Phase 0: Research & Technical Investigation

*Research tasks to resolve NEEDS CLARIFICATION items and establish implementation patterns.*

### Research Tasks

1. **Model Download Methods**
   - **Question**: Best approach for downloading HuggingFace models in Rust?
   - **Options**: 
     a) Shell out to existing Python script (optimum-cli)
     b) Pure Rust HTTP download with reqwest
     c) Hybrid: Python for ONNX export, Rust for verification
   - **Investigate**: Error handling, progress indicators, checksum verification, partial download recovery

2. **XDG Directory Patterns in Rust**
   - **Question**: Standard library/crate for XDG directory handling?
   - **Options**: 
     a) Manual environment variable parsing (existing in config.rs)
     b) `dirs` crate
     c) `xdg` crate
   - **Investigate**: Thread safety, Windows compatibility (XDG on Windows uses different paths)

3. **Process Management for PostgreSQL**
   - **Question**: How to reliably detect if PostgreSQL is running and manage lifecycle?
   - **Options**:
     a) Shell out to existing scripts (start-postgres.sh, stop-postgres.sh)
     b) Parse pg_ctl status output
     c) Attempt connection and infer from error
   - **Investigate**: Cross-platform compatibility, PID file handling, graceful shutdown

4. **Progress Indicators for Long Operations**
   - **Question**: User feedback mechanism during model download (potentially 5+ minutes)?
   - **Options**:
     a) `indicatif` crate (progress bars)
     b) Simple stdout percentage updates
     c) Spinner for indeterminate operations
   - **Investigate**: Terminal compatibility, async integration with tokio

5. **Checksum Verification**
   - **Question**: How to verify model integrity after download?
   - **Options**:
     a) HuggingFace provides checksums in model repo
     b) Compute SHA256 and compare against known good values
     c) File size validation only (weaker)
   - **Investigate**: HuggingFace API for checksum retrieval, storage location for expected checksums

*Output: research.md with decisions and rationale for each question*

---

## Phase 1: Design & Contracts

*Prerequisites: research.md complete*

### Design Artifacts

1. **data-model.md**
   - **Dependency** entity: name, status (missing/satisfied/corrupted), validator function, installer function
   - **ModelArtifact** entity: model_id, source_url, target_path, checksum, size_bytes, download_progress
   - **DatabaseInstance** entity: host, port, data_dir, pid, status (stopped/starting/running/error)

2. **contracts/cli-interface.md**
   - CLI contract for `cudgel deps` command
   - Flags: --check (validate only), --clean (remove models), --clean --all (remove everything), --verbose
   - Exit codes: 0 (success), 1 (dependency missing), 2 (validation error), 3 (installation failed), 4 (invalid usage)
   - Output format: Human-readable table with checkmarks/X marks for each dependency

3. **quickstart.md**
   - Usage examples:
     ```bash
     # Initial setup (first time)
     cudgel deps

     # Check status
     cudgel deps --check

     # Clean models only
     cudgel deps --clean

     # Clean everything (requires confirmation)
     cudgel deps --clean --all

     # Verbose output
     cudgel deps --check --verbose
     ```

### Agent Context Update

After generating design artifacts, run:
```bash
.specify/scripts/bash/update-agent-context.sh opencode
```

This will update AGENTS.md with:
- New module: `deps` (dependency management)
- New commands: `cudgel deps`, `cudgel deps --check`, `cudgel deps --clean`
- Testing additions: deps integration tests

---

## Phase 2: Task Breakdown

*This phase is handled by `/speckit.tasks` command - NOT part of this plan output.*

Will generate tasks.md with implementation tasks organized by user story priority (P1, P2, P3).

---

## Next Steps

1. ✅ Complete this plan.md
2. ⏳ Execute Phase 0: Generate research.md (automatic via plan command)
3. ⏳ Execute Phase 1: Generate data-model.md, contracts/, quickstart.md (automatic via plan command)
4. ⏳ Update agent context (automatic via plan command)
5. ⏳ Re-verify Constitution Check post-design
6. ⏳ User runs `/speckit.tasks` to generate tasks.md for implementation
