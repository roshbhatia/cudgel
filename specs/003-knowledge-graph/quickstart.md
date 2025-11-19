# Quickstart: Knowledge Graph for Code Understanding

**Feature**: 003-knowledge-graph  
**Date**: 2025-11-19

## Overview

This quickstart guide helps developers understand and implement the knowledge graph feature. It provides a high-level development path from basic graph operations to full integration with the indexing pipeline.

---

## Prerequisites

Before starting development:

1. **Read the specification**: [spec.md](./spec.md)
2. **Review research decisions**: [research.md](./research.md)
3. **Understand the data model**: [data-model.md](./data-model.md)
4. **Study the contracts**: [contracts/](./contracts/)
5. **Install Ollama**: `curl -fsSL https://ollama.ai/install.sh | sh`
6. **Pull LLM model**: `ollama pull llama3.2:3b`
7. **Start Ollama service**: `ollama serve`

---

## Development Roadmap

### Phase 1: Graph Database Foundation (P1 - User Story 1)

**Goal**: Set up SurrealDB and implement basic graph operations.

**Tasks**:
1. Add SurrealDB dependency to `Cargo.toml`
2. Create `src/graph/` module structure
3. Implement `GraphClient` trait with basic CRUD operations
4. Write unit tests for node creation and retrieval
5. Initialize graph schema on first run

**Deliverables**:
- `src/graph/mod.rs` - Module exports
- `src/graph/client.rs` - SurrealDB client implementation
- `src/graph/model.rs` - Node and edge type definitions
- `src/graph/schema.rs` - Schema initialization
- `tests/test_graph_client.rs` - Unit tests

**Success Criteria**:
- Can create Repository, Component, and CodeEntity nodes
- Can query nodes by ID and name
- All tests pass
- Schema initializes correctly on fresh database

---

### Phase 2: Entity Extraction (P1 - User Story 1)

**Goal**: Extract code entities from parsed files and populate graph.

**Tasks**:
1. Create `src/graph/builder.rs` for graph construction
2. Integrate with existing `parser.rs` to extract entities
3. Map tree-sitter AST nodes to CodeEntity nodes
4. Extract relationship information (imports, calls, inheritance)
5. Write integration tests with sample code

**Deliverables**:
- `src/graph/builder.rs` - Graph building logic
- `tests/fixtures/sample_repo/` - Test code samples
- `tests/test_graph_builder.rs` - Integration tests

**Success Criteria**:
- Can extract entities from Rust files
- Can identify dependencies between entities
- Integration test indexes small repository successfully

---

### Phase 3: LLM Integration (P1 - User Story 1)

**Goal**: Generate architecture summaries using Ollama.

**Tasks**:
1. Add `ollama-rs` dependency
2. Create `src/llm/` module structure
3. Implement `LlmClient` trait
4. Design prompt templates for different summary types
5. Add error handling and retry logic
6. Write tests with mock responses

**Deliverables**:
- `src/llm/mod.rs` - Module exports
- `src/llm/client.rs` - Ollama client wrapper
- `src/llm/prompts.rs` - Prompt templates
- `src/llm/summarizer.rs` - Summary generation orchestration
- `tests/test_llm_integration.rs` - Unit and integration tests

**Success Criteria**:
- Can generate repository-level summary
- Can generate module/component summaries
- Can generate entity-level summaries
- Graceful degradation when Ollama unavailable
- All tests pass (mock tests always, integration tests when Ollama available)

---

### Phase 4: Query Interface (P2 - User Story 2)

**Goal**: Parse natural language queries and return relevant graph data.

**Tasks**:
1. Create `src/graph/query.rs` for query parsing
2. Implement intent classification (rule-based + regex)
3. Add fuzzy entity matching using `strsim`
4. Map intents to graph traversal queries
5. Handle disambiguation for multiple matches
6. Write query parsing tests

**Deliverables**:
- `src/graph/query.rs` - Query parser and executor
- `tests/test_graph_queries.rs` - Query parsing tests

**Success Criteria**:
- Can parse common query patterns from spec
- Can find entities by name (exact and fuzzy)
- Can traverse relationships
- Returns clear error messages for ambiguous queries
- Query response time <3 seconds

---

### Phase 5: CLI Integration (P1, P2, P3)

**Goal**: Integrate knowledge graph with cudgel CLI.

**Tasks**:
1. Modify `src/main.rs` to add graph commands
2. Add `--enable-graph` flag to `index` command
3. Create `query` subcommand for natural language queries
4. Integrate graph building into indexing pipeline
5. Add progress indicators for summary generation
6. Update CLI help text and documentation

**Deliverables**:
- Modified `src/main.rs` with new commands
- Modified `src/orchestrator.rs` to coordinate graph building
- Modified `src/indexer.rs` to trigger graph updates

**Success Criteria**:
- `cudgel index --enable-graph` builds knowledge graph
- `cudgel query "what is the architecture?"` returns summary
- `cudgel query "what does Parser do?"` returns entity info
- CLI provides clear feedback during long operations
- Existing indexing functionality still works without `--enable-graph`

---

### Phase 6: Incremental Updates (P1)

**Goal**: Update graph incrementally when files change.

**Tasks**:
1. Extend `GraphBuilder` with incremental update methods
2. Detect changed files using existing SHA256 hashing
3. Delete affected entities and relationships
4. Re-extract and re-create updated entities
5. Regenerate summaries for affected components
6. Write tests for incremental update scenarios

**Deliverables**:
- Enhanced `src/graph/builder.rs` with incremental methods
- `tests/test_incremental_updates.rs` - Incremental update tests

**Success Criteria**:
- File changes trigger partial graph updates
- Unchanged entities are not re-processed
- Incremental update completes in <2 minutes for 10k files
- Graph remains consistent after updates

---

### Phase 7: Pattern Analysis (P4 - User Story 4)

**Goal**: Enable cross-cutting concern analysis.

**Tasks**:
1. Extend query parser for pattern queries
2. Implement pattern matching across entities
3. Generate pattern analysis summaries
4. Add specialized queries for common patterns

**Deliverables**:
- Enhanced `src/graph/query.rs` with pattern analysis
- Pattern-specific prompt templates in `src/llm/prompts.rs`

**Success Criteria**:
- Can query "how is error handling implemented?"
- Returns relevant entities and patterns
- Provides architectural summary of pattern usage

---

## Development Commands

### Build & Test

```bash
# Build the project
cargo build --release

# Run all tests
cargo test

# Run specific test
cargo test test_graph_client

# Run with Ollama integration tests (requires Ollama running)
OLLAMA_AVAILABLE=1 cargo test --ignored

# Lint
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt

# Check without building
cargo check
```

---

### Database Management

```bash
# Start PostgreSQL (existing)
./scripts/start-postgres.sh

# Initialize graph database (embedded, automatic)
# No separate server needed - runs in-process

# Inspect graph data (use SurrealDB CLI)
surreal sql --endpoint file://data/graph.db --namespace cudgel --database knowledge_graph
```

---

### Ollama Management

```bash
# Start Ollama service
ollama serve

# Pull model
ollama pull llama3.2:3b

# Test model
ollama run llama3.2:3b "Hello, test"

# List available models
ollama list

# Check Ollama is running
curl http://localhost:11434/api/version
```

---

## Testing Strategy

### Test Pyramid

```
           ┌─────────────┐
          /   E2E Tests   \     (5%) - Full workflows
         /─────────────────\
        /  Integration Tests \   (25%) - Component interactions
       /─────────────────────\
      /      Unit Tests       \  (70%) - Individual functions
     /─────────────────────────\
```

### Test Categories

**Unit Tests** (fast, no external dependencies):
- Graph client operations (with in-memory database)
- Query parsing logic
- Prompt template generation
- Data model validation

**Integration Tests** (medium speed, local dependencies):
- Full graph building from sample code
- LLM summary generation (with mock or real Ollama)
- Query execution against populated graph
- Incremental update workflows

**E2E Tests** (slow, full system):
- Index real repository with knowledge graph
- Execute queries against real graph
- Verify performance requirements

### Test Data

Create test fixtures in `tests/fixtures/sample_repo/`:
```
tests/fixtures/sample_repo/
├── src/
│   ├── main.rs          # Entry point with dependencies
│   ├── parser.rs        # Module with functions and structs
│   ├── indexer.rs       # Module with relationships
│   └── utils/
│       └── helpers.rs   # Nested module
└── Cargo.toml           # Metadata
```

---

## Performance Benchmarks

Set up benchmarks for critical operations:

```rust
// benches/graph_operations.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_entity_creation(c: &mut Criterion) {
    c.bench_function("create_entity", |b| {
        b.iter(|| {
            // Benchmark single entity creation
        });
    });
}

fn benchmark_batch_entity_creation(c: &mut Criterion) {
    c.bench_function("create_entities_batch_100", |b| {
        b.iter(|| {
            // Benchmark batch creation of 100 entities
        });
    });
}

fn benchmark_relationship_traversal(c: &mut Criterion) {
    c.bench_function("traverse_dependencies_3_hops", |b| {
        b.iter(|| {
            // Benchmark 3-hop dependency traversal
        });
    });
}

criterion_group!(benches, 
    benchmark_entity_creation,
    benchmark_batch_entity_creation,
    benchmark_relationship_traversal
);
criterion_main!(benches);
```

Run benchmarks:
```bash
cargo bench
```

---

## Common Pitfalls & Solutions

### Pitfall 1: Ollama Not Available

**Problem**: Tests fail when Ollama is not running.

**Solution**: 
- Use `#[ignore]` attribute for tests requiring Ollama
- Check health in test setup and skip gracefully
- Provide mock LLM client for unit tests

```rust
#[tokio::test]
#[ignore] // Only run with OLLAMA_AVAILABLE=1
async fn test_with_real_ollama() {
    // Test code
}
```

---

### Pitfall 2: Graph Database Lock Contention

**Problem**: Concurrent writes to SurrealDB cause deadlocks.

**Solution**:
- Batch entity creation instead of one-by-one
- Use transactions for related operations
- Serialize writes per repository

---

### Pitfall 3: Large Context Overwhelms LLM

**Problem**: Passing too much code context causes timeouts.

**Solution**:
- Truncate code snippets to 50 lines
- Limit entity lists to 20 items
- Use summary instead of full code for nested contexts

---

### Pitfall 4: Fuzzy Matching False Positives

**Problem**: Query for "Parser" matches "UrlParser", "JsonParser", etc.

**Solution**:
- Set appropriate threshold (0.85 works well)
- Present all matches with confidence scores
- Allow user to disambiguate with full path

---

## Next Steps After Quickstart

1. **Review constitution compliance**: Verify all principles are followed
2. **Update AGENTS.md**: Add new technologies and patterns
3. **Write comprehensive tests**: Aim for >80% coverage
4. **Performance profiling**: Ensure targets are met
5. **Documentation**: Update README with knowledge graph usage
6. **User feedback**: Test with real codebases and iterate

---

## Resources

- **SurrealDB Docs**: https://surrealdb.com/docs
- **Ollama API**: https://github.com/ollama/ollama/blob/main/docs/api.md
- **Tree-sitter**: https://tree-sitter.github.io/tree-sitter/
- **Constitution**: `.specify/memory/constitution.md`
- **Spec**: `specs/003-knowledge-graph/spec.md`
- **Research**: `specs/003-knowledge-graph/research.md`

---

## Questions & Support

For questions during development:
1. Refer to contracts for interface requirements
2. Check existing similar features (e.g., indexer.rs, parser.rs)
3. Review constitution for architectural guidance
4. Run tests frequently (TDD approach)
5. Profile performance regularly
