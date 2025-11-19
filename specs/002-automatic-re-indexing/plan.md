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

[Gates determined based on constitution file]

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
<!--
  ACTION REQUIRED: Replace the placeholder tree below with the concrete layout
  for this feature. Delete unused options and expand the chosen structure with
  real paths (e.g., apps/admin, packages/something). The delivered plan must
  not include Option labels.
-->

```text
# [REMOVE IF UNUSED] Option 1: Single project (DEFAULT)
src/
├── models/
├── services/
├── cli/
└── lib/

tests/
├── contract/
├── integration/
└── unit/

# [REMOVE IF UNUSED] Option 2: Web application (when "frontend" + "backend" detected)
backend/
├── src/
│   ├── models/
│   ├── services/
│   └── api/
└── tests/

frontend/
├── src/
│   ├── components/
│   ├── pages/
│   └── services/
└── tests/

# [REMOVE IF UNUSED] Option 3: Mobile + API (when "iOS/Android" detected)
api/
└── [same as backend above]

ios/ or android/
└── [platform-specific structure: feature modules, UI flows, platform tests]
```

**Structure Decision**: [Document the selected structure and reference the real
directories captured above]

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |
