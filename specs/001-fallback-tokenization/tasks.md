# Tasks: Fallback Tokenization Strategy

**Feature**: 001-fallback-tokenization  
**Prerequisites**: spec.md, plan.md, research.md, data-model.md, contracts/tokenizer-strategy.md, quickstart.md

**Test Strategy**: TDD - Write failing tests first, then implement to make them pass

**Organization**: Tasks grouped by user story for independent implementation and testing

---

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story (US1, US2, US3)
- File paths are relative to repository root

---

## Phase 1: Setup & Dependencies

**Purpose**: Initialize project structure and add required dependencies

- [X] T001 [P] Add dependencies to Cargo.toml (xxhash-rust, rand_chacha, unicode-segmentation, ndarray for projection)
- [X] T002 [P] Update src/lib.rs module exports to expose embeddings module publicly
- [X] T003 [P] Add InvalidTokenizerStrategy variant to src/error.rs Error enum

**Time Estimate**: 15 minutes  
**Checkpoint**: Dependencies added, project compiles ✅

---

## Phase 2: Foundational (Trait & Abstraction Layer)

**Purpose**: Core trait definition and type-safe backend enum - BLOCKS all user story work

**⚠️ CRITICAL**: No user story implementation can begin until this phase is complete

### Tests First ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T004 [P] [Foundation] Create tests/test_tokenizer_trait.rs with trait contract tests (384D, determinism, normalization)
- [X] T005 [P] [Foundation] Create tests/test_fallback_tokenizer.rs with FallbackTokenizer unit tests (encoding, hashing, projection)

### Implementation

- [X] T006 [Foundation] Create src/embeddings/mod.rs with TokenizerStrategy trait definition
  - **Trait methods**: initialize, encode, validate, name
  - **Invariants**: 384D output, L2 normalized, deterministic, thread-safe
  - **File**: New file `src/embeddings/mod.rs`
  
- [X] T007 [Foundation] Define EmbedderBackend enum in src/embeddings/mod.rs
  - **Variants**: Onnx(OnnxTokenizer), Fallback(FallbackTokenizer)
  - **Methods**: name() for strategy identification, encode() dispatch
  - **File**: Same as T006

- [X] T008 [Foundation] Add module exports and re-exports to src/embeddings/mod.rs
  - **Exports**: TokenizerStrategy trait, EmbedderBackend enum
  - **Re-exports**: OnnxTokenizer, FallbackTokenizer (for tests)
  - **File**: Same as T006

**Time Estimate**: 45 minutes  
**Acceptance Criteria**: 
- Tests compile but fail (trait/struct not implemented yet) ✅
- Links to FR-005, FR-006 (consistent API, 384D embeddings) ✅

**Checkpoint**: Foundation tests written and failing - user story implementation can now begin ✅

---

## Phase 3: User Story 1 - Restricted Environment Usage (Priority: P1) 🎯 MVP

**Goal**: Enable indexing without ONNX models using fallback tokenizer

**Independent Test**: Set `CUDGEL_TOKENIZER_STRATEGY=fallback`, run indexing without models, verify success

**Acceptance Criteria**: FR-001, FR-006, FR-009, SC-001, SC-002, SC-004

### Implementation for User Story 1

- [X] T009 [US1] Create src/embeddings/fallback.rs with FallbackTokenizer struct skeleton
  - **Struct fields**: projection_matrix (Array2<f32>), hash_dimension (usize)
  - **File**: New file `src/embeddings/fallback.rs`
  - **Dependencies**: Uses ndarray, rand_chacha, xxhash_rust
  - **Time**: 30 minutes

- [X] T010 [US1] Implement TokenizerStrategy::initialize() for FallbackTokenizer
  - **Logic**: Generate 384×8192 random projection matrix with ChaCha8Rng (seed=42)
  - **Validation**: Matrix shape check, scaling factor (1/sqrt(8192))
  - **File**: `src/embeddings/fallback.rs`
  - **Depends on**: T009
  - **Time**: 20 minutes

- [X] T011 [US1] Implement tokenize_code() helper method for FallbackTokenizer
  - **Logic**: Split on whitespace, handle camelCase/snake_case, lowercase
  - **Uses**: unicode-segmentation for proper splitting
  - **File**: `src/embeddings/fallback.rs`
  - **Depends on**: T009
  - **Time**: 25 minutes

- [X] T012 [US1] Implement create_feature_vector() helper method for FallbackTokenizer
  - **Logic**: Hash tokens with xxh3_64, create 8192D sparse vector, signed hash values
  - **File**: `src/embeddings/fallback.rs`
  - **Depends on**: T009, T011
  - **Time**: 20 minutes

- [X] T013 [US1] Implement project_to_embedding() helper method for FallbackTokenizer
  - **Logic**: Matrix multiplication (projection_matrix × sparse_vector)
  - **Output**: 384D dense vector
  - **File**: `src/embeddings/fallback.rs`
  - **Depends on**: T009, T010
  - **Time**: 15 minutes

- [X] T014 [US1] Implement normalize() helper method for FallbackTokenizer
  - **Logic**: L2 normalization (divide by Euclidean norm)
  - **Invariant**: Output norm ≈ 1.0
  - **File**: `src/embeddings/fallback.rs`
  - **Depends on**: T009
  - **Time**: 10 minutes

- [X] T015 [US1] Implement TokenizerStrategy::encode() for FallbackTokenizer
  - **Logic**: Pipeline - tokenize → hash → project → normalize
  - **File**: `src/embeddings/fallback.rs`
  - **Depends on**: T011, T012, T013, T014
  - **Time**: 15 minutes

- [X] T016 [US1] Implement TokenizerStrategy::validate() and name() for FallbackTokenizer
  - **validate()**: Check projection matrix shape (384, 8192)
  - **name()**: Return "fallback"
  - **File**: `src/embeddings/fallback.rs`
  - **Depends on**: T009
  - **Time**: 10 minutes

- [X] T017 [US1] Run tests from Phase 2 - verify they now PASS
  - **Command**: `cargo test test_fallback`
  - **Expected**: GREEN (all trait contract tests pass)
  - **Time**: 5 minutes

**Time Estimate**: 2.5 hours  
**Checkpoint**: Fallback tokenizer fully functional and tested independently ✅

---

## Phase 4: User Story 2 - Graceful Degradation (Priority: P2)

**Goal**: Refactor ONNX code to use trait abstraction and enable strategy switching

**Independent Test**: Run with models (ONNX used) and without models + fallback config (fallback used)

**Acceptance Criteria**: FR-002, FR-003, FR-004, FR-008, SC-003

### Tests First ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before refactoring**

- [ ] T018 [P] [US2] Add tests/test_onnx_tokenizer.rs with OnnxTokenizer trait conformance tests
- [ ] T019 [P] [US2] Add tests/test_embedder_backend.rs with EmbedderBackend enum dispatch tests

### Refactoring for User Story 2

- [ ] T020 [US2] Create src/embeddings/onnx.rs and move existing ONNX code from src/embeddings.rs
  - **Struct**: OnnxTokenizer { session: Mutex<Session>, tokenizer: Tokenizer }
  - **File**: New file `src/embeddings/onnx.rs`
  - **Source**: Extract from existing `src/embeddings.rs`
  - **Time**: 45 minutes

- [ ] T021 [US2] Implement TokenizerStrategy trait for OnnxTokenizer
  - **Methods**: initialize(), encode(), validate(), name()
  - **Logic**: Copy from existing EmbeddingGenerator implementation
  - **File**: `src/embeddings/onnx.rs`
  - **Depends on**: T020
  - **Time**: 30 minutes

- [ ] T022 [US2] Add strategy field to EmbeddingConfig in src/config.rs
  - **Field**: `pub strategy: String`
  - **Default**: Read from `CUDGEL_TOKENIZER_STRATEGY` env var, default "onnx"
  - **Validation**: Lowercase conversion
  - **File**: `src/config.rs`
  - **Time**: 15 minutes

- [ ] T023 [US2] Add strategy validation to Config::validate() in src/config.rs
  - **Valid values**: "onnx", "fallback"
  - **Error message**: Include syntax, environment variable example, strategy descriptions
  - **Links to**: FR-004, FR-010, SC-003
  - **File**: `src/config.rs`
  - **Depends on**: T022
  - **Time**: 20 minutes

- [ ] T024 [US2] Refactor EmbeddingGenerator in src/embeddings.rs to use EmbedderBackend
  - **Field**: Replace ONNX-specific fields with `backend: EmbedderBackend`
  - **Factory logic**: Match on config.strategy, instantiate appropriate backend
  - **Error handling**: Return InvalidTokenizerStrategy for unknown strategy
  - **File**: `src/embeddings.rs`
  - **Depends on**: T020, T021, T022
  - **Time**: 40 minutes

- [ ] T025 [US2] Add initialization logging to EmbeddingGenerator::new()
  - **Log message**: "Initialized embedding generator with 'X' strategy"
  - **Level**: Info
  - **Links to**: FR-007
  - **File**: `src/embeddings.rs`
  - **Depends on**: T024
  - **Time**: 10 minutes

- [ ] T026 [US2] Run existing ONNX tests - verify no regressions
  - **Command**: `cargo test test_embedding` (existing tests)
  - **Expected**: GREEN (ONNX still works)
  - **Time**: 5 minutes

- [ ] T027 [US2] Run new trait conformance tests - verify they now PASS
  - **Command**: `cargo test test_onnx_tokenizer test_embedder_backend`
  - **Expected**: GREEN
  - **Time**: 5 minutes

**Time Estimate**: 3 hours  
**Checkpoint**: Both ONNX and fallback strategies work independently, strategy switching functional

---

## Phase 5: User Story 3 - Configuration Transparency (Priority: P3)

**Goal**: Add logging, diagnostics, and configuration verification

**Independent Test**: Check logs for strategy confirmation, test verbose mode output

**Acceptance Criteria**: FR-007, SC-003

### Tests First ⚠️

- [ ] T028 [P] [US3] Add tests/test_strategy_logging.rs with log output verification tests

### Implementation for User Story 3

- [ ] T029 [US3] Add debug logging to strategy initialization
  - **Locations**: FallbackTokenizer::initialize(), OnnxTokenizer::initialize()
  - **Message**: "Initializing [strategy] tokenizer..."
  - **Level**: Debug
  - **Files**: `src/embeddings/fallback.rs`, `src/embeddings/onnx.rs`
  - **Time**: 15 minutes

- [ ] T030 [US3] Add validation logging to strategy validation
  - **Locations**: FallbackTokenizer::validate(), OnnxTokenizer::validate()
  - **Message**: "[Strategy] tokenizer validation passed"
  - **Level**: Debug
  - **Files**: `src/embeddings/fallback.rs`, `src/embeddings/onnx.rs`
  - **Time**: 15 minutes

- [ ] T031 [US3] Add config logging to Config::validate()
  - **Message**: "Using tokenization strategy: [strategy]"
  - **Level**: Info
  - **File**: `src/config.rs`
  - **Time**: 10 minutes

- [ ] T032 [US3] Update error messages to include active strategy context
  - **Locations**: ONNX model not found error, invalid strategy error
  - **Enhancement**: Add "Current strategy: [X]" to error context
  - **Files**: `src/embeddings/onnx.rs`, `src/config.rs`
  - **Time**: 15 minutes

**Time Estimate**: 1 hour  
**Checkpoint**: All user stories complete with full observability

---

## Phase 6: Integration Testing & Validation

**Purpose**: End-to-end tests spanning multiple user stories

### Integration Tests

- [ ] T033 [P] [Integration] Add test_strategy_switching_via_env_var() to tests/integration_tests.rs
  - **Test**: Set env var to "fallback", verify EmbeddingGenerator uses FallbackTokenizer
  - **Test**: Set env var to "onnx", verify EmbeddingGenerator uses OnnxTokenizer (if available)
  - **Links to**: FR-001, FR-008
  - **Time**: 20 minutes

- [ ] T034 [P] [Integration] Add test_invalid_strategy_rejected() to tests/integration_tests.rs
  - **Test**: Set env var to "invalid", verify Config::validate() returns error
  - **Links to**: FR-010, SC-003
  - **Time**: 15 minutes

- [ ] T035 [P] [Integration] Add test_fallback_quality_baseline() to tests/integration_tests.rs
  - **Test**: Encode similar code snippets, verify cosine similarity > 0.7
  - **Links to**: SC-005
  - **Time**: 25 minutes

- [ ] T036 [P] [Integration] Add test_onnx_not_found_with_fallback() to tests/integration_tests.rs
  - **Test**: Remove ONNX models, set fallback env var, verify successful initialization
  - **Links to**: FR-003, SC-001, SC-004
  - **Time**: 20 minutes

- [ ] T037 [P] [Integration] Add test_onnx_not_found_without_fallback() to tests/integration_tests.rs
  - **Test**: Remove ONNX models, no fallback config, verify helpful error message
  - **Links to**: FR-004, SC-003
  - **Time**: 15 minutes

**Time Estimate**: 1.5 hours  
**Checkpoint**: All integration tests passing

---

## Phase 7: Manual Quality Validation

**Purpose**: Real-world testing with actual codebase indexing

- [ ] T038 Index cudgel codebase with fallback strategy
  - **Command**: `CUDGEL_TOKENIZER_STRATEGY=fallback cudgel index .`
  - **Verify**: No errors, all files indexed
  - **Links to**: SC-001, SC-004
  - **Time**: 10 minutes

- [ ] T039 Run benchmark queries with fallback strategy
  - **Queries**: "embedding generation", "database connection", "error handling", "tokenization", "vector search"
  - **Verify**: Each query returns at least 1 relevant result in top 3
  - **Links to**: SC-005
  - **Time**: 15 minutes

- [ ] T040 Measure fallback initialization time
  - **Command**: `time CUDGEL_TOKENIZER_STRATEGY=fallback cudgel index /path/to/small/repo`
  - **Target**: <5 seconds total
  - **Links to**: SC-002
  - **Time**: 5 minutes

- [ ] T041 Compare fallback vs ONNX results (if ONNX available)
  - **Method**: Index with fallback, run queries, save results; index with ONNX, run same queries, compare
  - **Target**: Fallback finds ≥50% of ONNX results
  - **Links to**: SC-005
  - **Time**: 20 minutes

- [ ] T042 Test error messages with invalid configuration
  - **Test**: Set `CUDGEL_TOKENIZER_STRATEGY=xyz`
  - **Verify**: Error message includes valid options and environment variable syntax
  - **Links to**: SC-003
  - **Time**: 5 minutes

**Time Estimate**: 1 hour  
**Checkpoint**: Manual validation complete, all success criteria met

---

## Phase 8: Polish & Documentation

**Purpose**: Code quality, documentation, and final cleanup

- [ ] T043 [P] Run cargo clippy --all-targets -- -D warnings
  - **Target**: Zero warnings (project policy)
  - **Time**: 15 minutes

- [ ] T044 [P] Run cargo fmt
  - **Target**: All code formatted consistently
  - **Time**: 2 minutes

- [ ] T045 [P] Add module documentation to src/embeddings/mod.rs
  - **Content**: Trait explanation, strategy comparison table, usage examples
  - **Time**: 20 minutes

- [ ] T046 [P] Add struct documentation to src/embeddings/fallback.rs
  - **Content**: Algorithm description, performance characteristics, quality trade-offs
  - **Time**: 20 minutes

- [ ] T047 [P] Add struct documentation to src/embeddings/onnx.rs
  - **Content**: ONNX model requirements, initialization details
  - **Time**: 15 minutes

- [ ] T048 [P] Update README.md with fallback tokenization usage examples
  - **Sections**: Configuration, environment variable, restricted environments
  - **Time**: 25 minutes

- [ ] T049 [P] Update AGENTS.md with fallback tokenization context
  - **Content**: New dependencies, module structure, strategy pattern
  - **Time**: 10 minutes

- [ ] T050 Verify quickstart.md validation checklist complete
  - **Check**: All file modification checklist items completed
  - **Time**: 10 minutes

**Time Estimate**: 2 hours  
**Checkpoint**: All documentation complete, code ready for review

---

## Dependencies & Execution Order

### Phase Dependencies

```
Phase 1 (Setup)
    ↓
Phase 2 (Foundation) ← BLOCKS all user stories
    ↓
    ├─→ Phase 3 (US1: Fallback Implementation) 🎯 MVP
    ├─→ Phase 4 (US2: ONNX Refactor + Strategy Switching)
    └─→ Phase 5 (US3: Logging & Transparency)
         ↓
Phase 6 (Integration Tests)
    ↓
Phase 7 (Manual Validation)
    ↓
Phase 8 (Polish & Docs)
```

### Critical Path (MVP)

For minimum viable product (restricted environment support):

1. Phase 1: Setup → 15 min
2. Phase 2: Foundation (T004-T008) → 45 min
3. Phase 3: US1 Complete (T009-T017) → 2.5 hrs
4. Phase 4: US2 Partial (T022-T025 only - config + factory) → 1.5 hrs
5. Phase 6: Integration Tests (T033, T036 only) → 40 min
6. Phase 7: Manual Validation (T038-T040) → 30 min

**Total MVP Time**: ~5.5 hours

### Parallel Opportunities

**Phase 1**: All tasks (T001-T003) can run in parallel  
**Phase 2 Tests**: T004 and T005 can run in parallel  
**Phase 4 Tests**: T018 and T019 can run in parallel  
**Phase 6 Integration**: All tasks (T033-T037) can run in parallel  
**Phase 8 Documentation**: All tasks (T043-T049) can run in parallel

**Team Strategy**: With 2 developers after Phase 2:
- Developer A: Phase 3 (US1 - Fallback Implementation)
- Developer B: Phase 4 (US2 - ONNX Refactor)
- Both: Phase 5 (US3 - Logging) can split by file

---

## Implementation Strategy

### Option 1: MVP First (Fastest Path to Value)

1. ✅ Phase 1: Setup (15 min)
2. ✅ Phase 2: Foundation (45 min)
3. ✅ Phase 3: US1 (2.5 hrs) ← **STOP HERE for MVP**
4. ✅ T022-T025 from Phase 4 (config + factory) (1.5 hrs)
5. ✅ Test with `CUDGEL_TOKENIZER_STRATEGY=fallback`
6. Deploy/demo restricted environment support

**MVP Delivers**: FR-001, FR-006, FR-009, SC-001, SC-004 (core value proposition)

### Option 2: Incremental Delivery

1. Complete Phase 1-3 → MVP deployed
2. Add Phase 4 → Strategy switching deployed  
3. Add Phase 5 → Full transparency deployed
4. Each phase adds value without breaking previous work

### Option 3: Full Feature (Recommended)

1. Phase 1 → Setup complete (15 min)
2. Phase 2 → Foundation ready (45 min) ← **Checkpoint: Can start parallel work**
3. Phase 3-5 → All user stories (6.5 hrs total, or 3 hrs with 2 devs)
4. Phase 6 → Integration tests (1.5 hrs)
5. Phase 7 → Manual validation (1 hr)
6. Phase 8 → Polish (2 hrs)

**Total Time**: ~12 hours solo, ~7-8 hours with pair programming

---

## Notes

- **TDD Discipline**: Tests MUST be written first and MUST fail before implementation
- **[P] Marking**: Tasks with [P] can run in parallel (different files, no shared dependencies)
- **[Story] Labels**: Map tasks to user stories for requirement traceability
- **Time Estimates**: Based on single developer, experienced with Rust + project codebase
- **Commits**: Commit after each phase or logical task group
- **Checkpoints**: Stop and validate at each checkpoint before proceeding
- **Avoid**: Same-file conflicts when parallelizing, cross-story dependencies that break independence

---

## Success Verification Checklist

After implementation, verify all success criteria:

- [ ] **SC-001**: Index in restricted env → Set fallback env var, run index, no errors
- [ ] **SC-002**: Init <5s → Time FallbackTokenizer::initialize(), verify <5s
- [ ] **SC-003**: Actionable errors → Test invalid strategy, verify error includes syntax + options
- [ ] **SC-004**: 100% success → Index cudgel with fallback, all files succeed
- [ ] **SC-005**: Meaningful results → Manual query validation, ≥50% quality vs ONNX

All requirements (FR-001 through FR-010) traced through task descriptions ✅
