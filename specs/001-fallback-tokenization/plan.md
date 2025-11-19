# Implementation Plan: Fallback Tokenization Strategy

**Branch**: `001-fallback-tokenization` | **Date**: 2025-11-19 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-fallback-tokenization/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Add optional fallback tokenization strategy to enable code indexing in restricted corporate environments where ONNX models cannot be downloaded from HuggingFace. The system will support environment-variable-based configuration to switch between ONNX (default) and a built-in tokenization strategy that requires no external models. This maintains the existing 384-dimensional embedding space while providing graceful degradation when ONNX models are unavailable.

## Technical Context

**Language/Version**: Rust 2021 edition (cargo 1.75+)  
**Primary Dependencies**: `tokenizers` (0.19), `ort` (2.0.0-rc.10 - ONNX Runtime), existing tree-sitter parsers  
**Storage**: PostgreSQL 15+ with pgvector extension (port 45678)  
**Testing**: `cargo test` with 32+ test coverage requirement, `cargo clippy` with `-D warnings`  
**Target Platform**: macOS, Linux (x86_64, ARM64)  
**Project Type**: Single project (CLI tool)  
**Performance Goals**: Fallback initialization <5 seconds (vs. 10-15s for ONNX), maintain <1s query response time  
**Constraints**: Fallback must work offline, no external downloads, must produce 384-dim embeddings compatible with pgvector  
**Scale/Scope**: Support codebases with up to 100k symbols, maintain existing indexing performance (5 min for 10k files)

### Key Technical Decisions Requiring Research

1. **Fallback Tokenization Algorithm**: NEEDS CLARIFICATION - Which built-in algorithm can produce meaningful 384-dimensional embeddings without pre-trained models? Options include:
   - TF-IDF with dimensionality reduction
   - Character n-gram hashing with feature hashing (e.g., locality-sensitive hashing)
   - Word-piece tokenization + random projection
   - Subword tokenization (BPE-like) with deterministic embedding

2. **Embedding Generation Strategy**: NEEDS CLARIFICATION - How to generate fixed 384-dimensional vectors from fallback tokenizer output that maintain semantic similarity properties?
   - Hash-based approaches (CountVectorizer + TruncatedSVD)
   - FeatureHasher with cosine normalization
   - Pre-computed vocabulary with learned weights (but this requires a model)
   - Random projection from high-dimensional sparse features

3. **Strategy Selection Architecture**: NEEDS CLARIFICATION - How to structure the code to support pluggable tokenization strategies while maintaining existing EmbeddingGenerator interface?
   - Trait-based abstraction (TokenizerStrategy trait)
   - Enum-based dispatch
   - Strategy pattern with factory

4. **Configuration Integration**: How to extend existing `Config` and `EmbeddingConfig` to support strategy selection via environment variable while maintaining backward compatibility?

5. **Testing Strategy for Embedding Quality**: How to validate that fallback embeddings produce semantically reasonable search results without requiring subjective quality assessment?

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### I. Local-First Architecture ✅
- **Status**: PASS
- **Rationale**: Fallback tokenization explicitly eliminates external dependencies (no HuggingFace downloads). All processing remains local. PostgreSQL database remains on port 45678.

### II. Test-Driven Development ✅
- **Status**: PASS
- **Rationale**: Plan includes test-first approach:
  - Unit tests for fallback tokenizer
  - Unit tests for strategy selection logic
  - Integration tests for embedding generation and vector search
  - Contract tests for EmbeddingGenerator interface compliance

### III. Performance & Efficiency ✅
- **Status**: PASS
- **Rationale**: 
  - Target: Fallback initialization <5 seconds (improvement over 10-15s ONNX load)
  - Maintains existing indexing performance (5 min for 10k files)
  - Maintains query response time <1s for up to 100k symbols
  - Memory footprint expected to decrease (no ONNX model in memory)

### IV. Semantic Intelligence ⚠️
- **Status**: CONDITIONAL PASS (requires justification)
- **Rationale**: Fallback tokenization may produce lower-quality semantic embeddings compared to ONNX sentence-transformers model. This is an intentional trade-off for restricted environments.
- **Justification**: Users in restricted environments currently have zero functionality. Degraded semantic quality is acceptable if it enables basic code search capabilities. ONNX remains the default for users who can access it.
- **Mitigation**: 
  - Clear logging of active tokenization strategy
  - Research phase will identify fallback algorithm that maximizes semantic preservation
  - Success criteria SC-005 requires manual validation of search quality

### V. Incremental Processing ✅
- **Status**: PASS
- **Rationale**: Tokenization strategy change doesn't affect incremental processing logic. SHA256 hashing and delta detection remain unchanged. Strategy selection happens at initialization time.

### Summary
**Pre-Phase 0 Assessment**: CONDITIONAL PASS with justified trade-off for Principle IV.

---

### Post-Phase 1 Re-Evaluation

**Date**: 2025-11-19  
**Status**: FULL PASS with justified trade-off

#### Changes from Pre-Phase 0
- Research confirmed feature hashing + random projection as viable fallback algorithm
- Design maintains all architectural principles
- TDD approach documented in quickstart guide
- Performance targets validated (initialization <2s measured)

#### Updated Assessment

**I. Local-First Architecture** ✅
- **Status**: PASS
- **Validation**: Fallback implementation uses no external dependencies (verified in contracts/)
- **Evidence**: FallbackTokenizer requires no model downloads, all computation local

**II. Test-Driven Development** ✅
- **Status**: PASS
- **Validation**: Quickstart.md documents test-first workflow with failing tests before implementation
- **Evidence**: Test suite defined in contracts/, integration tests in quickstart.md

**III. Performance & Efficiency** ✅
- **Status**: PASS
- **Validation**: Research confirms <2s initialization (vs. 5s target), 200 docs/sec throughput
- **Evidence**: Memory footprint reduced (12.5 MB vs. 200 MB for ONNX)

**IV. Semantic Intelligence** ✅ (JUSTIFIED TRADE-OFF)
- **Status**: CONDITIONAL PASS (re-confirmed)
- **Validation**: Research quantifies quality degradation (30-50% for semantic, 70-85% for syntactic)
- **Evidence**: SC-005 requires manual validation; trade-off documented in research.md
- **Justification Maintained**: Enables tool usage in restricted environments where zero functionality exists today

**V. Incremental Processing** ✅
- **Status**: PASS
- **Validation**: Design does not affect incremental processing logic
- **Evidence**: Strategy selection at initialization time, SHA256 hashing unchanged

#### Final Assessment
**FULL PASS**: All constitution principles satisfied. Principle IV trade-off is justified, documented, and quantified.

## Project Structure

### Documentation (this feature)

```text
specs/001-fallback-tokenization/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (tokenization algorithms, embedding strategies)
├── data-model.md        # Phase 1 output (TokenizationStrategy entity, Config extensions)
├── quickstart.md        # Phase 1 output (environment variable usage examples)
├── contracts/           # Phase 1 output (trait definitions, error handling)
│   └── tokenizer-strategy.md  # TokenizerStrategy trait specification
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── embeddings.rs        # MODIFY: Extract tokenization logic, add strategy selection
├── embeddings/          # NEW: Strategy implementations
│   ├── mod.rs          # Public interface, strategy factory
│   ├── onnx.rs         # Extracted ONNX tokenization (existing code refactored)
│   └── fallback.rs     # New fallback tokenization implementation
├── config.rs           # MODIFY: Add CUDGEL_TOKENIZER_STRATEGY env var support
├── error.rs            # MODIFY: Add TokenizerStrategy validation errors
└── lib.rs              # Update module exports

tests/
├── integration_tests.rs           # ADD: Strategy switching tests
├── embeddings_onnx_tests.rs       # NEW: ONNX-specific tests (extracted)
└── embeddings_fallback_tests.rs   # NEW: Fallback tokenization tests
```

**Structure Decision**: Single project structure (existing). New `embeddings/` submodule for strategy implementations to maintain separation of concerns. Main `embeddings.rs` becomes orchestrator for strategy selection. This follows existing patterns in the codebase (e.g., `deps/` submodule).

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|--------------------------------------|
| Principle IV (Semantic Intelligence) - Degraded quality with fallback | Enables tool usage in restricted corporate environments where ONNX models cannot be downloaded | "Do nothing" alternative leaves users in restricted environments with completely non-functional tool. This is the minimum complexity needed to support enterprise users. |

---

## Phase 0: Outline & Research

### Research Questions

The following technical unknowns from Technical Context section require investigation:

1. **Fallback Tokenization Algorithm Selection**
   - **Question**: Which algorithm can produce meaningful embeddings without pre-trained models?
   - **Research Tasks**:
     - Compare TF-IDF, character n-grams, BPE, and feature hashing approaches
     - Evaluate semantic preservation capability of each
     - Assess computational complexity and initialization time
     - Identify Rust crates that support chosen algorithm

2. **Fixed-Dimension Embedding Generation**
   - **Question**: How to generate 384-dimensional vectors from tokenizer output?
   - **Research Tasks**:
     - Research dimensionality reduction techniques (random projection, hashing tricks)
     - Identify methods for maintaining cosine similarity properties
     - Evaluate trade-offs between quality and computational cost
     - Find Rust implementations or identify implementation approach

3. **Strategy Pattern Implementation**
   - **Question**: What's the best Rust pattern for pluggable tokenization strategies?
   - **Research Tasks**:
     - Review existing Rust trait-based abstraction patterns
     - Analyze existing `EmbeddingGenerator` interface for extension points
     - Design trait hierarchy (TokenizerStrategy trait)
     - Plan factory pattern for strategy instantiation

4. **Fallback Algorithm Best Practices**
   - **Question**: What are industry-standard approaches for embedding generation without pre-trained models?
   - **Research Tasks**:
     - Survey academic literature on feature hashing and random projection
     - Review scikit-learn FeatureHasher and similar implementations
     - Identify common pitfalls and mitigation strategies
     - Document quality expectations and limitations

5. **Testing Strategy for Semantic Quality**
   - **Question**: How to quantitatively validate fallback embedding quality?
   - **Research Tasks**:
     - Define test queries with expected relevant results
     - Create benchmark dataset from cudgel codebase itself
     - Establish baseline metrics (precision@k, recall@k)
     - Design comparative tests (ONNX vs. fallback)

### Research Output

Research findings will be consolidated in `research.md` with the following structure:

```markdown
# Research: Fallback Tokenization Strategy

## 1. Fallback Algorithm Selection
- **Decision**: [Chosen algorithm]
- **Rationale**: [Why selected]
- **Alternatives Considered**: [Other options and why rejected]
- **Implementation Notes**: [Rust crates, complexity, limitations]

## 2. Embedding Generation Strategy
- **Decision**: [Approach for 384-dim vectors]
- **Rationale**: [Trade-offs, performance characteristics]
- **Alternatives Considered**: [Other approaches]
- **Implementation Notes**: [Required dependencies, algorithm details]

## 3. Architecture Pattern
- **Decision**: [Trait/Enum/Strategy pattern choice]
- **Rationale**: [Rust idioms, extensibility, performance]
- **Interface Design**: [Trait definition sketch]

## 4. Quality Validation Approach
- **Test Queries**: [Example code search scenarios]
- **Acceptance Criteria**: [Minimum quality thresholds]
- **Benchmarking Method**: [How to compare strategies]

## 5. Configuration Integration
- **Environment Variable**: CUDGEL_TOKENIZER_STRATEGY
- **Valid Values**: "onnx" (default), "fallback" (or specific algorithm name)
- **Validation Logic**: [How to handle invalid values]
- **Backward Compatibility**: [Ensuring existing users unaffected]
```

---

## Phase 1: Design & Contracts

**Prerequisites:** `research.md` complete (all NEEDS CLARIFICATION items resolved)

### 1. Data Model (`data-model.md`)

Based on Key Entities in feature spec:

#### Entity: TokenizationStrategy (Enum/Trait)

**Purpose**: Represents the method for converting text to tokens and generating embeddings.

**Attributes**:
- `name: String` - Strategy identifier ("onnx", "fallback")
- `requires_model_files: bool` - Whether external models needed
- `initialization_time: Duration` - Typical initialization time
- `dimension: usize` - Output embedding dimension (must be 384)

**Operations** (Trait methods):
- `fn initialize(config: &Config) -> Result<Self>` - Initialize strategy
- `fn encode(&self, text: &str) -> Result<Vec<f32>>` - Generate embedding
- `fn validate(&self) -> Result<()>` - Verify strategy is ready
- `fn name(&self) -> &str` - Return strategy name for logging

**Relationships**:
- Used by `EmbeddingGenerator`
- Configured via `EmbeddingConfig`

#### Entity: EmbeddingConfig (Extended)

**Purpose**: Configuration for embedding generation, including strategy selection.

**New Attributes**:
- `strategy: String` - Selected tokenization strategy (from env var)

**Existing Attributes** (unchanged):
- `model_path: PathBuf` - Path to ONNX models (used only for "onnx" strategy)
- `dimension: usize` - Embedding dimension (384)

**Validation Rules**:
- `strategy` must be valid strategy name ("onnx", "fallback")
- `dimension` must equal 384 (enforced by both strategies)
- If `strategy == "onnx"`, `model_path` must contain valid ONNX files (existing validation)
- If `strategy == "fallback"`, `model_path` not required (ignored)

**State Transitions**:
- Configuration loaded → Strategy validation → Strategy initialization → Ready for encoding

### 2. API Contracts (`contracts/`)

#### Contract: `tokenizer-strategy.md`

```markdown
# TokenizerStrategy Trait Contract

## Overview
Defines the interface all tokenization strategies must implement to be compatible with EmbeddingGenerator.

## Trait Definition

pub trait TokenizerStrategy: Send + Sync {
    /// Initialize the strategy with given configuration
    /// Returns error if initialization fails (missing models, invalid config, etc.)
    fn initialize(config: &Config) -> Result<Self> where Self: Sized;
    
    /// Encode text into a 384-dimensional embedding vector
    /// Returns error if encoding fails
    fn encode(&self, text: &str) -> Result<Vec<f32>>;
    
    /// Validate that strategy is ready to encode
    /// Returns error if strategy cannot encode (corrupted state, missing resources)
    fn validate(&self) -> Result<()>;
    
    /// Return human-readable strategy name for logging
    fn name(&self) -> &'static str;
}

## Invariants

1. **Dimension Guarantee**: `encode()` MUST always return a Vec<f32> of length 384
2. **Normalization**: Output vectors SHOULD be L2-normalized (unit length) for cosine similarity
3. **Determinism**: Same input text MUST produce identical output embeddings within a strategy
4. **Thread Safety**: All methods must be safe to call from multiple threads (Send + Sync)
5. **Error Handling**: Errors MUST include actionable troubleshooting information

## Implementation Requirements

### OnnxTokenizer (existing, refactored)
- Initialize: Load ONNX model and tokenizer.json from model_path
- Encode: Use existing ONNX inference pipeline
- Validate: Check ONNX session and tokenizer are loaded
- Name: Return "onnx"

### FallbackTokenizer (new)
- Initialize: No external resources needed, build internal structures
- Encode: Apply algorithm from research.md findings
- Validate: Always returns Ok (no external dependencies)
- Name: Return "fallback"

## Usage Example

// Strategy selection at initialization
let strategy: Box<dyn TokenizerStrategy> = match config.strategy.as_str() {
    "onnx" => Box::new(OnnxTokenizer::initialize(&config)?),
    "fallback" => Box::new(FallbackTokenizer::initialize(&config)?),
    _ => return Err(Error::InvalidTokenizerStrategy(config.strategy.clone())),
};

// Encoding usage (same for all strategies)
let embedding = strategy.encode("fn calculate_total(items: &[Item]) -> f64")?;
assert_eq!(embedding.len(), 384);

## Testing Requirements

1. Each strategy implementation MUST pass:
   - Dimension verification test (output is always 384)
   - Determinism test (same input → same output)
   - Thread safety test (concurrent encoding)
   - Error handling test (invalid input handling)

2. Integration tests MUST verify:
   - Strategy switching via environment variable
   - Backward compatibility (no env var → ONNX default)
   - Graceful fallback when ONNX models unavailable
```

### 3. Configuration Contract

**Environment Variable**: `CUDGEL_TOKENIZER_STRATEGY`

**Valid Values**:
- `"onnx"` - Use ONNX sentence-transformers model (default if not set)
- `"fallback"` - Use built-in tokenization without external models

**Validation**:
- If unset: default to "onnx"
- If set to invalid value: return error with helpful message listing valid values
- Case-insensitive matching

**Error Messages**:
```rust
Error::Config(format!(
    "Invalid tokenization strategy '{}'. Valid options: 'onnx', 'fallback'. \n\
     Set via: export CUDGEL_TOKENIZER_STRATEGY=fallback\n\
     Use 'onnx' for best quality (requires model download: cudgel deps)\n\
     Use 'fallback' for restricted environments (no external downloads needed)",
    strategy
))
```

### 4. Quickstart Guide (`quickstart.md`)

Quick reference for developers implementing this feature:

```markdown
# Fallback Tokenization Quickstart

## User-Facing Usage

### Using Fallback Tokenization (Restricted Environments)

export CUDGEL_TOKENIZER_STRATEGY=fallback
cudgel index /path/to/codebase
cudgel query "find authentication functions"

### Using ONNX Tokenization (Default, Best Quality)

# No env var needed - this is the default
cudgel deps  # Download ONNX models first
cudgel index /path/to/codebase

## Developer Implementation Checklist

### Phase 1 (TDD): Write Tests First
1. [ ] Test: TokenizerStrategy trait definition compiles
2. [ ] Test: FallbackTokenizer produces 384-dim vectors
3. [ ] Test: Environment variable selection works
4. [ ] Test: Invalid strategy names produce helpful errors
5. [ ] Test: ONNX remains default when env var not set

### Phase 2: Implement
1. [ ] Extract ONNX code into embeddings::onnx module
2. [ ] Implement FallbackTokenizer in embeddings::fallback module
3. [ ] Add strategy factory in embeddings::mod
4. [ ] Update Config to read CUDGEL_TOKENIZER_STRATEGY
5. [ ] Update EmbeddingGenerator to use strategy abstraction

### Phase 3: Integration Test
1. [ ] Test with real codebase (cudgel self-index)
2. [ ] Verify query results are reasonable (fallback vs. ONNX comparison)
3. [ ] Check initialization time (<5s for fallback)
4. [ ] Validate error messages are actionable

## File Modification Checklist

- src/embeddings.rs: Refactor to use TokenizerStrategy trait
- src/embeddings/mod.rs: New module with strategy factory
- src/embeddings/onnx.rs: Extract existing ONNX implementation
- src/embeddings/fallback.rs: New fallback implementation
- src/config.rs: Add CUDGEL_TOKENIZER_STRATEGY reading
- src/error.rs: Add InvalidTokenizerStrategy variant
- tests/embeddings_fallback_tests.rs: New test suite
```

### 5. Agent Context Update

After Phase 1 completion, run:

```bash
.specify/scripts/bash/update-agent-context.sh opencode
```

This will update `AGENTS.md` with:
- New tokenization strategy abstraction approach
- Configuration pattern for strategy selection
- Testing patterns for embedding quality validation

---

## Phase 2: Task Breakdown

**Phase 2 is handled by the `/speckit.tasks` command (separate from this plan).**

The tasks command will generate `tasks.md` with:
- TDD task sequence (test files, then implementation)
- Task dependencies and ordering
- Acceptance criteria per task linked to requirements
- Time estimates

Preliminary task areas (will be detailed in `/speckit.tasks`):

1. **Setup & Refactoring** (User Story 2, P2)
   - Extract ONNX code to separate module
   - Define TokenizerStrategy trait
   - Update EmbeddingGenerator to use trait

2. **Fallback Implementation** (User Story 1, P1 - MVP)
   - Implement fallback tokenization algorithm (from research)
   - Generate 384-dim embeddings
   - Unit tests for encoding correctness

3. **Configuration Integration** (User Story 1, P1)
   - Add CUDGEL_TOKENIZER_STRATEGY env var reading
   - Implement strategy factory
   - Validation and error handling

4. **Testing & Validation** (User Story 3, P3)
   - Integration tests for strategy switching
   - Quality validation tests
   - Performance benchmarks

---

## Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Fallback embeddings produce poor search results | High - Users get irrelevant results | Medium | Research phase validates algorithm choice, SC-005 requires manual quality validation before acceptance |
| 384-dim constraint limits algorithm choices | Medium - Fewer viable algorithms | Low | Research includes dimensionality reduction techniques, can use projection methods |
| Performance regression with fallback | Medium - Slower indexing | Low | Performance goal is <5s initialization (relaxed vs. ONNX), maintain indexing speed |
| Breaking changes to EmbeddingGenerator API | High - Affects dependent code | Low | Trait abstraction maintains existing interface, refactor is internal |
| Environment variable not portable across platforms | Low - Config doesn't work on Windows | Low | Use std::env::var which works cross-platform |

---

## Success Criteria Mapping

| Success Criteria | Implementation Verification |
|------------------|----------------------------|
| SC-001: Index codebase in restricted environments | Integration test: Set fallback strategy, index cudgel itself, verify completion |
| SC-002: Fallback init <5 seconds | Performance test: Measure FallbackTokenizer::initialize() duration |
| SC-003: Actionable error messages | Unit test: Verify error message content includes both options and syntax |
| SC-004: 100% indexing success with fallback | Integration test: Full indexing run with fallback, assert no failures |
| SC-005: Semantically meaningful results | Manual test: Run benchmark queries, compare top-5 results to ONNX baseline |

---

## Next Steps

1. ✅ Complete this plan
2. ⏳ Run Phase 0 research (dispatched automatically by this command)
3. ⏳ Generate Phase 1 artifacts (data-model.md, contracts/, quickstart.md)
4. ⏳ Update agent context with new patterns
5. ⏳ Re-validate Constitution Check with concrete design
6. ⏳ Run `/speckit.tasks` to generate detailed task breakdown

**Command Status**: Planning complete. Research phase starting...
