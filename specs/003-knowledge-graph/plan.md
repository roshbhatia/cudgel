# Implementation Plan: Knowledge Graph for Code Understanding

**Branch**: `003-knowledge-graph` | **Date**: 2025-11-19 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/003-knowledge-graph/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Add knowledge graph capabilities to cudgel's indexing pipeline to generate LLM-powered architecture summaries and store queryable entity relationships in a graph database. This enables developers to understand unfamiliar codebases through natural language queries about architecture, component purposes, and entity relationships.

## Technical Context

**Language/Version**: Rust 2021 edition (cargo 1.75+)  
**Primary Dependencies**: tokio (async), postgres + pgvector, ort (ONNX), tree-sitter parsers, ollama-rs (Ollama client), NEEDS CLARIFICATION: graph database Rust client  
**Storage**: PostgreSQL 15+ (port 45678) for existing data + NEEDS CLARIFICATION: graph database selection (Neo4j, MemGraph, or embedded option like SurrealDB)  
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

### Principle Compliance

| Principle | Status | Notes |
|-----------|--------|-------|
| **I. Local-First Architecture** | ⚠️ NEEDS JUSTIFICATION | Ollama is external service (local but separate process). Graph database selection must support local deployment. |
| **II. Test-Driven Development** | ✅ COMPLIANT | Will follow Red-Green-Refactor for all components. Integration tests will verify LLM integration and graph operations. |
| **III. Performance & Efficiency** | ✅ COMPLIANT | Success criteria align: 30min for 50k files, <3s queries. Incremental re-indexing required. |
| **IV. Semantic Intelligence** | ✅ COMPLIANT | Extends existing semantic search with graph relationships. Leverages existing tree-sitter and embeddings infrastructure. |
| **V. Incremental Processing** | ✅ COMPLIANT | Must update graph incrementally based on file changes using existing SHA256 content hashing. |

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
1. Feature is explicitly optional (can index without knowledge graph)
2. Ollama runs locally, no cloud API calls
3. Aligns with constitution's "except optional Ollama for knowledge graph features"
4. Users maintain full control over their code and data

**Graph Database Selection**: Must choose a database that:
1. Runs locally without external services
2. Supports offline operation after initial setup
3. Provides Rust client library
4. Handles 100k nodes / 500k edges efficiently

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
