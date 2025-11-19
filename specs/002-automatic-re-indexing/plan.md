# Implementation Plan: Automatic Re-indexing

**Branch**: `002-automatic-re-indexing` | **Date**: 2025-11-19 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-automatic-re-indexing/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Add automatic re-indexing capabilities to Cudgel through a background orchestrator daemon that manages scheduled tasks. Developers can schedule periodic indexing (hourly, daily, weekly) for repositories, and the orchestrator executes incremental re-indexing at the scheduled intervals. The feature uses the existing `scheduled_tasks` database table, PID file-based process management, and integrates with the existing `Indexer` service.

## Technical Context

**Language/Version**: Rust 2021 edition (cargo 1.75+)  
**Primary Dependencies**: tokio (async runtime), chrono (time handling), tracing (logging), postgres (database), existing cudgel modules (Indexer, Database)  
**Storage**: PostgreSQL 15+ with existing `scheduled_tasks` table (port 54321)  
**Testing**: cargo test (unit tests via `--lib`, integration tests via `--test integration_tests`)  
**Target Platform**: macOS, Linux (x86_64, ARM64)
**Project Type**: single (CLI tool with daemon capabilities)  
**Performance Goals**: Orchestrator must use <50MB RAM when idle, scheduled tasks must execute within 60 seconds of scheduled time, daemon must run continuously for 24+ hours without crashing  
**Constraints**: Local-first (no external services), graceful shutdown on SIGTERM/SIGINT, idempotent operations (safe to re-run), PID file-based process management  
**Scale/Scope**: Support multiple concurrent scheduled tasks across different repositories, 60-second polling interval, handle hourly/daily/weekly schedules

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### I. Local-First Architecture
✅ **PASS** - Orchestrator runs as local daemon process. All data stored in local PostgreSQL (scheduled_tasks table). No external service dependencies. PID file and logs stored in `~/.local/state/cudgel/`.

### II. Test-Driven Development
✅ **PASS** - Spec includes comprehensive testing strategy:
- Unit tests: Schedule interval calculations, task filtering, PID file operations
- Integration tests: CLI schedule creation, database verification, daemon lifecycle, time-based execution
- Manual testing: End-to-end workflow documented in spec
- Tests will be written first following Red-Green-Refactor cycle

### III. Performance & Efficiency
✅ **PASS** - Performance goals specified and measurable:
- Memory: <50MB RAM when idle (within 500MB constraint)
- Timing: Tasks execute within 60 seconds of scheduled time
- Reliability: Daemon must run 24+ hours without crashing
- Incremental: Uses existing incremental indexing (only changed files)

### IV. Semantic Intelligence
✅ **PASS** - No changes to semantic search capabilities. Feature builds on existing Indexer service that already uses tree-sitter AST parsing and vector embeddings.

### V. Incremental Processing
✅ **PASS** - Leverages existing incremental indexing. Orchestrator calls existing `Indexer` service which already implements SHA256 content hashing and delta updates. Idempotent operations—safe to re-run.

### Technology Stack Compliance
✅ **PASS** - Uses required stack:
- Rust 2021 edition
- Existing dependencies: tokio, chrono, tracing, postgres
- PostgreSQL with existing schema (scheduled_tasks table)
- cargo test for testing
- cargo clippy/fmt for quality

### Development Workflow Compliance
✅ **PASS** - Follows established patterns:
- Error handling: thiserror for domain errors, user-friendly messages
- Naming: snake_case functions, PascalCase types
- Testing: Unit tests (`cargo test --lib`), integration tests (`cargo test --test integration_tests`)
- Code review: All tests pass, zero clippy warnings, formatted

### Overall Assessment
**STATUS**: ✅ ALL GATES PASS - No constitution violations. Feature is a natural extension of existing indexing capabilities with proper daemon management.

## Project Structure

### Documentation (this feature)

```text
specs/002-automatic-re-indexing/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
│   └── orchestrator-cli.md  # CLI interface contracts
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── orchestrator.rs      # NEW: Daemon management, polling loop, task execution
├── database.rs          # MODIFIED: Add scheduled task CRUD operations
├── main.rs              # MODIFIED: Add --schedule/--unschedule flags, orchestrator subcommand
├── indexer.rs           # EXISTING: Used by orchestrator for task execution
├── config.rs            # EXISTING: May need PID/log path configuration
├── error.rs             # MODIFIED: Add orchestrator-specific errors
└── lib.rs               # MODIFIED: Export orchestrator module

tests/
├── integration_tests.rs # MODIFIED: Add orchestrator lifecycle tests
└── unit/                # NEW: Unit tests for orchestrator
    └── orchestrator_tests.rs

~/.local/state/cudgel/   # Runtime state (created at runtime)
├── orchestrator.pid     # Process ID file
└── orchestrator.log     # Daemon log file
```

**Structure Decision**: Single project structure (Option 1). This is a CLI tool with daemon capabilities, fitting the existing Cudgel architecture. The orchestrator module is a new service alongside existing services (indexer, parser, query, embeddings). Process management files go in standard XDG state directory (`~/.local/state/cudgel/`).

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No violations detected. All constitution principles are satisfied by this feature design.
