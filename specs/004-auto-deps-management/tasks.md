# Tasks: Automatic Dependency Management

**Input**: Design documents from `/specs/004-auto-deps-management/`  
**Branch**: `004-auto-deps-management`  
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli-interface.md, quickstart.md

**Tests**: Following TDD principles per constitution - tests written FIRST, shown to FAIL, then implementation proceeds.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `- [ ] [ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Single Rust project at repository root:
- Source: `src/`
- Tests: `tests/`
- Scripts: `scripts/`
- Docs: `specs/004-auto-deps-management/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and dependency updates

- [ ] T001 Update Cargo.toml to add hf-hub = { version = "0.4", features = ["tokio"] }
- [ ] T002 Update Cargo.toml to upgrade indicatif = { version = "0.18", features = ["tokio"] }
- [ ] T003 [P] Run cargo build to verify new dependencies compile
- [ ] T004 [P] Run cargo clippy to ensure zero warnings with new dependencies

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core error types and configuration that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T005 Add DependencyError variants to src/error.rs (DependencyMissing, ModelDownloadFailed, DatabaseStartFailed, SchemaInitFailed, InsufficientDiskSpace, CorruptedModel)
- [ ] T006 Implement Error::to_user_message() for all new DependencyError variants with troubleshooting steps
- [ ] T007 Create src/deps/ module directory structure (mod.rs, model.rs, database.rs, schema.rs, checker.rs)
- [ ] T008 Export deps module in src/lib.rs
- [ ] T009 Create tests/unit/ directory for deps unit tests

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - First-Time Setup (Priority: P1) 🎯 MVP

**Goal**: Enable developers to run `cudgel deps` and have all dependencies (models, database, schema) automatically installed and configured.

**Independent Test**: Run `cudgel deps` in a fresh environment (no models, no database) and verify that all dependencies are downloaded, database is running, schema is initialized, and command exits with success.

### Tests for User Story 1 (TDD - Write FIRST) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T010 [P] [US1] Create test_deps_validate_all_satisfied in tests/integration_tests.rs
- [ ] T011 [P] [US1] Create test_deps_download_missing_model in tests/integration_tests.rs
- [ ] T012 [P] [US1] Create test_deps_start_postgres_when_stopped in tests/integration_tests.rs
- [ ] T013 [P] [US1] Create test_deps_initialize_schema in tests/integration_tests.rs
- [ ] T014 [P] [US1] Create test_deps_idempotent_when_satisfied in tests/integration_tests.rs
- [ ] T015 [P] [US1] Create test_model_download_with_progress in tests/unit/test_model.rs
- [ ] T016 [P] [US1] Create test_database_is_running_check in tests/unit/test_database.rs
- [ ] T017 [P] [US1] Create test_schema_initialization in tests/unit/test_schema.rs

### Implementation for User Story 1

#### Core Entities (from data-model.md)

- [ ] T018 [P] [US1] Define Dependency struct in src/deps/mod.rs with fields: name, component_type, status, required, validator, installer, error_message
- [ ] T019 [P] [US1] Define ModelArtifact struct in src/deps/model.rs with fields: model_id, filename, source_url, target_path, expected_size_bytes, download_progress_bytes, download_state
- [ ] T020 [P] [US1] Define DatabaseInstance struct in src/deps/database.rs with fields: host, port, data_dir, scripts_dir, status
- [ ] T021 [P] [US1] Define SchemaVersion struct in src/deps/schema.rs with fields: version, tables, indexes, extensions, initialized_at

#### Model Download Implementation

- [ ] T022 [US1] Implement ModelDownloader in src/deps/model.rs using hf-hub Api::new()
- [ ] T023 [US1] Add download_model_artifact() async fn in src/deps/model.rs with progress bar using indicatif
- [ ] T024 [US1] Add verify_model_integrity() fn in src/deps/model.rs (3-layer: ETag, size, functional)
- [ ] T025 [US1] Add cleanup_partial_downloads() fn in src/deps/model.rs for failed downloads
- [ ] T026 [US1] Add disk_space_check() fn in src/deps/model.rs to verify sufficient space before download

#### Database Management Implementation

- [ ] T027 [US1] Implement PostgresManager in src/deps/database.rs with is_running() using pg_isready
- [ ] T028 [US1] Add start() fn in src/deps/database.rs that shells out to scripts/start-postgres.sh
- [ ] T029 [US1] Add detect_port_conflict() fn in src/deps/database.rs using lsof check
- [ ] T030 [US1] Add wait_for_startup() fn in src/deps/database.rs with 30-second timeout

#### Schema Initialization Implementation

- [ ] T031 [US1] Implement SchemaInitializer in src/deps/schema.rs with check_initialized() fn
- [ ] T032 [US1] Add initialize_schema() fn in src/deps/schema.rs using CREATE TABLE IF NOT EXISTS
- [ ] T033 [US1] Add verify_extensions() fn in src/deps/schema.rs to check pgvector is installed
- [ ] T034 [US1] Add spinner progress indicator for schema initialization using indicatif

#### Dependency Validation & Orchestration

- [ ] T035 [US1] Implement DependencyChecker in src/deps/checker.rs with validate_all() fn
- [ ] T036 [US1] Add check_prerequisites() fn in src/deps/checker.rs (PostgreSQL, disk space)
- [ ] T037 [US1] Implement install_all() fn in src/deps/mod.rs that orchestrates model download, DB start, schema init
- [ ] T038 [US1] Add dependency ordering logic in src/deps/mod.rs (DB before schema, etc.)
- [ ] T039 [US1] Add idempotency checks in src/deps/mod.rs (skip satisfied dependencies)

#### CLI Integration

- [ ] T040 [US1] Add Deps subcommand to src/main.rs with clap derive
- [ ] T041 [US1] Implement execute_deps_command() fn in src/main.rs that calls deps::install_all()
- [ ] T042 [US1] Add success/failure output formatting in src/main.rs with checkmarks/X marks
- [ ] T043 [US1] Add --verbose flag parsing to Deps subcommand in src/main.rs

#### Error Message Cleanup (FR-009)

- [ ] T044 [US1] Update src/embeddings.rs to replace manual setup instructions (lines 266-296) with "Run: cudgel deps"
- [ ] T045 [US1] Search codebase for other manual setup instructions and replace with "Run: cudgel deps"
- [ ] T046 [US1] Add lightweight dependency validation to src/indexer.rs startup (check models exist)
- [ ] T047 [US1] Add lightweight dependency validation to src/query.rs startup (check models + DB)
- [ ] T048 [US1] Add lightweight dependency validation to src/orchestrator.rs startup (check models + DB)

**Checkpoint**: At this point, `cudgel deps` should work end-to-end for first-time setup

---

## Phase 4: User Story 2 - Dependency Validation (Priority: P2)

**Goal**: Enable developers to run `cudgel deps --check` to validate their environment without modifying the system, and `cudgel deps --check --verbose` for detailed diagnostics.

**Independent Test**: Run `cudgel deps --check` in various states (missing models, DB stopped, partial setup) and verify accurate status reporting. Run `cudgel deps --check --verbose` and verify detailed paths/versions are shown.

### Tests for User Story 2 (TDD - Write FIRST) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T049 [P] [US2] Create test_deps_check_all_satisfied in tests/integration_tests.rs
- [ ] T050 [P] [US2] Create test_deps_check_missing_dependencies in tests/integration_tests.rs
- [ ] T051 [P] [US2] Create test_deps_check_verbose_output in tests/integration_tests.rs
- [ ] T052 [P] [US2] Create test_deps_check_does_not_modify_system in tests/integration_tests.rs
- [ ] T053 [P] [US2] Create test_deps_check_fast_performance in tests/unit/test_checker.rs (verify <2 sec)

### Implementation for User Story 2

- [ ] T054 [US2] Add --check flag to Deps subcommand in src/main.rs
- [ ] T055 [US2] Implement validate_only() fn in src/deps/mod.rs that checks without installing
- [ ] T056 [US2] Add format_validation_table() fn in src/deps/checker.rs for status output table
- [ ] T057 [US2] Add collect_diagnostics() fn in src/deps/checker.rs for --verbose mode
- [ ] T058 [US2] Add show_verbose_diagnostics() fn in src/deps/checker.rs to display paths, versions, PIDs
- [ ] T059 [US2] Update execute_deps_command() in src/main.rs to handle --check flag and exit codes

**Checkpoint**: At this point, `cudgel deps --check` and `cudgel deps --check --verbose` should work independently

---

## Phase 5: User Story 3 - Clean Dependency Management (Priority: P3)

**Goal**: Enable developers to run `cudgel deps --clean` to remove models (keep DB) or `cudgel deps --clean --all` to remove all data including database.

**Independent Test**: Run `cudgel deps --clean` after setup and verify models are removed but DB preserved. Run `cudgel deps --clean --all` and verify all XDG directories are removed.

### Tests for User Story 3 (TDD - Write FIRST) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T060 [P] [US3] Create test_deps_clean_removes_models in tests/integration_tests.rs
- [ ] T061 [P] [US3] Create test_deps_clean_preserves_database in tests/integration_tests.rs
- [ ] T062 [P] [US3] Create test_deps_clean_all_removes_everything in tests/integration_tests.rs
- [ ] T063 [P] [US3] Create test_deps_clean_all_stops_database in tests/integration_tests.rs
- [ ] T064 [P] [US3] Create test_deps_clean_confirmation_prompt in tests/integration_tests.rs

### Implementation for User Story 3

- [ ] T065 [US3] Add --clean flag to Deps subcommand in src/main.rs
- [ ] T066 [US3] Add --all flag to Deps subcommand in src/main.rs
- [ ] T067 [US3] Add stop() fn to PostgresManager in src/deps/database.rs that shells out to scripts/stop-postgres.sh
- [ ] T068 [US3] Implement clean_models() fn in src/deps/model.rs to remove XDG_DATA_HOME/cudgel/models/
- [ ] T069 [US3] Implement clean_database() fn in src/deps/database.rs to stop DB and remove data directory
- [ ] T070 [US3] Implement clean_all() fn in src/deps/mod.rs to remove all XDG directories
- [ ] T071 [US3] Add confirm_destructive_operation() fn in src/deps/mod.rs with TTY detection
- [ ] T072 [US3] Add calculate_freed_space() fn in src/deps/mod.rs to report disk space freed
- [ ] T073 [US3] Update execute_deps_command() in src/main.rs to handle --clean and --all flags

**Checkpoint**: At this point, all three user stories should work independently

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T074 [P] Add comprehensive doc comments to src/deps/mod.rs public API
- [ ] T075 [P] Add comprehensive doc comments to src/deps/model.rs
- [ ] T076 [P] Add comprehensive doc comments to src/deps/database.rs
- [ ] T077 [P] Add comprehensive doc comments to src/deps/schema.rs
- [ ] T078 [P] Add comprehensive doc comments to src/deps/checker.rs
- [ ] T079 [P] Update README.md with quickstart example using `cudgel deps`
- [ ] T080 Run cargo clippy --all-targets -- -D warnings to ensure zero warnings
- [ ] T081 Run cargo fmt to ensure consistent formatting
- [ ] T082 Run cargo test to verify all 32+ tests pass
- [ ] T083 Verify quickstart.md examples match actual CLI behavior
- [ ] T084 Measure performance: verify `cudgel deps --check` completes in <2 seconds
- [ ] T085 Measure performance: verify `cudgel deps` first-time setup completes in <5 minutes (reasonable connection)
- [ ] T086 Manual test: Verify XDG environment variables are respected (XDG_DATA_HOME, XDG_STATE_HOME, XDG_CACHE_HOME)
- [ ] T087 Manual test: Run through all quickstart.md examples end-to-end
- [ ] T088 Update AGENTS.md if new patterns or conventions emerged during implementation

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-5)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if team capacity allows)
  - Or sequentially in priority order (P1 → P2 → P3)
- **Polish (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Reuses US1 entities but independently testable
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Reuses US1 entities but independently testable

**Note**: All user stories are designed to be independently testable and deliverable. US2 and US3 reuse entities from US1 but add new functionality that can be tested in isolation.

### Within Each User Story (TDD Flow)

1. **Tests FIRST**: Write all tests for the story, ensure they FAIL
2. **Red → Green → Refactor**:
   - Red: Tests fail (expected)
   - Green: Implement minimum code to pass tests
   - Refactor: Clean up implementation
3. **Order within story**:
   - Tests (all in parallel)
   - Entities/models (can run in parallel)
   - Services/logic (depend on entities)
   - CLI integration (depends on services)
   - Error message cleanup (depends on CLI working)

### Parallel Opportunities

**Phase 1 (Setup)**: All tasks marked [P] can run in parallel
- T003 and T004 can run in parallel after T001-T002 complete

**Phase 2 (Foundational)**: No parallel opportunities - sequential setup

**Phase 3 (US1 - Tests)**: Tasks T010-T017 marked [P] can all run in parallel

**Phase 3 (US1 - Entities)**: Tasks T018-T021 marked [P] can all run in parallel (after tests written)

**Phase 3 (US1 - Error Cleanup)**: Tasks T044-T048 can run in parallel after CLI working

**Phase 4 (US2 - Tests)**: Tasks T049-T053 marked [P] can all run in parallel

**Phase 5 (US3 - Tests)**: Tasks T060-T064 marked [P] can all run in parallel

**Phase 6 (Polish)**: Tasks T074-T079 marked [P] can all run in parallel

**Cross-Story Parallelism**: If team has 3 developers:
- Dev 1: Complete US1 (Phase 3)
- Dev 2: Start US2 (Phase 4) after Foundational
- Dev 3: Start US3 (Phase 5) after Foundational

---

## Parallel Example: User Story 1

```bash
# After Phase 2 completes, run tests in parallel
cargo test test_deps_validate_all_satisfied &
cargo test test_deps_download_missing_model &
cargo test test_deps_start_postgres_when_stopped &
cargo test test_deps_initialize_schema &
cargo test test_deps_idempotent_when_satisfied &
cargo test test_model_download_with_progress &
cargo test test_database_is_running_check &
cargo test test_schema_initialization &
wait

# Tests should FAIL (expected - red phase)

# Then implement entities in parallel
# (Open 4 terminal windows or use parallel make)
# T018: Implement Dependency struct
# T019: Implement ModelArtifact struct
# T020: Implement DatabaseInstance struct  
# T021: Implement SchemaVersion struct

# Then proceed with services (sequential dependencies)
# T022-T026: Model download (depends on T019)
# T027-T030: Database management (depends on T020)
# T031-T034: Schema initialization (depends on T021)
# T035-T039: Orchestration (depends on all above)
# T040-T043: CLI integration (depends on T037)

# Finally error cleanup in parallel
# T044-T048: Update error messages across codebase
```

---

## Implementation Strategy

### MVP Scope (Recommended First Deliverable)

**Deliver User Story 1 ONLY as MVP**:
- Enables complete first-time setup: `cudgel deps`
- Provides immediate value to users
- Fully testable and demonstrable
- ~40 tasks (T001-T048)
- Estimated: 1-2 days for experienced Rust developer

**After MVP validated, add**:
- User Story 2 (validation): ~15 tasks (T049-T059) - adds `--check` functionality
- User Story 3 (cleanup): ~15 tasks (T060-T073) - adds `--clean` functionality

### TDD Workflow (Per Constitution)

1. **Write failing tests**: Start each story by writing ALL tests (marked [P])
2. **Verify tests fail**: Run `cargo test` and confirm failures (RED phase)
3. **Implement minimum code**: Write simplest implementation to pass tests (GREEN phase)
4. **Refactor**: Clean up implementation while keeping tests passing (REFACTOR phase)
5. **Integration test**: Run full `cudgel deps` command to verify end-to-end behavior
6. **Move to next story**: Only after current story is complete and tests passing

### Incremental Delivery

Each user story should be:
- ✅ Independently implementable (doesn't require other stories)
- ✅ Independently testable (has its own test suite)
- ✅ Independently valuable (delivers user benefit on its own)
- ✅ Incrementally deliverable (can ship to users after story completes)

**Delivery Order**:
1. **US1 (MVP)**: Ship `cudgel deps` for first-time setup
2. **US2**: Ship `cudgel deps --check` for validation
3. **US3**: Ship `cudgel deps --clean` for cleanup

---

## Task Summary

**Total Tasks**: 88

**Tasks by Phase**:
- Phase 1 (Setup): 4 tasks
- Phase 2 (Foundational): 5 tasks
- Phase 3 (US1 - First-Time Setup): 40 tasks
  - Tests: 8 tasks (T010-T017)
  - Implementation: 32 tasks (T018-T048)
- Phase 4 (US2 - Validation): 11 tasks
  - Tests: 5 tasks (T049-T053)
  - Implementation: 6 tasks (T054-T059)
- Phase 5 (US3 - Cleanup): 14 tasks
  - Tests: 5 tasks (T060-T064)
  - Implementation: 9 tasks (T065-T073)
- Phase 6 (Polish): 14 tasks (T074-T088)

**Parallel Opportunities**: 35 tasks marked [P] can run in parallel within their phase

**Independent Test Criteria**:
- US1: Run `cudgel deps` in fresh environment → all dependencies installed
- US2: Run `cudgel deps --check` in various states → accurate status reported
- US3: Run `cudgel deps --clean --all` → all data removed cleanly

**Suggested MVP Scope**: User Story 1 only (49 tasks including setup and foundational)

**Estimated Implementation Time** (experienced Rust developer):
- MVP (US1): 1-2 days
- US2 (Validation): 0.5 days
- US3 (Cleanup): 0.5 days
- Polish: 0.5 days
- **Total**: 2.5-3.5 days

---

## Format Validation

✅ All tasks follow checklist format: `- [ ] [ID] [P?] [Story?] Description with file path`
✅ Task IDs sequential: T001 through T088
✅ All user story tasks labeled: [US1], [US2], [US3]
✅ Parallel tasks marked: [P] where applicable
✅ File paths specified: All implementation tasks include exact file paths
✅ Dependencies clear: Phase structure shows execution order
✅ TDD workflow: Tests written FIRST for each story
✅ Independent stories: Each story can be tested and delivered independently
