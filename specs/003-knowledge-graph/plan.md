# Implementation Plan: Knowledge Graph for Code Understanding

**Branch**: `003-knowledge-graph` | **Date**: 2025-11-19 | **Spec**: [/specs/003-knowledge-graph/spec.md]
**Input**: Feature specification from `/specs/003-knowledge-graph/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Implement LLM-powered knowledge graph generation during code indexing using PostgreSQL adjacency lists for graph storage. Feature adds natural language architecture summaries and entity relationship queries to existing cudgel indexing pipeline, enabling rapid codebase understanding through Ollama-generated documentation and graph traversal queries.

## Technical Context

**Language/Version**: Rust 2021 Edition (MSRV 1.75+)  
**Primary Dependencies**: tree-sitter, tokio, postgres + pgvector, ort (ONNX), ollama-rs, clap, thiserror  
**Storage**: PostgreSQL 15+ with pgvector extension (port 45678) using adjacency lists and junction tables  
**Testing**: cargo test (unit + integration)  
**Target Platform**: macOS, Linux (x86_64, ARM64)  
**Project Type**: Single CLI application with local-first architecture  
**Performance Goals**: <5s architecture queries, <3s entity queries, <30min indexing for 50k files  
**Constraints**: <500MB RSS during operations, offline-capable, local PostgreSQL only  
**Scale/Scope**: Up to 100k graph nodes, 500k edges per repository

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: Local-First Architecture ✅
- Using PostgreSQL local storage (port 45678) 
- No external service dependencies except optional Ollama
- Code and data remain on local machine

### Principle II: Test-Driven Development ✅
- Feature spec includes independent test criteria for each user story
- Will follow Red-Green-Refactor cycle
- Integration tests will verify contract compliance

### Principle III: Performance & Efficiency ✅
- Query targets: <5s architecture, <3s entity queries
- Memory target: <500MB RSS during operations
- Incremental processing via SHA256 hash change detection

### Principle IV: Semantic Intelligence ✅
- Using ONNX embeddings (sentence-transformers/all-MiniLM-L6-v2)
- AST parsing via tree-sitter for accurate language understanding
- Vector similarity search via pgvector

### Principle V: Incremental Processing ✅
- SHA256 content hashing for change detection
- Skip unchanged files during re-indexing
- Idempotent database operations

### Technology Stack Requirements ✅
- Rust 2021 edition with required dependencies
- PostgreSQL + pgvector for storage
- ONNX Runtime for embeddings
- Ollama integration for LLM summaries

### Development Workflow ✅
- Error handling with thiserror
- Naming conventions followed
- Testing strategy with cargo test
- Code review gates defined

**GATE STATUS: PASS - No constitution violations identified**

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

```text
src/
├── kg/                    # Knowledge graph module (new)
│   ├── mod.rs
│   ├── client.rs            # PostgreSQL graph client
│   ├── model.rs            # Graph data models
│   └── schema.rs           # Database schema for graph tables
├── llm/                   # LLM integration (existing from 003)
│   ├── mod.rs
│   ├── client.rs            # Ollama client
│   └── prompts.rs          # Summary generation prompts
├── parser.rs              # Existing tree-sitter parser
├── indexer.rs             # Existing indexer (will be enhanced)
├── database.rs            # Existing PostgreSQL database
├── query.rs              # Existing query engine
└── main.rs               # CLI entry point (enhanced)

tests/
├── integration/
│   ├── test_kg_client.rs
│   ├── test_llm_integration.rs
│   └── test_knowledge_graph.rs
└── unit/
    ├── test_kg_models.rs
    └── test_graph_schema.rs
```

**Structure Decision**: Single CLI application with new `kg/` module for knowledge graph functionality, integrating with existing parser, indexer, and database modules.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| No violations identified | - | - |
