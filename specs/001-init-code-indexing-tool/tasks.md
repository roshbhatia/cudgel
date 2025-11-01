# Tasks: Cudgel Code Intelligence System

**Input**: Design documents from `/specs/001-init-code-indexing-tool/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are NOT explicitly requested in the feature specification, so test tasks are excluded per template guidelines.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Paths shown below are for single Rust CLI project

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [X] T001 Create Cargo project with workspace structure and bin/lib targets
- [X] T002 [P] Add core dependencies to Cargo.toml (clap, tokio, sqlx, thiserror, anyhow, tracing)
- [X] T003 [P] Add tree-sitter dependencies to Cargo.toml (tree-sitter + 8 language grammars)
- [X] T004 [P] Add utility dependencies to Cargo.toml (config, walkdir, sha2, serde, chrono, ollama-rs)
- [ ] T005 [P] Create flake.nix with package output, devShell, and PostgreSQL service
- [ ] T006 [P] Create .pre-commit-config.yaml with cargo fmt, clippy, test hooks
- [ ] T007 [P] Create project directory structure (src/cli, src/services, src/db, src/traits, src/utils)
- [ ] T007a [P] Document ONNX model setup in README.md (download sentence-transformers/all-MiniLM-L6-v2 to ~/.local/share/cudgel/models/)
- [X] T008 Create src/lib.rs with public module exports
- [X] T009 Create src/main.rs with clap CLI framework and command dispatch
- [X] T010 Create src/error.rs with Error enum using thiserror for all error types

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

**Note**: Schema is implemented inline in src/database.rs (not separate migrations), includes scheduled_tasks and knowledge_documents tables

- [X] T011 Create migrations/001_init.sql with repositories, files, symbols tables (implemented in database.rs)
- [X] T012 Create migrations/002_pgvector.sql with embeddings table and HNSW index (implemented in database.rs)
- [X] T013 Create migrations/003_schedules.sql with scheduled_tasks table (implemented in database.rs)
- [X] T014 Create migrations/004_knowledge.sql with knowledge_documents table (implemented in database.rs)
- [X] T015 Create src/config.rs implementing Config struct with config crate + XDG paths
- [ ] T016 [P] Create src/utils/xdg.rs with XDG Base Directory helper functions (partially in config.rs)
- [ ] T017 [P] Create src/utils/git.rs with git2 wrapper for listing tracked files
- [ ] T018 [P] Create src/utils/hash.rs with SHA256 file content hashing (inline in indexer.rs)
- [X] T019 Create src/db/mod.rs with PgPool connection and migration runner (database.rs)
- [X] T020 [P] Create src/db/repos.rs with Repository CRUD operations (sqlx) (inline in database.rs)
- [X] T021 [P] Create src/db/files.rs with File CRUD operations (sqlx) (inline in database.rs)
- [X] T022 [P] Create src/db/symbols.rs with Symbol CRUD operations (sqlx) (inline in database.rs)
- [X] T023 [P] Create src/db/embeddings.rs with Embedding CRUD and pgvector similarity search (inline in database.rs)
- [ ] T024 Create src/traits/mod.rs with public trait exports
- [ ] T025 Create src/traits/parser.rs defining LanguageParser trait (parser is direct impl, not trait-based)
- [ ] T026 Create src/traits/embeddings.rs defining EmbeddingGenerator trait (embeddings is direct impl)
- [ ] T027 Create src/services/mod.rs with public service exports

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Index and Query Codebase (Priority: P1) 🎯 MVP

**Goal**: Enable developers to manually index repositories and perform semantic searches

**Independent Test**: Run `cudgel index /path/to/repo` followed by `cudgel query "authentication logic"` and verify results in table format

### Implementation for User Story 1

- [X] T028 [P] [US1] Create src/services/parser.rs with ParserService struct and initialization
- [X] T029 [P] [US1] Implement PythonParser in src/services/parser.rs using tree-sitter-python
- [ ] T030 [P] [US1] Implement JavaScriptParser in src/services/parser.rs using tree-sitter-javascript
- [ ] T031 [P] [US1] Implement TypeScriptParser in src/services/parser.rs using tree-sitter-typescript
- [ ] T032 [P] [US1] Implement RustParser in src/services/parser.rs using tree-sitter-rust
- [ ] T033 [P] [US1] Implement GoParser in src/services/parser.rs using tree-sitter-go
- [ ] T034 [P] [US1] Implement CParser in src/services/parser.rs using tree-sitter-c
- [ ] T035 [P] [US1] Implement CppParser in src/services/parser.rs using tree-sitter-cpp
- [ ] T036 [P] [US1] Implement JavaParser in src/services/parser.rs using tree-sitter-java
- [ ] T037 [US1] Implement language detection logic in ParserService based on file extensions
- [X] T038 [US1] Implement symbol extraction via tree-sitter queries in ParserService
- [X] T039 [US1] Create src/services/embeddings.rs with OllamaEmbeddingService using ollama-rs client
- [ ] T040 [US1] Implement embedding generation for text via Ollama llama3.2:8b in EmbeddingService
- [ ] T041 [US1] Implement batch embedding generation with connection pooling in EmbeddingService
- [ ] T042 [US1] Create src/services/indexer.rs with IndexService struct
- [ ] T043 [US1] Implement repository discovery and validation in IndexService
- [ ] T044 [US1] Implement git file listing integration in IndexService using utils/git.rs
- [ ] T045 [US1] Implement incremental indexing logic with hash comparison in IndexService
- [ ] T046 [US1] Implement file parsing workflow (discover → parse → extract symbols) in IndexService
- [ ] T047 [US1] Implement embedding generation workflow (symbols → embeddings) in IndexService
- [X] T048 [US1] Implement database persistence workflow (repos → files → symbols → embeddings) in IndexService
- [X] T049 [US1] Add progress bar output using indicatif crate in IndexService
- [ ] T050 [US1] Create src/cli/index.rs implementing `cudgel index` command handler
- [ ] T051 [US1] Implement path argument parsing and validation in cli/index.rs
- [ ] T052 [US1] Integrate IndexService into cli/index.rs command execution
- [ ] T053 [US1] Implement dependency checks (PostgreSQL, Ollama, git) at startup in cli/index.rs
- [ ] T054 [US1] Implement error handling and exit codes (1-4) in cli/index.rs
- [ ] T055 [US1] Create src/services/query.rs with QueryEngine struct
- [ ] T056 [US1] Implement query embedding generation via Ollama in QueryEngine
- [ ] T057 [US1] Implement pgvector similarity search using db/embeddings.rs in QueryEngine
- [X] T058 [US1] Implement result ranking by similarity score in QueryEngine
- [X] T059 [US1] Implement table formatting using comfy-table crate in QueryEngine
- [X] T060 [US1] Create src/cli/query.rs implementing `cudgel query` command handler
- [X] T061 [US1] Implement search term argument parsing in cli/query.rs
- [X] T062 [US1] Implement --limit flag handling (default 50, max 1000) in cli/query.rs
- [X] T063 [US1] Implement --repo, --language, --type filter flags in cli/query.rs
- [X] T064 [US1] Integrate QueryEngine into cli/query.rs command execution
- [X] T065 [US1] Implement "no results found" handling with helpful message in cli/query.rs
- [X] T066 [US1] Update src/main.rs to register index and query subcommands

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Schedule Automatic Re-indexing (Priority: P2)

**Goal**: Enable automatic repository re-indexing via background orchestrator daemon

**Independent Test**: Run `cudgel index --schedule hourly /path/to/repo`, verify daemon starts, wait for interval, confirm auto re-indexing

### Implementation for User Story 2

- [ ] T067 [US2] Create src/db/schedules.rs with ScheduledTask CRUD operations (sqlx)
- [X] T068 [US2] Implement schedule creation with interval parsing (hourly/daily/N hours) in db/schedules.rs
- [X] T069 [US2] Implement schedule deletion (unschedule) in db/schedules.rs
- [X] T070 [US2] Implement schedule listing with filters (active only) in db/schedules.rs
- [X] T071 [US2] Implement next_run_at calculation logic in db/schedules.rs
- [X] T072 [US2] Create src/services/orchestrator.rs with OrchestratorService struct
- [X] T073 [US2] Implement polling loop with tokio::time::interval (default 60s) in OrchestratorService
- [X] T074 [US2] Implement due task discovery query (SELECT FOR UPDATE SKIP LOCKED) in OrchestratorService
- [X] T075 [US2] Implement concurrent task execution using tokio::spawn in OrchestratorService
- [X] T076 [US2] Integrate IndexService for scheduled re-indexing in OrchestratorService
- [ ] T077 [US2] Implement next_run_at updates after task execution in OrchestratorService
- [X] T078 [US2] Implement graceful shutdown handling (SIGTERM, SIGINT) in OrchestratorService
- [X] T079 [US2] Implement logging to ~/.local/state/cudgel/orchestrator.log using tracing
- [X] T080 [US2] Implement PID file management in ~/.local/state/cudgel/orchestrator.pid
- [X] T081 [US2] Create src/cli/orchestrator.rs implementing `cudgel orchestrator` command
- [X] T082 [US2] Implement `orchestrator start` subcommand with daemon spawning in cli/orchestrator.rs
- [X] T083 [US2] Implement `orchestrator stop` subcommand with PID kill in cli/orchestrator.rs
- [X] T084 [US2] Implement `orchestrator status` subcommand with table output in cli/orchestrator.rs
- [X] T085 [US2] Implement `orchestrator restart` subcommand (stop + start) in cli/orchestrator.rs
- [X] T086 [US2] Implement --foreground flag for debugging in orchestrator start
- [ ] T087 [US2] Update src/cli/index.rs to add --schedule and --unschedule flags
- [X] T088 [US2] Implement schedule creation logic in cli/index.rs when --schedule flag present
- [X] T089 [US2] Implement auto-start orchestrator if not running when scheduling in cli/index.rs
- [X] T090 [US2] Implement unschedule logic in cli/index.rs when --unschedule flag present
- [X] T091 [US2] Update src/main.rs to register orchestrator subcommand

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Generate Knowledge Graph Documentation (Priority: P3)

**Goal**: Generate AI-powered structured documentation for indexed repositories

**Independent Test**: Run `cudgel knowledge` on indexed repo, verify markdown opens in $EDITOR with structured sections, confirm storage in database

### Implementation for User Story 3

- [X] T092 [US3] Create src/db/knowledge.rs with KnowledgeDocument CRUD operations (sqlx)
- [X] T093 [US3] Implement knowledge graph insert/update with version tracking in db/knowledge.rs
- [X] T094 [US3] Implement knowledge graph retrieval by repo_id in db/knowledge.rs
- [X] T095 [US3] Implement last_edited_at timestamp updates in db/knowledge.rs
- [X] T096 [US3] Create src/services/knowledge.rs with KnowledgeGenerator struct
- [ ] T097 [US3] Implement indexed data aggregation (dependencies, architecture, files) in KnowledgeGenerator
- [X] T098 [US3] Implement dependency extraction from manifest files (Cargo.toml, package.json) in KnowledgeGenerator
- [X] T099 [US3] Implement architecture pattern detection (MVC, microservices, monolith) in KnowledgeGenerator
- [X] T100 [US3] Implement build process detection (parse Cargo.toml, Makefile) in KnowledgeGenerator
- [X] T101 [US3] Implement licensing extraction (SPDX identifiers, LICENSE files) in KnowledgeGenerator
- [X] T102 [US3] Implement Ollama prompt construction with aggregated data in KnowledgeGenerator
- [X] T103 [US3] Implement LLM call to llama3.2:8b via ollama-rs for content generation in KnowledgeGenerator
- [X] T104 [US3] Implement markdown parsing and section structuring from LLM response in KnowledgeGenerator
- [X] T105 [US3] Implement manual edit preservation logic (compare sections) in KnowledgeGenerator
- [X] T106 [US3] Implement $EDITOR integration with fallback (vim → nano) in KnowledgeGenerator
- [X] T107 [US3] Implement editor subprocess management (spawn, wait, capture) in KnowledgeGenerator
- [X] T108 [US3] Implement content save-on-close logic in KnowledgeGenerator
- [X] T109 [US3] Create src/cli/knowledge.rs implementing `cudgel knowledge` command
- [X] T110 [US3] Implement path argument parsing (default: current directory) in cli/knowledge.rs
- [X] T111 [US3] Implement --edit flag to open existing document in cli/knowledge.rs
- [X] T112 [US3] Implement --refresh flag to update auto-sections only in cli/knowledge.rs
- [X] T113 [US3] Implement --replace flag to regenerate completely in cli/knowledge.rs
- [X] T114 [US3] Implement --output flag to save to file without editor in cli/knowledge.rs
- [X] T115 [US3] Implement --no-editor flag to print to stdout in cli/knowledge.rs
- [X] T116 [US3] Integrate KnowledgeGenerator into cli/knowledge.rs command execution
- [X] T117 [US3] Implement dependency check for Ollama service in cli/knowledge.rs
- [X] T118 [US3] Implement "repository not indexed" error handling in cli/knowledge.rs
- [X] T119 [US3] Update src/main.rs to register knowledge subcommand

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: User Story 4 - Export Query Results for LLM Consumption (Priority: P4)

**Goal**: Enable machine-readable query output formats (JSON, minified)

**Independent Test**: Run `cudgel query "parser logic" --json` and verify valid JSON output suitable for piping

### Implementation for User Story 4

- [X] T120 [P] [US4] Create src/utils/minifier.rs with LLM-OpenAPI-minifier logic
- [X] T121 [US4] Implement JSON minification (compact format, no whitespace) in utils/minifier.rs
- [X] T122 [US4] Implement key abbreviation (file_path → p, line_number → l, name → n) in utils/minifier.rs
- [X] T123 [US4] Implement null/empty field omission in utils/minifier.rs
- [X] T124 [US4] Update src/services/query.rs to add JSON serialization using serde_json
- [X] T125 [US4] Implement compact JSON formatter in QueryEngine
- [X] T126 [US4] Implement pretty JSON formatter (indented) in QueryEngine
- [X] T127 [US4] Integrate utils/minifier.rs for minified output in QueryEngine
- [X] T128 [US4] Update src/cli/query.rs to add --json flag
- [X] T129 [US4] Update src/cli/query.rs to add --json-pretty flag
- [X] T130 [US4] Update src/cli/query.rs to add --minified flag
- [X] T131 [US4] Implement output format selection logic (table/json/json-pretty/minified) in cli/query.rs
- [X] T132 [US4] Implement stdout output bypass (skip table rendering for JSON modes) in cli/query.rs
- [X] T133 [US4] Validate JSON output is parseable by standard tools (jq compatibility) in cli/query.rs

**Checkpoint**: All user stories complete, full feature set available

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T134 [P] Create README.md with installation instructions (Nix + Cargo), quickstart
- [ ] T135 [P] Update CLAUDE.md with implementation notes and architecture overview
- [ ] T136 [P] Create CONTRIBUTING.md with development setup and PR guidelines
- [ ] T137 [P] Create Taskfile.yml with common tasks (build, test, fmt, clippy, db-start, db-stop)
- [ ] T138 [P] Implement shell completion generation (bash, zsh, fish) using clap in src/main.rs
- [ ] T139 [P] Add --version flag implementation in src/main.rs using Cargo.toml version
- [ ] T140 [P] Add global --config flag handling in src/config.rs
- [ ] T141 [P] Add global --log-level flag with tracing-subscriber initialization in src/main.rs
- [ ] T142 [P] Implement dependency checks helper function (PostgreSQL, Ollama, git) in src/utils
- [ ] T143 [P] Implement XDG directory creation on startup in src/config.rs
- [ ] T144 [P] Add performance monitoring (indexing speed, query latency) logging
- [ ] T145 [P] Add memory usage monitoring during indexing with alerts when approaching 500MB RSS limit
- [ ] T146 Create CHANGELOG.md with initial release notes (v0.1.0)
- [ ] T147 Update Cargo.toml metadata (description, authors, license, repository)
- [ ] T148 Create LICENSE file (MIT based on research)
- [ ] T149 Validate flake.nix builds successfully (nix build)
- [ ] T150 Validate pre-commit hooks run correctly (cargo fmt, clippy, test)
- [ ] T151 Run quickstart.md validation (manual test of 5-minute tour)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3 → P4)
- **Polish (Phase 7)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Extends US1's IndexService but independently testable
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Uses indexed data from US1 but independently testable
- **User Story 4 (P4)**: Can start after Foundational (Phase 2) - Extends US1's QueryEngine but independently testable

### Within Each User Story

**User Story 1 (Index and Query)**:
1. Parser implementations (T028-T038) can run in parallel
2. EmbeddingService (T039-T041) can develop in parallel with Parser
3. IndexService (T042-T049) depends on Parser + EmbeddingService completion
4. cli/index.rs (T050-T054) depends on IndexService completion
5. QueryEngine (T055-T059) can develop in parallel with IndexService
6. cli/query.rs (T060-T065) depends on QueryEngine completion
7. main.rs update (T066) depends on both CLI handlers

**User Story 2 (Scheduling)**:
1. db/schedules.rs (T067-T071) can run in parallel with OrchestratorService
2. OrchestratorService (T072-T080) can develop independently
3. cli/orchestrator.rs (T081-T086) depends on OrchestratorService
4. cli/index.rs updates (T087-T090) can proceed in parallel
5. main.rs update (T091) depends on cli/orchestrator.rs

**User Story 3 (Knowledge Graph)**:
1. db/knowledge.rs (T092-T095) can run in parallel with KnowledgeGenerator
2. KnowledgeGenerator (T096-T108) components can develop in parallel
3. cli/knowledge.rs (T109-T118) depends on KnowledgeGenerator
4. main.rs update (T119) depends on cli/knowledge.rs

**User Story 4 (LLM Formats)**:
1. utils/minifier.rs (T120-T123) can run in parallel with QueryEngine updates
2. QueryEngine updates (T124-T127) can proceed independently
3. cli/query.rs updates (T128-T133) depend on QueryEngine + minifier

### Parallel Opportunities

- All Setup tasks (T002-T007) can run in parallel after T001
- All Foundational database modules (T020-T023) can run in parallel after T019
- All language parsers (T029-T036) can run in parallel within US1
- US1, US2, US3, US4 can be worked on in parallel by different developers after Foundational complete
- All Polish tasks (T134-T145) marked [P] can run in parallel

---

## Parallel Execution Examples

### User Story 1 Parallel Launch

```bash
# Launch all language parsers together:
Task: "Implement PythonParser in src/services/parser.rs"
Task: "Implement JavaScriptParser in src/services/parser.rs"
Task: "Implement TypeScriptParser in src/services/parser.rs"
Task: "Implement RustParser in src/services/parser.rs"
Task: "Implement GoParser in src/services/parser.rs"
Task: "Implement CParser in src/services/parser.rs"
Task: "Implement CppParser in src/services/parser.rs"
Task: "Implement JavaParser in src/services/parser.rs"

# Launch embedding service in parallel:
Task: "Create src/services/embeddings.rs with OllamaEmbeddingService"
```

### Foundational Phase Parallel Launch

```bash
# Launch all database modules together after connection established:
Task: "Create src/db/repos.rs with Repository CRUD operations"
Task: "Create src/db/files.rs with File CRUD operations"
Task: "Create src/db/symbols.rs with Symbol CRUD operations"
Task: "Create src/db/embeddings.rs with Embedding CRUD and similarity search"
```

### Multi-Story Parallel Development

```bash
# With 4 developers, after Foundational complete:
Developer A: User Story 1 (T028-T066) - Index and Query
Developer B: User Story 2 (T067-T091) - Scheduling
Developer C: User Story 3 (T092-T119) - Knowledge Graph
Developer D: User Story 4 (T120-T133) - LLM Formats
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T007a) - 11 tasks
2. Complete Phase 2: Foundational (T011-T027) - CRITICAL - 17 tasks
3. Complete Phase 3: User Story 1 (T028-T066) - 39 tasks
4. **STOP and VALIDATE**: Test `cudgel index` and `cudgel query` independently
5. Deploy/demo MVP

**MVP Scope**: 67 tasks total (Setup + Foundational + US1)

**MVP Delivers**:
- Manual repository indexing with incremental updates
- Semantic code search with table output
- 8 language support (Python, JS, TS, Rust, Go, C, C++, Java)
- Local-first (PostgreSQL + Ollama)
- XDG compliant

### Incremental Delivery

1. **v0.1.0 (MVP)**: Setup + Foundational + US1 → Manual index + query (T001-T066)
2. **v0.2.0**: Add US2 → Automatic scheduling (T067-T091)
3. **v0.3.0**: Add US3 → Knowledge graphs (T092-T119)
4. **v0.4.0**: Add US4 → LLM formats (T120-T133)
5. **v1.0.0**: Polish + documentation (T134-T151)

Each version adds value without breaking previous features.

### Parallel Team Strategy

With multiple developers:

1. **Week 1**: Team completes Setup + Foundational together (T001-T027)
2. **Week 2-3**: Once Foundational is done:
   - Developer A: User Story 1 (T028-T066) - 39 tasks
   - Developer B: User Story 2 (T067-T091) - 25 tasks
   - Developer C: User Story 3 (T092-T119) - 28 tasks
   - Developer D: User Story 4 (T120-T133) - 14 tasks
3. **Week 4**: Integrate and test, complete Polish (T134-T151)

---

## Task Summary

**Total Tasks**: 152

**Breakdown by Phase**:
- Phase 1 (Setup): 11 tasks (includes T007a for ONNX model setup)
- Phase 2 (Foundational): 17 tasks
- Phase 3 (User Story 1 - P1): 39 tasks
- Phase 4 (User Story 2 - P2): 25 tasks
- Phase 5 (User Story 3 - P3): 28 tasks
- Phase 6 (User Story 4 - P4): 14 tasks
- Phase 7 (Polish): 18 tasks

**Parallel Opportunities**: 68 tasks marked [P] (45% parallelizable)

**Independent Test Criteria**:
- US1: `cudgel index /path/to/repo && cudgel query "search term"` returns table results
- US2: `cudgel index --schedule hourly /path/to/repo && cudgel orchestrator status` shows scheduled job
- US3: `cudgel knowledge` opens markdown in $EDITOR with structured sections
- US4: `cudgel query "term" --json | jq` outputs valid JSON

**Suggested MVP Scope**: T001-T066 (Setup + Foundational + User Story 1) = 67 tasks

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- All file paths are exact, no placeholders
- Tests excluded per template guidelines (not explicitly requested in spec)
