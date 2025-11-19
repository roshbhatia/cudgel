# Implementation Tasks: Automatic Re-indexing

**Feature**: 002-automatic-re-indexing  
**Branch**: `002-automatic-re-indexing`  
**Date**: 2025-11-19

## Overview

Implementation tasks for automatic re-indexing feature, organized by user story for independent development and testing. This feature adds a background orchestrator daemon that manages scheduled code indexing tasks (hourly, daily, weekly).

---

## Task Summary

**Total Tasks**: 35  
**Test Tasks**: 11 (TDD approach)  
**Implementation Tasks**: 24  
**Parallel Opportunities**: 15 tasks marked with [P]

---

## User Story Mapping

### User Story 1 (P2): Schedule Automatic Re-indexing

**Goal**: Allow developers to schedule periodic automatic indexing for repositories without manual intervention.

**Acceptance Criteria**:
1. ✅ Schedule creation stores task in database and starts orchestrator if needed
2. ✅ Orchestrator automatically executes incremental indexing at scheduled intervals
3. ✅ Multiple repositories can have different schedules running concurrently
4. ✅ Status command shows all scheduled jobs with execution history
5. ✅ Unschedule removes task from database

**Independent Test**: Run `cudgel index --schedule hourly /path/to/repo`, confirm orchestrator running, wait for scheduled interval, verify automatic re-indexing occurred.

**Task Count**: 35 tasks (entire feature is one user story)

---

## Phase 1: Setup & Database Schema

**Goal**: Extend database schema and set up test infrastructure.

**Prerequisites**: None (blocking for all other phases)

### Tasks

- [ ] T001 Add schema migration for scheduled_tasks table extensions in migrations/ (add status, version, retry_count, error_message columns)
- [ ] T002 Create indexes for scheduled_tasks table: idx_tasks_next_run (WHERE status='idle'), idx_tasks_status
- [ ] T003 Add ScheduledTask struct to src/database.rs with all fields (id, repository_id, schedule_type, schedule_value, last_run_at, next_run_at, created_at, status, version, retry_count, error_message)
- [ ] T004 Add orchestrator-specific errors to src/error.rs (AlreadyRunning, PidLockError, OrchestatorError, InvalidScheduleType)
- [ ] T005 [P] Create tests/unit/ directory for unit tests
- [ ] T006 [P] Add test helper function setup_test_db() to tests/integration_tests.rs for orchestrator tests

**Phase Complete When**: Database schema extended, error types defined, test infrastructure ready

---

## Phase 2: Foundational - Database Operations

**Goal**: Implement scheduled task CRUD operations (blocking for user story implementation).

**Prerequisites**: Phase 1 complete

### Tasks

- [ ] T007 Write unit test for create_scheduled_task() in tests/unit/database_tests.rs
- [ ] T008 Implement create_scheduled_task(repo_id, schedule_type) in src/database.rs (inserts row, calculates next_run_at)
- [ ] T009 [P] Write unit test for get_scheduled_tasks() in tests/unit/database_tests.rs
- [ ] T010 [P] Implement get_scheduled_tasks() in src/database.rs (returns all scheduled tasks)
- [ ] T011 [P] Write unit test for get_due_tasks() in tests/unit/database_tests.rs
- [ ] T012 [P] Implement get_due_tasks() in src/database.rs (WHERE next_run_at <= NOW() AND status='idle')
- [ ] T013 [P] Write unit test for claim_task() with optimistic locking in tests/unit/database_tests.rs
- [ ] T014 [P] Implement claim_task(task_id, version) in src/database.rs (UPDATE with version check, returns Option)
- [ ] T015 [P] Write unit test for complete_task() in tests/unit/database_tests.rs
- [ ] T016 [P] Implement complete_task(task_id, schedule_type) in src/database.rs (update last_run_at, calculate next_run_at, reset status/retry_count/error)
- [ ] T017 [P] Write unit test for delete_scheduled_task() in tests/unit/database_tests.rs
- [ ] T018 [P] Implement delete_scheduled_task(repo_id) in src/database.rs (DELETE from scheduled_tasks)

**Phase Complete When**: All database operations implemented and tested, tests pass

---

## Phase 3: User Story 1 (P2) - Core Orchestrator Module

**Goal**: Implement orchestrator daemon with PID management, polling loop, and task execution.

**Prerequisites**: Phase 2 complete

### Tasks - PID File Management

- [ ] T019 [US1] Write unit test for PidLock::acquire() in tests/unit/orchestrator_tests.rs (stale file removal, lock acquisition)
- [ ] T020 [US1] Create PidLock struct in src/orchestrator.rs with file: File and path: PathBuf fields
- [ ] T021 [US1] Implement PidLock::acquire(path) in src/orchestrator.rs (atomic creation, exclusive lock, write PID)
- [ ] T022 [US1] [P] Write unit test for PidLock::is_running() in tests/unit/orchestrator_tests.rs
- [ ] T023 [US1] [P] Implement PidLock::is_running(path) in src/orchestrator.rs (try_lock test)
- [ ] T024 [US1] [P] Implement Drop for PidLock in src/orchestrator.rs (remove PID file)

### Tasks - Schedule Calculation

- [ ] T025 [US1] [P] Write unit test for calculate_next_run() in tests/unit/orchestrator_tests.rs (hourly, daily, weekly)
- [ ] T026 [US1] [P] Implement calculate_next_run(schedule_type, last_run) in src/orchestrator.rs (returns DateTime<Utc>)

### Tasks - Graceful Shutdown

- [ ] T027 [US1] Create shutdown module in src/orchestrator.rs with shutdown_signal() function
- [ ] T028 [US1] Implement shutdown_signal() in src/orchestrator.rs (tokio::signal::ctrl_c + SIGTERM handler)
- [ ] T029 [US1] Create Shutdown struct in src/orchestrator.rs with broadcast::Receiver and is_shutdown() method
- [ ] T030 [US1] Implement graceful shutdown coordination in src/orchestrator.rs (broadcast + mpsc channels, 30s timeout)

### Tasks - Task Execution & Polling Loop

- [ ] T031 [US1] Write unit test for execute_task() in tests/unit/orchestrator_tests.rs (mock Indexer call)
- [ ] T032 [US1] Implement execute_task(db, task) in src/orchestrator.rs (calls Indexer, handles success/failure, updates task)
- [ ] T033 [US1] Implement handle_failure(db, task_id, error) in src/orchestrator.rs (retry logic with exponential backoff, max 5 retries)
- [ ] T034 [US1] Implement run_polling_loop(db, shutdown) in src/orchestrator.rs (60s interval, get due tasks, spawn parallel execution, graceful shutdown)

**Phase Complete When**: Core orchestrator module implemented with PID management, polling, and graceful shutdown; unit tests pass

---

## Phase 4: User Story 1 (P2) - CLI Integration

**Goal**: Add CLI commands for schedule management and orchestrator control.

**Prerequisites**: Phase 3 complete

### Tasks - CLI Structure

- [ ] T035 [US1] Add schedule: Option<String> field to Index command in src/main.rs
- [ ] T036 [US1] Add unschedule: bool field to Index command in src/main.rs
- [ ] T037 [US1] Create Orchestrator enum with Start, Stop, Restart, Status variants in src/main.rs
- [ ] T038 [US1] Add Orchestrator subcommand to Commands enum in src/main.rs

### Tasks - Schedule Management Commands

- [ ] T039 [US1] Implement handle_schedule_command(db, path, frequency) in src/main.rs (validate path, check if indexed, create scheduled task, start orchestrator if needed)
- [ ] T040 [US1] Implement handle_unschedule_command(db, path) in src/main.rs (validate path, delete scheduled task, output success message)

### Tasks - Orchestrator Control Commands

- [ ] T041 [US1] Implement handle_orchestrator_start(db) in src/main.rs (check if running via PidLock, spawn daemon process, output PID/paths)
- [ ] T042 [US1] Implement handle_orchestrator_stop() in src/main.rs (read PID, send SIGTERM, wait 30s, send SIGKILL if timeout, remove PID file)
- [ ] T043 [US1] Implement handle_orchestrator_restart(db) in src/main.rs (call stop then start)
- [ ] T044 [US1] Implement handle_orchestrator_status(db) in src/main.rs (check PID running, get all scheduled tasks, format output with next run times)

**Phase Complete When**: All CLI commands implemented; manual testing shows commands work

---

## Phase 5: User Story 1 (P2) - Integration Tests

**Goal**: Verify end-to-end functionality with integration tests.

**Prerequisites**: Phase 4 complete

### Tasks

- [ ] T045 [US1] Write integration test for schedule creation in tests/integration_tests.rs (CLI --schedule, verify DB row, check orchestrator started)
- [ ] T046 [US1] Write integration test for task execution in tests/integration_tests.rs (schedule task, mock time to trigger, verify indexing called, verify task updated)
- [ ] T047 [US1] Write integration test for multiple concurrent schedules in tests/integration_tests.rs (schedule 3 repos with different frequencies, verify all execute)
- [ ] T048 [US1] Write integration test for orchestrator status in tests/integration_tests.rs (schedule tasks, call status, verify output format)
- [ ] T049 [US1] Write integration test for unschedule in tests/integration_tests.rs (schedule then unschedule, verify DB row deleted)
- [ ] T050 [US1] Write integration test for daemon lifecycle in tests/integration_tests.rs (start, verify running, stop, verify stopped)
- [ ] T051 [US1] Write integration test for graceful shutdown in tests/integration_tests.rs (start orchestrator with active task, send SIGTERM, verify task completes before shutdown)

**Phase Complete When**: All integration tests pass, feature is independently testable

---

## Phase 6: Polish & Documentation

**Goal**: Final touches for production readiness.

**Prerequisites**: Phase 5 complete

### Tasks

- [ ] T052 Run cargo clippy --all-targets -- -D warnings and fix any warnings in all modified files
- [ ] T053 Run cargo fmt to format all code
- [ ] T054 Run full test suite (cargo test) and verify all tests pass (32+ tests expected)
- [ ] T055 Update README.md with orchestrator documentation (commands, examples, troubleshooting)

**Phase Complete When**: Zero clippy warnings, all tests pass, documentation updated

---

## Dependency Graph

```
Phase 1 (Setup)
    ↓
Phase 2 (Database Operations)
    ↓
Phase 3 (Core Orchestrator) + Phase 4 (CLI Integration)
    ↓                               ↓
    └───────────┬───────────────────┘
                ↓
        Phase 5 (Integration Tests)
                ↓
        Phase 6 (Polish)
```

**Critical Path**: Phase 1 → Phase 2 → Phase 3 → Phase 5 → Phase 6

**Parallel Opportunities**: 
- Phase 3 and Phase 4 can be developed in parallel after Phase 2
- Within Phase 2: Tasks T009-T018 can be parallelized (different DB operations)
- Within Phase 3: Tasks T022-T026 can be parallelized (PID testing + schedule calculation)

---

## Parallel Execution Examples

### After Phase 2 Complete:

**Team Member A** (Phase 3 - Core Orchestrator):
```bash
# Work on orchestrator internals
- T019-T024: PID file management
- T025-T026: Schedule calculation  
- T027-T034: Shutdown + polling loop
```

**Team Member B** (Phase 4 - CLI Integration):
```bash
# Work on CLI commands
- T035-T038: CLI structure
- T039-T044: Command handlers
```

Both can work independently since orchestrator module and CLI are separate concerns.

---

## Testing Strategy

**TDD Approach**: All tasks follow Red-Green-Refactor cycle:
1. Write failing test (T007, T009, T011, etc.)
2. Implement minimum code to pass (T008, T010, T012, etc.)
3. Refactor if needed

**Test Coverage**:
- **Unit Tests** (tests/unit/): Database operations, schedule calculation, PID management, task execution
- **Integration Tests** (tests/integration_tests.rs): End-to-end CLI workflows, daemon lifecycle, concurrent execution
- **Manual Testing** (from quickstart.md): Real-world usage verification

**Success Criteria**:
- All unit tests pass (20+ tests)
- All integration tests pass (7+ tests)
- Zero clippy warnings
- Manual test workflow completes successfully

---

## Implementation Strategy

### MVP Scope (User Story 1 Only)

**Minimum Viable Product** includes:
1. Database schema extensions (Phase 1)
2. Database CRUD operations (Phase 2)
3. Core orchestrator module (Phase 3)
4. CLI integration (Phase 4)
5. Basic integration tests (Phase 5)

**Why this is MVP**: Single user story feature - all components are required for the feature to work. No optional components.

### Incremental Delivery

**Deliverable 1** (After Phase 2):
- Database operations tested and working
- Can manually insert/query scheduled tasks
- Foundation for daemon development

**Deliverable 2** (After Phase 3):
- Orchestrator module complete
- Can start/stop daemon programmatically
- Polling loop operational

**Deliverable 3** (After Phase 4):
- Full CLI integration
- Feature complete (all acceptance criteria met)
- Ready for integration testing

**Deliverable 4** (After Phase 6):
- Production ready
- All tests pass
- Documentation complete

---

## Task Validation Checklist

✅ **Format Compliance**:
- [x] All tasks use `- [ ]` checkbox format
- [x] All tasks have Task ID (T001-T055)
- [x] User story tasks marked with [US1]
- [x] Parallelizable tasks marked with [P]
- [x] All tasks include file paths

✅ **Organization**:
- [x] Tasks organized by phase
- [x] Phases follow dependency order
- [x] Each phase has clear goal and completion criteria
- [x] Independent test criteria defined for user story

✅ **Completeness**:
- [x] All spec components covered (database, orchestrator, CLI, tests)
- [x] TDD approach (tests before implementation)
- [x] Error handling tasks included
- [x] Documentation tasks included

✅ **Executability**:
- [x] Each task is specific and actionable
- [x] File paths provided for all code changes
- [x] Test strategy clearly defined
- [x] Parallel execution opportunities identified

---

## Quick Reference

**File Modifications**:
- `migrations/`: Schema extensions (T001-T002)
- `src/database.rs`: CRUD operations (T003, T008, T010, T012, T014, T016, T018)
- `src/error.rs`: New error types (T004)
- `src/orchestrator.rs`: NEW module (T020-T034)
- `src/main.rs`: CLI updates (T035-T044)
- `src/lib.rs`: Export orchestrator module
- `tests/unit/`: New unit tests (T005, T007-T032)
- `tests/integration_tests.rs`: Integration tests (T006, T045-T051)
- `README.md`: Documentation (T055)

**Key Patterns**:
- **Optimistic locking**: `version` field prevents duplicate execution
- **Graceful shutdown**: 3-phase (detect → notify → complete)
- **PID management**: Atomic creation + advisory lock
- **Schedule calculation**: UTC-only, database as clock source
- **Parallel execution**: Per-repository concurrency

**Performance Targets**:
- Memory: <50MB RAM when idle
- Timing: Tasks execute within 60 seconds of scheduled time
- Reliability: 24+ hour continuous operation
- Test suite: <30 seconds total execution time
