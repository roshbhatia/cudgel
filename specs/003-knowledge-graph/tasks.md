# Tasks: Knowledge Graph for Code Understanding

**Input**: Design documents from `/specs/003-knowledge-graph/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: This feature follows Test-Driven Development (TDD) per constitution requirement. Tests are written FIRST before implementation.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Project structure from plan.md:
  - New modules: `src/graph/`, `src/llm/`
  - Modified files: `src/orchestrator.rs`, `src/indexer.rs`, `src/main.rs`
  - Tests: `tests/test_graph_*.rs`, `tests/fixtures/sample_repo/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and dependency setup

- [x] T001 Add PostgreSQL dependency to Cargo.toml (already exists: tokio-postgres)
- [x] T002 Add Ollama client dependency to Cargo.toml (ollama-rs = "0.2")
- [x] T003 [P] Add fuzzy matching dependency to Cargo.toml (strsim = "0.11")
- [x] T004 [P] Add regex dependency to Cargo.toml (regex = "1.10")
- [x] T005 Create src/kg/mod.rs module structure (already exists)
- [x] T006 Create src/llm/mod.rs module structure (already exists)
- [x] T007 Create tests/fixtures/sample_repo/ directory for test data

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core graph and LLM infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Graph Database Foundation

- [x] T008 Define KgError enum with thiserror in src/kg/mod.rs (already exists)
- [x] T009 Define core data types (Repository, Component, CodeEntity, EntityType, ComponentType, Visibility, EntityMetadata) in src/kg/model.rs (already exists)
- [x] T010 Define relationship types (DependencyType, EntityRelationships, RelatedEntity, RepositoryStats) in src/kg/model.rs (already exists)
- [x] T011 [P] Define EntityMatch, EntityRelationships, RelatedEntity, RepositoryStats types in src/kg/model.rs (already exists)
- [x] T012 Define KgClient trait interface per contract in src/kg/client.rs (already exists)
- [x] T013 Implement PostgreSQL connection and initialization in src/kg/client.rs (already exists)
- [x] T014 Implement initialize_schema() to create tables and indexes in src/kg/schema.rs (already exists)
- [x] T015 Implement is_schema_initialized() check in src/kg/schema.rs (already exists)

### LLM Client Foundation

- [x] T016 Define LlmError enum with thiserror in src/llm/mod.rs
- [x] T017 Define context types (RepositoryContext, ComponentContext, EntityContext) in src/llm/client.rs
- [x] T018 Define request types (SummaryRequest, SummaryResult, ServiceHealth) in src/llm/client.rs
- [x] T019 Define LlmClient trait interface per contract in src/llm/client.rs
- [x] T020 Implement Ollama HTTP client connection in src/llm/client.rs
- [x] T021 [P] Implement health_check() and list_models() in src/llm/client.rs
- [x] T022 [P] Define prompt templates (REPOSITORY_PROMPT, COMPONENT_PROMPT, ENTITY_PROMPT, PATTERN_ANALYSIS_PROMPT) in src/llm/prompts.rs

### Test Infrastructure

- [x] T023 Create setup_test_graph_client() helper in tests/test_graph_client.rs
- [x] T024 Create setup_test_llm_client() helper (mock) in tests/test_llm_integration.rs
- [x] T025 [P] Create sample Rust code in tests/fixtures/sample_repo/src/main.rs
- [x] T026 [P] Create sample Rust module in tests/fixtures/sample_repo/src/parser.rs
- [x] T027 [P] Create sample Rust module in tests/fixtures/sample_repo/src/indexer.rs
- [x] T028 [P] Create Cargo.toml metadata in tests/fixtures/sample_repo/Cargo.toml

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Repository Architecture Understanding (Priority: P1) 🎯 MVP

**Goal**: Enable developers to query "what is the architecture?" and get LLM-generated repository-level summaries with component listings

**Independent Test**: Index tests/fixtures/sample_repo/, query "what is the overall architecture?", verify coherent summary is returned

### Tests for User Story 1 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T029 [P] [US1] Unit test: test_create_and_get_repository in tests/test_graph_client.rs
- [x] T030 [P] [US1] Unit test: test_create_and_get_component in tests/test_graph_client.rs
- [x] T031 [P] [US1] Unit test: test_create_and_get_entity in tests/test_graph_client.rs
- [x] T032 [P] [US1] Unit test: test_create_entities_batch in tests/test_graph_client.rs
- [x] T033 [P] [US1] Unit test: test_update_repository_summary in tests/test_graph_client.rs
- [x] T034 [P] [US1] Unit test: test_get_components in tests/test_graph_client.rs
- [x] T035 [P] [US1] Unit test: test_generate_repository_summary (mock) in tests/test_llm_integration.rs
- [x] T036 [P] [US1] Unit test: test_generate_component_summary (mock) in tests/test_llm_integration.rs
- [x] T037 [P] [US1] Unit test: test_llm_health_check in tests/test_llm_integration.rs
- [x] T038 [P] [US1] Unit test: test_llm_graceful_degradation in tests/test_llm_integration.rs
- [x] T039 [US1] Integration test: test_graph_builder_extract_entities in tests/test_graph_builder.rs
- [x] T040 [US1] Integration test: test_full_indexing_with_summaries in tests/integration_tests.rs

### Implementation for User Story 1

**Graph Client Operations**:
- [x] T041 [P] [US1] Implement create_repository() in src/graph/client.rs
- [x] T042 [P] [US1] Implement get_repository_by_path() in src/graph/client.rs
- [x] T043 [P] [US1] Implement update_repository_summary() in src/graph/client.rs
- [x] T044 [P] [US1] Implement create_component() in src/graph/client.rs
- [x] T045 [P] [US1] Implement get_components() in src/graph/client.rs
- [x] T046 [P] [US1] Implement update_component_summary() in src/graph/client.rs
- [x] T047 [P] [US1] Implement create_entity() in src/graph/client.rs
- [x] T048 [P] [US1] Implement create_entities_batch() with transaction batching in src/graph/client.rs
- [x] T049 [P] [US1] Implement get_entity() in src/graph/client.rs
- [x] T050 [P] [US1] Implement get_repository_stats() in src/graph/client.rs

**Entity Extraction**:
- [x] T051 [US1] Create GraphBuilder struct in src/graph/builder.rs
- [x] T052 [US1] Implement extract_entities_from_file() leveraging existing parser.rs in src/graph/builder.rs
- [x] T053 [US1] Implement map_ast_to_code_entity() to convert tree-sitter nodes to CodeEntity in src/graph/builder.rs
- [x] T054 [US1] Implement identify_component_from_path() to group entities into components in src/graph/builder.rs
- [x] T055 [US1] Implement build_from_index() orchestration method in src/graph/builder.rs

**LLM Summary Generation**:
- [x] T056 [P] [US1] Implement generate_repository_summary() in src/llm/client.rs
- [x] T057 [P] [US1] Implement generate_component_summary() in src/llm/client.rs
- [x] T058 [P] [US1] Implement generate_entity_summary() in src/llm/client.rs
- [x] T059 [P] [US1] Implement generate_summaries_batch() with rate limiting (semaphore, max 3 concurrent) in src/llm/client.rs
- [x] T060 [US1] Implement SummaryOrchestrator in src/llm/summarizer.rs
- [x] T061 [US1] Implement generate_summaries_parallel() with error handling and retries in src/llm/summarizer.rs

**Query Interface - Architecture Queries**:
- [x] T062 [US1] Create QueryParser struct in src/graph/query.rs
- [x] T063 [US1] Define QueryIntent enum (DescribeArchitecture, ListComponents, DescribeEntity, FindRelationships, AnalyzePattern) in src/graph/query.rs
- [x] T064 [US1] Implement parse() with architecture pattern regex in src/graph/query.rs
- [x] T065 [US1] Implement execute_architecture_query() to fetch repository summary in src/graph/query.rs
- [x] T066 [US1] Implement execute_list_components_query() to fetch all components in src/graph/query.rs

**CLI Integration**:
- [x] T067 [US1] Add --enable-graph flag to index command in src/main.rs
- [x] T068 [US1] Add query subcommand definition to CLI in src/main.rs
- [x] T069 [US1] Integrate GraphBuilder into Orchestrator.run() in src/orchestrator.rs
- [x] T070 [US1] Add graph building hook in Indexer after file parsing in src/indexer.rs
- [x] T071 [US1] Implement query command handler to execute natural language queries in src/main.rs
- [x] T072 [US1] Add progress indicators for summary generation using existing patterns in src/orchestrator.rs

**Checkpoint**: At this point, User Story 1 should be fully functional - can index repo, generate summaries, query architecture

---

## Phase 4: User Story 2 - Entity Relationship Discovery (Priority: P2)

**Goal**: Enable developers to query "what does Parser interact with?" and get detailed relationship information (dependencies, dependents, calls)

**Independent Test**: Query "what does Parser module interact with?", verify response includes entities Parser depends on and entities that depend on Parser

### Tests for User Story 2 ⚠️

- [ ] T073 [P] [US2] Unit test: test_create_dependency_relationship in tests/test_graph_client.rs
- [ ] T074 [P] [US2] Unit test: test_create_uses_relationship in tests/test_graph_client.rs
- [ ] T075 [P] [US2] Unit test: test_create_calls_relationship in tests/test_graph_client.rs
- [ ] T076 [P] [US2] Unit test: test_create_implements_relationship in tests/test_graph_client.rs
- [ ] T077 [P] [US2] Unit test: test_get_outgoing_relationships in tests/test_graph_client.rs
- [ ] T078 [P] [US2] Unit test: test_get_incoming_relationships in tests/test_graph_client.rs
- [ ] T079 [P] [US2] Unit test: test_get_all_relationships in tests/test_graph_client.rs
- [ ] T080 [P] [US2] Unit test: test_traverse_dependencies_multi_hop in tests/test_graph_client.rs
- [x] T081 [P] [US2] Unit test: test_query_parser_relationship_intent in tests/test_graph_client.rs
- [x] T082 [P] [US2] Unit test: test_fuzzy_entity_matching in tests/test_graph_client.rs
- [ ] T083 [US2] Integration test: test_extract_relationships_from_code in tests/test_graph_builder.rs (SKIPPED - requires AST analysis)
- [x] T084 [US2] Integration test: test_relationship_query_workflow in tests/integration_tests.rs

### Implementation for User Story 2

**PostgreSQL Graph Relationship Implementation**:
- [x] T085 [US2] Fix duplicate methods in PostgresKgClient (get_repository_by_path, update_repository_summary)
- [x] T086 [US2] Implement missing entity operations in PostgresKgClient (get_entity, find_entities_by_name, etc.)
- [x] T087 [US2] Implement relationship operations with recursive CTEs for graph traversal
- [x] T088 [US2] Add graph traversal methods for 'find all entities that depend on X'
- [x] T089 [US2] Add graph traversal methods for 'show relationships for entity Y'

**Graph Client Relationship Operations**:
- [x] T090 [P] [US2] Implement create_dependency() in src/kg/client.rs
- [x] T091 [P] [US2] Implement create_uses() in src/kg/client.rs
- [x] T092 [P] [US2] Implement create_contains() in src/kg/client.rs
- [x] T093 [P] [US2] Implement create_implements() in src/kg/client.rs
- [x] T094 [P] [US2] Implement create_calls() in src/kg/client.rs
- [x] T095 [US2] Implement get_outgoing_relationships() in src/kg/client.rs
- [x] T096 [US2] Implement get_incoming_relationships() in src/kg/client.rs
- [x] T097 [US2] Implement get_all_relationships() combining incoming + outgoing in src/kg/client.rs
- [x] T098 [US2] Implement traverse_dependencies() with recursive CTEs and max depth limit in src/kg/client.rs

**Entity Lookup**:
- [ ] T098 [P] [US2] Implement find_entities_by_name() exact match in src/graph/client.rs
- [x] T099 [US2] Implement EntityMatcher with fuzzy matching (strsim, threshold 0.85) in src/kg/query.rs
- [x] T100 [US2] Implement search_entities_by_name() fuzzy search - integrated into EntityMatcher
- [x] T101 [US2] Implement handle_ambiguous_entities() disambiguation logic - integrated into execute_relationship_query()

**Query Interface - Relationship Queries**:
- [x] T102 [US2] Add relationship pattern regex to QueryParser in src/kg/query.rs
- [x] T103 [US2] Implement extract_entity_from_query() helper - integrated into QueryParser
- [x] T104 [US2] Implement execute_relationship_query() with fuzzy entity matching in src/kg/query.rs
- [x] T105 [US2] Format relationship results for user display in src/main.rs

**Integration**:
- [ ] T106 [US2] Update build_from_index() to extract and create relationship edges in src/graph/builder.rs (SKIPPED - requires AST analysis)
- [x] T107 [US2] Add relationship queries to query command handler in src/main.rs

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently - can query architecture AND relationships

---

## Phase 5: User Story 3 - Component Purpose Discovery (Priority: P3)

**Goal**: Enable developers to query "what does the Database module do?" and get entity-level summaries without reading code

**Independent Test**: Query "what does the Database module do?", verify response provides meaningful summary of its responsibilities

### Tests for User Story 3 ⚠️

- [x] T108 [P] [US3] Unit test: test_update_entity_summary in tests/test_graph_client.rs
- [x] T109 [P] [US3] Unit test: test_get_entity_by_id in tests/test_graph_client.rs
- [x] T110 [P] [US3] Unit test: test_query_parser_entity_description_intent in tests/test_graph_queries.rs
- [x] T111 [P] [US3] Unit test: test_generate_entity_summary_with_code_snippet in tests/test_llm_integration.rs
- [x] T112 [US3] Integration test: test_entity_summary_generation_workflow in tests/integration_tests.rs

### Implementation for User Story 3

**Entity Summary Management**:
- [x] T113 [US3] Implement update_entity_summary() in src/graph/client.rs (ALREADY EXISTS)
- [x] T114 [US3] Implement get_entity() by ID in src/graph/client.rs (ALREADY EXISTS)
- [ ] T115 [US3] Extend generate_entity_summary() to truncate code snippets (50 lines max) in src/llm/client.rs
- [ ] T116 [US3] Implement entity summary generation in summarizer orchestration in src/llm/summarizer.rs

**Query Interface - Entity Description**:
- [x] T117 [US3] Add entity description pattern regex to QueryParser in src/graph/query.rs
- [x] T118 [US3] Implement execute_entity_description_query() in src/graph/query.rs
- [x] T119 [US3] Handle case where entity has no summary (fetch from DB or generate on-demand) in src/graph/query.rs

**Integration**:
- [ ] T120 [US3] Add entity summary generation to build_from_index() pipeline in src/graph/builder.rs
- [x] T121 [US3] Add entity description queries to query command handler in src/main.rs

**Checkpoint**: All three priority stories (P1, P2, P3) should now be independently functional

---

## Phase 6: User Story 4 - Cross-Cutting Concern Analysis (Priority: P4)

**Goal**: Enable developers to query "how is error handling implemented?" and get pattern analysis across the codebase

**Independent Test**: Query "how is error handling implemented?", verify response identifies pattern and relevant components

### Tests for User Story 4 ✅

- [x] T122 [P] [US4] Unit test: test_search_entities_by_name_pattern in tests/test_graph_client.rs
- [x] T123 [P] [US4] Unit test: test_execute_query_for_pattern_matching in tests/test_graph_client.rs
- [x] T124 [P] [US4] Unit test: test_generate_pattern_analysis_summary in tests/test_graph_client.rs (MockLlmClient)
- [x] T125 [P] [US4] Unit test: test_query_parser_pattern_analysis_intent in tests/test_graph_client.rs
- [x] T126 [US4] Integration test: test_pattern_analysis_workflow in tests/test_graph_client.rs

### Implementation for User Story 4 ✅

**Pattern Matching**:
- [x] T127 [US4] Pattern matching using EntityMatcher with Jaro-Winkler similarity in src/kg/query.rs
- [x] T128 [US4] Fuzzy entity search with configurable threshold (0.5 default) in src/kg/query.rs
- [x] T129 [US4] Implement analyze_pattern() LLM method with CodeEntity support in src/llm/client.rs

**Query Interface - Pattern Analysis**:
- [x] T130 [US4] Add PatternAnalysis variant and regex patterns to QueryParser in src/kg/query.rs
- [x] T131 [US4] Implement execute_pattern_analysis_query() with fuzzy matching in src/kg/query.rs
- [x] T132 [US4] Generate pattern summary with LLM using matched entities in src/kg/query.rs

**Integration**:
- [x] T133 [US4] Add pattern analysis queries to query command handler in src/main.rs

**Checkpoint**: All four user stories should now be fully functional

---

## Phase 7: Incremental Updates (Cross-Story Enhancement)

**Goal**: Update graph incrementally when files change instead of full re-index

**Independent Test**: Modify a file in indexed repo, re-index, verify only affected entities updated and graph remains consistent

### Tests for Incremental Updates ⚠️

- [ ] T134 [P] Unit test: test_get_entities_by_file in tests/test_graph_client.rs
- [ ] T135 [P] Unit test: test_delete_entity_cascade in tests/test_graph_client.rs
- [ ] T136 [P] Unit test: test_cascade_deletes_relationships in tests/test_graph_client.rs
- [ ] T137 Integration test: test_incremental_update_workflow in tests/test_incremental_updates.rs
- [ ] T138 Integration test: test_incremental_update_performance in tests/test_incremental_updates.rs

### Implementation for Incremental Updates

- [ ] T139 Implement get_entities_by_file() in src/graph/client.rs
- [ ] T140 Implement delete_entity_cascade() with relationship cleanup in src/graph/client.rs
- [ ] T141 Implement update_incremental() in src/graph/builder.rs
- [ ] T142 Implement find_affected_modules() to identify components needing summary regeneration in src/graph/builder.rs
- [ ] T143 Integrate incremental update into Indexer change detection logic in src/indexer.rs
- [ ] T144 Add incremental update tests with file modification scenarios in tests/test_incremental_updates.rs

**Checkpoint**: Incremental updates functional for all user stories

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

**Error Handling & Validation**:
- [ ] T145 [P] Implement to_user_message() for GraphError in src/graph/mod.rs
- [ ] T146 [P] Implement to_user_message() for LlmError in src/llm/mod.rs
- [ ] T147 [P] Add input validation for entity names (1-255 chars) in src/graph/client.rs
- [ ] T148 [P] Add validation for line_start <= line_end in src/graph/model.rs
- [ ] T149 [P] Add timeout to all graph queries (3 second limit) in src/graph/client.rs

**Performance Optimization**:
- [ ] T150 [P] Implement optimize() vacuum operation in src/graph/client.rs
- [ ] T151 [P] Add batch write optimization (100 entities per transaction) in src/graph/client.rs
- [ ] T152 [P] Add query result caching for repeated entity lookups in src/graph/query.rs
- [ ] T153 Implement connection pooling for parallel batch writes in src/graph/client.rs

**Observability**:
- [ ] T154 [P] Add tracing logs for graph operations in src/graph/client.rs
- [ ] T155 [P] Add tracing logs for LLM summary generation in src/llm/client.rs
- [ ] T156 [P] Add tracing logs for query parsing and execution in src/graph/query.rs
- [ ] T157 Add performance metrics tracking (operation latencies) in src/graph/client.rs

**Documentation**:
- [ ] T158 [P] Add doc comments to GraphClient trait methods in src/graph/client.rs
- [ ] T159 [P] Add doc comments to LlmClient trait methods in src/llm/client.rs
- [ ] T160 [P] Add usage examples to CLI help text in src/main.rs
- [ ] T161 Update README.md with knowledge graph feature documentation

**Testing**:
- [ ] T162 [P] Add ignored integration test with real Ollama (test_with_real_ollama) in tests/test_llm_integration.rs
- [ ] T163 [P] Add edge case tests (empty repo, large repo, circular dependencies) in tests/integration_tests.rs
- [ ] T164 Run cargo clippy and fix all warnings
- [ ] T165 Run cargo fmt and ensure code is formatted
- [ ] T166 Verify all tests pass with cargo test
- [ ] T167 Run quickstart.md validation steps

**Security**:
- [ ] T168 [P] Sanitize user query input to prevent SurrealQL injection in src/graph/query.rs
- [ ] T169 [P] Validate file paths to prevent directory traversal in src/graph/builder.rs

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational (Phase 2) - Repository architecture understanding (MVP)
- **User Story 2 (Phase 4)**: Depends on Foundational (Phase 2) - Can start in parallel with US1, builds on graph client
- **User Story 3 (Phase 5)**: Depends on Foundational (Phase 2) - Can start in parallel with US1/US2, extends summary generation
- **User Story 4 (Phase 6)**: Depends on Foundational (Phase 2) - Can start in parallel with other stories
- **Incremental Updates (Phase 7)**: Depends on US1 completion (needs base graph operations)
- **Polish (Phase 8)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Foundation only - independently testable
  - Delivers: Repository/component summaries, architecture queries, basic graph operations
  - MVP: This story alone provides valuable architecture understanding
  
- **User Story 2 (P2)**: Foundation only - independently testable
  - Delivers: Relationship discovery, dependency analysis, entity lookups
  - Can be developed in parallel with US1 by different developer
  - Integration point: Uses entity lookup from US1 but adds relationship traversal
  
- **User Story 3 (P3)**: Foundation only - independently testable
  - Delivers: Entity-level summaries, component purpose queries
  - Can be developed in parallel with US1/US2
  - Integration point: Extends summary generation from US1
  
- **User Story 4 (P4)**: Foundation only - independently testable
  - Delivers: Pattern analysis, cross-cutting concern queries
  - Can be developed in parallel with all other stories
  - Integration point: Uses entity search and LLM from US1

### Within Each User Story

- Tests MUST be written and FAIL before implementation (TDD requirement)
- Foundation (graph client + LLM client) before entity extraction
- Entity extraction before relationship extraction (US2)
- Core implementation before CLI integration
- Story complete before moving to next priority

### Parallel Opportunities

- **Setup (Phase 1)**: T003, T004 can run in parallel
- **Foundational (Phase 2)**: 
  - T011 can run in parallel with T008-T010
  - T021, T022 can run in parallel with T016-T020
  - T025-T028 (test fixtures) can all run in parallel
  
- **User Story 1 Tests**: T029-T038 can all run in parallel (all different test files)
- **User Story 1 Graph Operations**: T041-T050 can all run in parallel (implementing different methods)
- **User Story 1 LLM Operations**: T056-T059 can run in parallel
- **User Story 2 Tests**: T073-T082 can all run in parallel
- **User Story 2 Relationship Ops**: T089-T093 can run in parallel, T098-T099 can run in parallel
- **User Story 3 Tests**: T108-T111 can all run in parallel
- **User Story 4 Tests**: T122-T125 can all run in parallel
- **Incremental Tests**: T134-T136 can run in parallel
- **Polish Phase**: Most tasks marked [P] can run in parallel (T145-T169)

- **Story-Level Parallelism**: After Foundational (Phase 2) completes, all 4 user stories can be developed in parallel by different team members

---

## Parallel Example: User Story 1

```bash
# Write all tests for User Story 1 together (they MUST fail):
Task: "Unit test: test_create_and_get_repository in tests/test_graph_client.rs"
Task: "Unit test: test_create_and_get_component in tests/test_graph_client.rs"
Task: "Unit test: test_create_and_get_entity in tests/test_graph_client.rs"
Task: "Unit test: test_generate_repository_summary (mock) in tests/test_llm_integration.rs"

# Then implement Graph Client operations together:
Task: "Implement create_repository() in src/graph/client.rs"
Task: "Implement get_repository_by_path() in src/graph/client.rs"
Task: "Implement create_component() in src/graph/client.rs"
Task: "Implement get_components() in src/graph/client.rs"

# Then implement LLM operations together:
Task: "Implement generate_repository_summary() in src/llm/client.rs"
Task: "Implement generate_component_summary() in src/llm/client.rs"
Task: "Implement generate_entity_summary() in src/llm/client.rs"
```

---

## Parallel Example: Cross-Story Development

```bash
# After Foundational (Phase 2) completes:

Developer A: Focus on User Story 1 (T029-T072)
Developer B: Focus on User Story 2 (T073-T107)  
Developer C: Focus on User Story 3 (T108-T121)
Developer D: Focus on User Story 4 (T122-T133)

# Each developer can work independently and deliver their story
# All stories integrate through common GraphClient and LlmClient interfaces
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T007)
2. Complete Phase 2: Foundational (T008-T028) - CRITICAL foundation
3. Complete Phase 3: User Story 1 (T029-T072)
   - Write tests first (T029-T040) - ensure they FAIL
   - Implement graph operations (T041-T050)
   - Implement entity extraction (T051-T055)
   - Implement LLM summaries (T056-T061)
   - Implement architecture queries (T062-T066)
   - Integrate with CLI (T067-T072)
4. **STOP and VALIDATE**: Run all US1 tests, verify they pass
5. Test manually: `cudgel index --enable-graph test-repo && cudgel query "what is the architecture?"`
6. MVP ready for demo/deployment!

### Incremental Delivery

1. **Foundation** (Setup + Foundational) → T001-T028 complete → Foundation validated
2. **MVP** (+ User Story 1) → T029-T072 complete → Can query architecture, test independently → Deploy/Demo
3. **V2** (+ User Story 2) → T073-T107 complete → Can query relationships → Deploy/Demo
4. **V3** (+ User Story 3) → T108-T121 complete → Can query entity purposes → Deploy/Demo
5. **V4** (+ User Story 4) → T122-T133 complete → Can analyze patterns → Deploy/Demo
6. **Production** (+ Incremental + Polish) → T134-T169 complete → Full feature ready

### Parallel Team Strategy (4 Developers)

**Week 1**: All developers collaborate on Foundation (T001-T028)
**Week 2-3**: After foundation complete, parallel development:
- Dev A: User Story 1 (T029-T072) - MVP track
- Dev B: User Story 2 (T073-T107)
- Dev C: User Story 3 (T108-T121)
- Dev D: User Story 4 (T122-T133)

**Week 4**: Integration and polish
- All: Incremental updates (T134-T144)
- All: Polish tasks (T145-T169)

Each story completes independently, can be deployed separately.

---

## Task Summary

**Total Tasks**: 169
- **Setup**: 7 tasks (T001-T007)
- **Foundational**: 21 tasks (T008-T028)
- **User Story 1** (P1 - MVP): 44 tasks (T029-T072)
  - Tests: 12 tasks
  - Implementation: 32 tasks
- **User Story 2** (P2): 35 tasks (T073-T107)
  - Tests: 12 tasks
  - Implementation: 23 tasks
- **User Story 3** (P3): 14 tasks (T108-T121)
  - Tests: 5 tasks
  - Implementation: 9 tasks
- **User Story 4** (P4): 12 tasks (T122-T133)
  - Tests: 5 tasks
  - Implementation: 7 tasks
- **Incremental Updates**: 11 tasks (T134-T144)
  - Tests: 5 tasks
  - Implementation: 6 tasks
- **Polish**: 25 tasks (T145-T169)

**Parallel Opportunities**: 78 tasks marked [P] can run in parallel within their phase

**MVP Scope** (Minimum Viable Product):
- Phase 1: Setup (7 tasks)
- Phase 2: Foundational (21 tasks)
- Phase 3: User Story 1 only (44 tasks)
- **Total MVP**: 72 tasks to deliver core value

**Independent Test Criteria**:
- **US1**: Index sample repo, query architecture, verify summary returned
- **US2**: Query entity relationships, verify dependencies and dependents listed
- **US3**: Query entity purpose, verify meaningful summary without code
- **US4**: Query pattern, verify cross-cutting concern analysis

---

## Notes

- **TDD**: All tests MUST be written first and FAIL before implementation (constitution requirement)
- **[P] tasks**: Different files, no dependencies, can run in parallel
- **[Story] labels**: Map tasks to user stories for traceability
- **Each user story**: Independently completable and testable
- **Red-Green-Refactor**: Write test (RED) → Implement (GREEN) → Polish (REFACTOR)
- **Commit frequently**: After each task or logical group
- **Stop at checkpoints**: Validate story works independently before proceeding
- **Constitution compliance**: Zero clippy warnings, TDD, local-first architecture, performance targets
- **File paths**: All file paths are absolute from repository root
- **Dependencies**: Follow phase dependencies strictly - Foundation BLOCKS all user stories
