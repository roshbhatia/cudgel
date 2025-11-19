# Implementation Plan: Knowledge Graph for Code Understanding

**Branch**: `003-knowledge-graph` | **Date**: 2025-11-19 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/003-knowledge-graph/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Add knowledge graph capabilities to cudgel's indexing pipeline to generate LLM-powered architecture summaries and store queryable entity relationships in a graph database. This enables developers to understand unfamiliar codebases through natural language queries about architecture, component purposes, and entity relationships.

## Technical Context

**Language/Version**: Rust 2021 edition (cargo 1.75+)  
**Primary Dependencies**: tokio (async), postgres + pgvector, ort (ONNX), tree-sitter parsers, ollama-rs (Ollama client), surrealdb (graph database), strsim (fuzzy matching), regex (query parsing)  
**Storage**: PostgreSQL 15+ (port 45678) for existing data + SurrealDB embedded (file-based) for graph data  
**Testing**: cargo test (existing setup with setup_test_db() helper)  
**Target Platform**: macOS, Linux (x86_64, ARM64)  
**Project Type**: Single CLI application  
**Performance Goals**: 
- Index 50k files in <30 minutes
- Query response <3 seconds
- Architecture summary generation <5 seconds
- Incremental re-index <2 minutes for 10k files  

**Constraints**: 
- Local-first: graph database must run locally
- Memory: <500MB RSS during operations (may need streaming LLM interactions)
- Ollama integration for LLM summaries (external service but local)  

**Scale/Scope**: 
- Support codebases up to 50k files
- Graph with up to 100k nodes, 500k edges
- Query performance acceptable at this scale

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### ✅ Phase 0 Check (Pre-Research) - PASSED

All unknowns resolved in research.md. Ready for Phase 1.

### ✅ Phase 1 Check (Post-Design) - PASSED

All design decisions documented. Ready for implementation.

### Principle Compliance

| Principle | Status | Notes |
|-----------|--------|-------|
| **I. Local-First Architecture** | ✅ COMPLIANT (JUSTIFIED) | Ollama runs locally. SurrealDB embedded mode (file-based, no server). See justification below. |
| **II. Test-Driven Development** | ✅ COMPLIANT | Test pyramid defined in quickstart.md. Red-Green-Refactor approach documented. |
| **III. Performance & Efficiency** | ✅ COMPLIANT | Success criteria meet constitution: 30min for 50k files, <3s queries, <2min incremental updates. |
| **IV. Semantic Intelligence** | ✅ COMPLIANT | Extends existing semantic search with graph relationships. Uses existing tree-sitter + embeddings. |
| **V. Incremental Processing** | ✅ COMPLIANT | Incremental graph updates via SHA256 change detection. Cascade updates documented in data-model.md. |

### Technology Stack Compliance

| Requirement | Status | Notes |
|-------------|--------|-------|
| Rust 2021 + cargo 1.75+ | ✅ COMPLIANT | Existing project standard |
| tokio async runtime | ✅ COMPLIANT | Already in use |
| postgres + pgvector | ✅ COMPLIANT | Existing infrastructure at port 45678 |
| tree-sitter parsers | ✅ COMPLIANT | Will leverage existing parser infrastructure |
| thiserror + anyhow errors | ✅ COMPLIANT | Will follow existing error handling patterns |
| Zero clippy warnings | ✅ COMPLIANT | CI gates already enforce this |

### Local-First Architecture Justification

**Ollama Dependency**: Ollama runs as a local service and is required for knowledge graph feature (FR-001). This is acceptable because:
1. Feature is explicitly optional (can index without knowledge graph via `--enable-graph` flag)
2. Ollama runs locally, no cloud API calls, no data leaves machine
3. Aligns with constitution's "except optional Ollama for knowledge graph features"
4. Users maintain full control over their code and data
5. Graceful degradation: indexing continues without summaries if Ollama unavailable

**Graph Database Selection**: SurrealDB embedded mode selected because:
1. ✅ Runs in-process (no separate server)
2. ✅ File-based storage (offline-capable)
3. ✅ Native Rust client with async support
4. ✅ Handles target scale (100k nodes, 500k edges)
5. ✅ No network ports required
6. ✅ Single binary deployment

**Verdict**: Feature maintains local-first architecture. All data remains on user's machine.

### Development Workflow Compliance

| Requirement | Status | Notes |
|-------------|--------|-------|
| Error handling via thiserror | ✅ COMPLIANT | Will create GraphError enum with user-friendly messages |
| Early validation | ✅ COMPLIANT | Validate queries, entity names, graph operations |
| Test naming conventions | ✅ COMPLIANT | test_graph_*, test_llm_*, test_query_* |
| Database test helpers | ✅ COMPLIANT | Will extend setup_test_db() for graph operations |
| Unit + integration tests | ✅ COMPLIANT | Unit tests per component, integration for E2E flows |

## Project Structure

### Documentation (this feature)

```text
specs/003-knowledge-graph/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
│   ├── graph-schema.md  # Graph node types, relationships, properties
│   └── llm-interface.md # Ollama integration contract
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── graph/               # NEW: Knowledge graph module
│   ├── mod.rs          # Module exports
│   ├── client.rs       # Graph database client abstraction
│   ├── model.rs        # Node and edge types
│   ├── builder.rs      # Graph construction from parsed code
│   ├── query.rs        # Query interface and NL parsing
│   └── schema.rs       # Graph schema definitions
├── llm/                # NEW: LLM integration module
│   ├── mod.rs          # Module exports
│   ├── client.rs       # Ollama client wrapper
│   ├── prompts.rs      # Prompt templates for summaries
│   └── summarizer.rs   # Summary generation logic
├── orchestrator.rs     # MODIFY: Integrate graph + LLM into indexing
├── indexer.rs          # MODIFY: Hook graph building into indexing pipeline
├── parser.rs           # USE: Leverage existing tree-sitter parsing
├── database.rs         # USE: Existing PostgreSQL operations
└── main.rs             # MODIFY: Add graph query commands

tests/
├── test_graph_builder.rs      # NEW: Graph construction tests
├── test_llm_integration.rs    # NEW: Ollama integration tests
├── test_graph_queries.rs      # NEW: Query processing tests
├── integration_tests.rs       # MODIFY: Add E2E graph workflows
└── fixtures/                  # NEW: Sample code for testing
    └── sample_repo/           # Small test repository
```

**Structure Decision**: Single project structure (Option 1) as cudgel is a CLI tool. New modules for graph and LLM functionality integrate with existing indexer/orchestrator. Existing parser and database infrastructure will be leveraged.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Ollama external service | LLM summaries require inference capability beyond basic indexing | Local LLM inference in Rust would require massive model files (GBs) and complex inference code. Ollama provides standardized local LLM access with <100MB client overhead. |
| Graph database addition | Queryable relationship traversal requires graph semantics not available in PostgreSQL | PostgreSQL recursive CTEs are significantly slower for multi-hop relationship queries and don't provide graph-specific query languages. Graph databases optimize for traversal patterns. |

**Justification**: Both additions are explicitly called out in the constitution as acceptable: "except optional Ollama for knowledge graph features". Feature is opt-in and maintains local-first architecture.

---

## Phase Completion Status

### ✅ Phase 0: Research (Complete)

**Deliverable**: [research.md](./research.md)

**Key Decisions**:
- Graph Database: SurrealDB (embedded mode)
- LLM Integration: ollama-rs crate with llama3.2:3b model
- Query Parsing: Hybrid rule-based + fuzzy matching (strsim)

**All NEEDS CLARIFICATION items resolved.**

---

### ✅ Phase 1: Design & Contracts (Complete)

**Deliverables**:
- [data-model.md](./data-model.md) - Graph schema with nodes and relationships
- [contracts/graph-client-interface.md](./contracts/graph-client-interface.md) - GraphClient trait definition
- [contracts/llm-client-interface.md](./contracts/llm-client-interface.md) - LlmClient trait definition
- [quickstart.md](./quickstart.md) - Development roadmap and guide
- AGENTS.md - Updated with new technologies

**Constitution Check**: ✅ PASSED (post-design)

**Ready for Phase 2**: `/speckit.tasks` to generate implementation tasks.

---

## Next Steps

1. Run `/speckit.tasks` to break down implementation into concrete tasks
2. Begin Phase 1 implementation (Graph Database Foundation)
3. Follow TDD approach: write tests first, then implement
4. Track progress using generated tasks.md checklist
