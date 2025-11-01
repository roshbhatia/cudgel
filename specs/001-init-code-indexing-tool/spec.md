# Feature Specification: Cudgel Code Intelligence System

**Feature Branch**: `001-init-code-indexing-tool`
**Created**: 2025-10-31
**Status**: Implemented (US1 + US4 Complete)
**Input**: User description: "Build Cudgel, a local-first codebase intelligence system with four components: orchestrator daemon, index CLI, query CLI, and knowledge CLI"

**Implementation Note**: This spec originally covered 4 user stories. US1 (Index and Query) and US4 (LLM Export Formats) are now complete and production-ready. US2 (Scheduling) and US3 (Knowledge Graph) will be implemented in separate specs/branches (002-automatic-re-indexing, 003-knowledge-graph-generation).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Index and Query Codebase (Priority: P1) ✅ IMPLEMENTED

**Status**: ✅ Complete and Production-Ready

A developer wants to index their local repository and perform semantic searches to understand their codebase without manually grepping or browsing files.

**Why this priority**: This is the core MVP functionality. Without the ability to index and search, no other features provide value. This delivers immediate utility by enabling fast, semantic code discovery.

**Independent Test**: Can be fully tested by running `cudgel index /path/to/repo` followed by `cudgel query "authentication logic"` and verifying that relevant code symbols are returned in a human-readable table format.

**Implementation Features**:
- ✅ Multi-language support (Python, JavaScript, TypeScript, Rust, Go, C, C++, Java)
- ✅ Incremental indexing with SHA256 hash-based change detection
- ✅ ONNX-based embeddings (sentence-transformers/all-MiniLM-L6-v2)
- ✅ pgvector semantic search with HNSW indexing
- ✅ Advanced file filtering (glob patterns, include/exclude, language filtering)
- ✅ Go-style recursive path syntax (`./...`)
- ✅ Table and JSON output formats
- ✅ Comprehensive test coverage (32 tests passing)

**Acceptance Scenarios**:

1. **Given** a git repository with tracked files, **When** the developer runs `cudgel index /path/to/repo`, **Then** the system parses all git-tracked source files, extracts symbols via tree-sitter AST parsing, generates embeddings, and stores them in the local PostgreSQL database.

2. **Given** an indexed repository, **When** the developer runs `cudgel query "user authentication"`, **Then** the system performs semantic vector search and returns relevant code symbols (functions, classes, methods) in a formatted table showing file path, line number, symbol name, and code snippet.

3. **Given** an already-indexed repository, **When** the developer re-runs `cudgel index /path/to/repo`, **Then** the system detects changed files via content hashing and only re-indexes modified files (incremental update).

4. **Given** a query with no matching results, **When** the developer runs `cudgel query "nonexistent feature"`, **Then** the system displays a message indicating no results found with suggestions for broadening the query.

---

### User Story 2 - Schedule Automatic Re-indexing (Priority: P2)

A developer wants their actively-developed repository to stay indexed automatically, without manual re-indexing after every code change.

**Why this priority**: Automation improves the developer experience by keeping the index fresh. This builds on P1 by adding scheduling capability, but P1 must work independently first.

**Independent Test**: Can be fully tested by running `cudgel index --schedule hourly /path/to/repo`, confirming the orchestrator daemon is running, waiting for the scheduled interval, and verifying the repository is re-indexed automatically.

**Acceptance Scenarios**:

1. **Given** a repository, **When** the developer runs `cudgel index --schedule hourly /path/to/repo`, **Then** the system stores the schedule in the database and starts the orchestrator daemon (if not already running) to execute indexing every hour.

2. **Given** a scheduled indexing job, **When** the orchestrator daemon reaches the scheduled time, **Then** the system automatically runs incremental indexing for the repository without user intervention.

3. **Given** multiple repositories with different schedules, **When** the orchestrator daemon is running, **Then** it manages all scheduled jobs concurrently and executes each according to its schedule.

4. **Given** the orchestrator daemon is running, **When** the developer runs `cudgel orchestrator status`, **Then** the system displays all scheduled jobs with their next run time and last execution status.

5. **Given** a scheduled job, **When** the developer runs `cudgel index --unschedule /path/to/repo`, **Then** the system removes the schedule from the database and stops auto-indexing that repository.

---

### User Story 3 - Generate Knowledge Graph Documentation (Priority: P3)

A developer wants to generate high-level structured documentation about their codebase (architecture, dependencies, build process, licensing) to provide rich context to AI assistants without uploading source code.

**Why this priority**: Knowledge graphs add analytical value on top of raw code search. This is valuable but not critical for basic code intelligence, making it lower priority than indexing and scheduling.

**Independent Test**: Can be fully tested by running `cudgel knowledge` on an indexed repository, verifying a markdown document opens in the user's editor with structured sections (design, dependencies, build, licensing), and confirming the content is stored in the database.

**Acceptance Scenarios**:

1. **Given** an indexed repository, **When** the developer runs `cudgel knowledge`, **Then** the system analyzes the indexed data, generates a structured markdown document with sections for design/architecture, dependencies, build process, and licensing, and opens it in the user's default `$EDITOR`.

2. **Given** an existing knowledge graph document, **When** the developer runs `cudgel knowledge --edit`, **Then** the system opens the current version for editing, saves changes to the database when the editor closes, and preserves manual edits.

3. **Given** an existing knowledge graph, **When** the developer runs `cudgel knowledge --refresh`, **Then** the system re-analyzes the indexed code and updates auto-generated sections while preserving manually-edited sections.

4. **Given** an existing knowledge graph, **When** the developer runs `cudgel knowledge --replace`, **Then** the system completely regenerates the knowledge graph from scratch, discarding all previous content including manual edits.

5. **Given** a knowledge graph generation request, **When** the system uses Ollama to analyze patterns, **Then** it runs the llama3.2:8b model locally and includes findings about architectural patterns, dependency relationships, and code organization.

---

### User Story 4 - Export Query Results for LLM Consumption (Priority: P4) ✅ IMPLEMENTED

**Status**: ✅ Complete and Production-Ready

A developer wants to export query results in LLM-friendly formats (JSON, minified) to easily provide codebase context to AI assistants without manual copying or formatting.

**Why this priority**: This enhances the query functionality from P1 by adding machine-readable output formats. Valuable for LLM integration workflows but not essential for basic code search.

**Independent Test**: Can be fully tested by running `cudgel query "parser logic" --json` and verifying the output is valid JSON containing the same search results as the default table format, suitable for piping to other tools or LLMs.

**Implementation Features**:
- ✅ `--json` flag for compact JSON output (single line)
- ✅ `--json-pretty` flag for indented, human-readable JSON
- ✅ `--minified` flag for LLM-optimized format with abbreviated keys
- ✅ Token-efficient minification (p=path, l=line, n=name, k=kind, s=similarity)
- ✅ Omits empty/null fields to reduce token count
- ✅ jq-compatible output for piping to other tools

**Acceptance Scenarios**:

1. **Given** an indexed repository, **When** the developer runs `cudgel query "search term" --json`, **Then** the system returns results in compact JSON format with all result fields (file path, line number, symbol name, code snippet, similarity score).

2. **Given** an indexed repository, **When** the developer runs `cudgel query "search term" --json-pretty`, **Then** the system returns results in pretty-printed JSON format with indentation for human readability.

3. **Given** an indexed repository, **When** the developer runs `cudgel query "search term" --minified`, **Then** the system returns results in LLM-OpenAPI-minifier format, optimized for token efficiency by removing redundant whitespace and metadata.

4. **Given** query results in any format, **When** the developer pipes the output to another tool (e.g., `cudgel query "term" --json | jq`), **Then** the output is valid and parseable by standard JSON processors.

---

### Edge Cases

- **What happens when the repository path doesn't exist?** System displays an error message with the invalid path and exits with non-zero status code.

- **What happens when PostgreSQL is not running?** System fails fast with an actionable error message: "PostgreSQL is not running on port 54321. Run 'task db-start' to start the database."

- **What happens when git is not installed or the path is not a git repository?** System displays an error: "Not a git repository or git not found. Cudgel only indexes git-tracked files."

- **What happens when embedding models are missing?** System detects missing models at startup and displays an error with download instructions.

- **What happens when the orchestrator daemon crashes?** System logs the crash reason to `~/.local/state/cudgel/orchestrator.log` and exits. User must manually restart with `cudgel orchestrator start`.

- **What happens when two indexing operations run concurrently on the same repository?** System uses database-level locking to prevent concurrent indexing. The second operation waits or fails with a "repository currently being indexed" error.

- **What happens when the user's `$EDITOR` environment variable is not set for knowledge graph?** System falls back to `vim`, then `nano`, then displays an error if neither is available.

- **What happens when Ollama service is not running for knowledge graph generation?** System displays an error: "Ollama is not running on localhost:11434. Start Ollama service to use knowledge graph features."

- **What happens when query returns thousands of results?** System limits default output to 50 results and displays a message: "Showing top 50 results. Use --limit N to adjust."

## Requirements *(mandatory)*

### Functional Requirements

#### Indexing Requirements

- **FR-001**: System MUST index only git-tracked files in the specified repository path.

- **FR-002**: System MUST support incremental indexing by comparing file content hashes and only re-parsing changed files.

- **FR-003**: System MUST parse source files using tree-sitter to extract code symbols (functions, classes, methods, variables).

- **FR-004**: System MUST generate vector embeddings for each extracted symbol and store them in PostgreSQL with pgvector extension.

- **FR-005**: System MUST support scheduling indexing with intervals: hourly, daily, or custom hours (e.g., every 6 hours).

- **FR-006**: System MUST store repository metadata (path, last indexed time, file count, symbol count) in PostgreSQL.

#### Orchestrator Requirements

- **FR-007**: System MUST provide a background daemon (`cudgel orchestrator`) that runs scheduled indexing jobs using a polling loop.

- **FR-008**: Orchestrator MUST store scheduled jobs in PostgreSQL with repository path, schedule interval, and next run time.

- **FR-009**: Orchestrator MUST execute scheduled jobs at their designated times without user intervention.

- **FR-010**: Orchestrator MUST log execution events (start, completion, errors) to `~/.local/state/cudgel/orchestrator.log`.

- **FR-011**: System MUST allow starting, stopping, and checking status of the orchestrator daemon via CLI commands.

#### Query Requirements

- **FR-012**: System MUST perform semantic vector similarity search using pgvector against indexed embeddings.

- **FR-013**: System MUST return query results sorted by semantic similarity score (highest first).

- **FR-014**: System MUST display results in human-readable table format by default, showing file path, line number, symbol name, and code snippet.

- **FR-015**: System MUST support `--json` flag to output results in compact JSON format.

- **FR-016**: System MUST support `--json-pretty` flag to output results in formatted JSON with indentation.

- **FR-017**: System MUST support `--minified` flag to output results in LLM-OpenAPI-minifier format for token efficiency.

- **FR-018**: System MUST support `--limit N` flag to control the maximum number of results returned (default: 50).

#### Knowledge Graph Requirements

- **FR-019**: System MUST generate structured markdown documentation with sections: design/architecture, dependencies, build process, and licensing.

- **FR-020**: System MUST use Ollama with llama3.2:8b model to analyze indexed code and generate knowledge graph content.

- **FR-021**: System MUST open the knowledge graph in the user's `$EDITOR` environment variable (fallback: vim, then nano).

- **FR-022**: System MUST store knowledge graph content in PostgreSQL, associated with the repository.

- **FR-023**: System MUST support `--edit` flag to open and modify existing knowledge graph content.

- **FR-024**: System MUST support `--refresh` flag to update auto-generated sections while preserving manual edits.

- **FR-025**: System MUST support `--replace` flag to completely regenerate the knowledge graph, discarding previous content.

#### Infrastructure Requirements

- **FR-026**: System MUST verify PostgreSQL is running on configured port (default: 54321) at startup.

- **FR-027**: System MUST auto-initialize database schema if not present (tables for repos, files, symbols, embeddings, schedules, knowledge graphs).

- **FR-028**: System MUST verify embedding models exist at `~/.local/share/cudgel/models/` before indexing.

- **FR-029**: System MUST store all data in XDG-compliant directories (data, config, cache, state).

- **FR-030**: System MUST fail fast with actionable error messages when dependencies (PostgreSQL, Ollama, git, embedding models) are missing.

### Key Entities

- **Repository**: Represents a git repository being indexed. Attributes: path, last indexed timestamp, total files, total symbols, indexing status.

- **File**: Represents a source file within a repository. Attributes: path relative to repo root, content hash (SHA256), language, last parsed timestamp, symbol count.

- **Symbol**: Represents a code construct (function, class, method, variable). Attributes: name, type (function/class/method/etc), file reference, line number, code snippet, documentation.

- **Embedding**: Represents a vector embedding for a symbol. Attributes: symbol reference, vector (384-dimensional float array), generated timestamp.

- **Schedule**: Represents a scheduled indexing job. Attributes: repository reference, interval (hours), next run time, last execution time, status (active/paused).

- **KnowledgeDocument**: Represents structured documentation for a repository. Attributes: repository reference, markdown content, generated timestamp, last edited timestamp, version number.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Developers can index a repository with 10,000 files in under 5 minutes on commodity hardware (4-core CPU minimum, 8GB RAM minimum, e.g., M1 MacBook).

- **SC-002**: Developers can execute semantic queries and receive results in under 1 second for repositories with up to 100,000 indexed symbols.

- **SC-003**: System maintains index freshness by automatically re-indexing scheduled repositories with 99% on-time execution (within 5 minutes of scheduled time).

- **SC-004**: Developers can generate a complete knowledge graph document in under 2 minutes for a repository with 50,000 lines of code.

- **SC-005**: Query result accuracy achieves 80% relevance in top 10 results (verified by rating top 10 results per query as relevant/not-relevant for your search intent across 20+ test queries).

- **SC-006**: System memory footprint remains under 500MB RSS during active indexing operations.

- **SC-007**: Incremental re-indexing processes only changed files, resulting in 90% reduction in re-index time compared to full re-indexing for repositories with <10% file changes.

- **SC-008**: JSON and minified output formats reduce token usage by 40% compared to copying raw source files when providing context to LLMs.

- **SC-009**: Knowledge graph generation identifies architectural patterns (MVC, microservices, monolith) with 70% accuracy compared to manual developer assessment.

- **SC-010**: System startup time (dependency checks + database connection) completes in under 500ms, providing immediate feedback to developers.
