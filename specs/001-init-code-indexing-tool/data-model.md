# Data Model: Cudgel Code Intelligence System

**Date**: 2025-10-31
**Feature**: 001-init-code-indexing-tool

## Overview

This document defines the data model for Cudgel, including entity relationships, database schema, validation rules, and state transitions. All data persists in PostgreSQL with pgvector extension for vector similarity search.

## Entity Relationship Diagram

```
┌─────────────────┐
│  Repository     │
│  ──────────────│
│  id (PK)        │
│  path (unique)  │
│  last_indexed   │
│  file_count     │
│  symbol_count   │
│  status         │
└────────┬────────┘
         │
         │ 1:N
         │
┌────────▼────────┐          ┌─────────────────┐
│  File           │          │ ScheduledTask   │
│  ──────────────│          │  ──────────────│
│  id (PK)        │          │  id (PK)        │
│  repo_id (FK)   │◄─────────│  repo_id (FK)   │
│  path           │          │  interval_hours │
│  content_hash   │          │  next_run_at    │
│  language       │          │  last_run_at    │
│  last_parsed    │          │  status         │
│  symbol_count   │          └─────────────────┘
└────────┬────────┘
         │                   ┌──────────────────┐
         │ 1:N               │ KnowledgeDocument│
         │                   │  ───────────────│
┌────────▼────────┐          │  id (PK)         │
│  Symbol         │          │  repo_id (FK)    │◄──┐
│  ──────────────│          │  content         │   │
│  id (PK)        │          │  generated_at    │   │
│  file_id (FK)   │          │  last_edited_at  │   │
│  name           │          │  version         │   │
│  symbol_type    │          └──────────────────┘   │
│  line_number    │                                 │ 1:1
│  code_snippet   │                                 │
│  documentation  │                                 │
└────────┬────────┘                                 │
         │                                          │
         │ 1:1                                      │
         │                                          │
┌────────▼────────┐                                 │
│  Embedding      │                                 │
│  ──────────────│                                 │
│  id (PK)        │                                 │
│  symbol_id (FK) │                                 │
│  vector(384)    │─────────────────────────────────┘
│  generated_at   │
└─────────────────┘
```

## Entities

### 1. Repository

Represents a git repository being indexed by Cudgel.

**Fields**:
- `id` (SERIAL, PRIMARY KEY): Unique identifier
- `path` (TEXT, UNIQUE, NOT NULL): Absolute filesystem path to repository root
- `last_indexed_at` (TIMESTAMPTZ): Timestamp of last successful indexing operation
- `file_count` (INTEGER, DEFAULT 0): Number of indexed files
- `symbol_count` (INTEGER, DEFAULT 0): Total number of extracted symbols
- `status` (TEXT, DEFAULT 'pending'): Current indexing status

**Validation Rules**:
- `path` must be absolute path starting with `/` (Unix) or drive letter (Windows)
- `path` must exist and be a valid git repository (`.git` directory present)
- `path` must be unique across all repositories
- `status` must be one of: `pending`, `indexing`, `completed`, `failed`
- `file_count` >= 0
- `symbol_count` >= 0

**State Transitions**:
```
pending → indexing → completed
          ↓
        failed
```

**Relationships**:
- One repository → Many files (1:N)
- One repository → Many scheduled tasks (1:N)
- One repository → One knowledge document (1:1, optional)

**Indexes**:
- Primary key on `id`
- Unique index on `path`
- Index on `status` for filtering active repositories

### 2. File

Represents a source file within a repository.

**Fields**:
- `id` (SERIAL, PRIMARY KEY): Unique identifier
- `repo_id` (INTEGER, NOT NULL, FOREIGN KEY): References `repositories.id`
- `path` (TEXT, NOT NULL): Path relative to repository root
- `content_hash` (TEXT, NOT NULL): SHA256 hash of file contents (for incremental indexing)
- `language` (TEXT, NOT NULL): Detected programming language
- `last_parsed_at` (TIMESTAMPTZ): Timestamp of last successful parse
- `symbol_count` (INTEGER, DEFAULT 0): Number of symbols extracted from this file

**Validation Rules**:
- `path` must be relative (no leading `/`)
- `path` must be unique within repository (`UNIQUE(repo_id, path)`)
- `content_hash` must be valid SHA256 hex string (64 characters)
- `language` must be one of: `python`, `javascript`, `typescript`, `rust`, `go`, `c`, `cpp`, `java`
- `symbol_count` >= 0

**Relationships**:
- Many files → One repository (N:1)
- One file → Many symbols (1:N)

**Indexes**:
- Primary key on `id`
- Unique index on `(repo_id, path)`
- Index on `content_hash` for incremental indexing checks
- Foreign key index on `repo_id`

### 3. Symbol

Represents a code construct (function, class, method, variable) extracted from a file.

**Fields**:
- `id` (SERIAL, PRIMARY KEY): Unique identifier
- `file_id` (INTEGER, NOT NULL, FOREIGN KEY): References `files.id`
- `name` (TEXT, NOT NULL): Symbol name (e.g., function name, class name)
- `symbol_type` (TEXT, NOT NULL): Type of symbol
- `line_number` (INTEGER, NOT NULL): Line number where symbol is defined
- `code_snippet` (TEXT): Excerpt of code defining the symbol (up to 500 characters)
- `documentation` (TEXT): Docstring or comment associated with symbol
- `created_at` (TIMESTAMPTZ, DEFAULT NOW()): When symbol was extracted

**Validation Rules**:
- `name` must not be empty
- `symbol_type` must be one of: `function`, `class`, `method`, `struct`, `enum`, `trait`, `interface`, `variable`, `constant`
- `line_number` > 0
- `code_snippet` max length: 500 characters
- `documentation` max length: 5000 characters

**Relationships**:
- Many symbols → One file (N:1)
- One symbol → One embedding (1:1)

**Indexes**:
- Primary key on `id`
- Index on `name` for text search
- Index on `symbol_type` for filtering
- Foreign key index on `file_id`

### 4. Embedding

Represents a vector embedding for semantic search of a symbol.

**Fields**:
- `id` (SERIAL, PRIMARY KEY): Unique identifier
- `symbol_id` (INTEGER, NOT NULL, UNIQUE, FOREIGN KEY): References `symbols.id`
- `vector` (vector(384), NOT NULL): Embedding vector from Ollama llama3.2:8b
- `generated_at` (TIMESTAMPTZ, DEFAULT NOW()): When embedding was generated

**Validation Rules**:
- `symbol_id` must be unique (one embedding per symbol)
- `vector` must have dimension 384 (llama3.2 embedding size)
- `vector` must not contain NaN or Inf values

**Relationships**:
- One embedding → One symbol (1:1)

**Indexes**:
- Primary key on `id`
- Unique index on `symbol_id`
- HNSW index on `vector` for similarity search:
  ```sql
  CREATE INDEX ON embeddings USING hnsw (vector vector_cosine_ops);
  ```

**Vector Operations**:
- Cosine similarity: `vector <=> query_vector`
- Inner product: `vector <#> query_vector`
- L2 distance: `vector <-> query_vector`

### 5. ScheduledTask

Represents a scheduled indexing job for automatic repository re-indexing.

**Fields**:
- `id` (SERIAL, PRIMARY KEY): Unique identifier
- `repo_id` (INTEGER, NOT NULL, FOREIGN KEY): References `repositories.id`
- `interval_hours` (INTEGER, NOT NULL): How often to run (in hours)
- `next_run_at` (TIMESTAMPTZ, NOT NULL): When task should run next
- `last_run_at` (TIMESTAMPTZ): When task last executed
- `status` (TEXT, DEFAULT 'active'): Task status
- `created_at` (TIMESTAMPTZ, DEFAULT NOW()): When schedule was created

**Validation Rules**:
- `interval_hours` must be > 0 and <= 8760 (max 1 year)
- `next_run_at` must be in the future when created
- `status` must be one of: `active`, `paused`, `cancelled`

**Relationships**:
- Many scheduled tasks → One repository (N:1)

**Indexes**:
- Primary key on `id`
- Index on `next_run_at` for polling queries
- Index on `status` for filtering active tasks
- Foreign key index on `repo_id`

**State Transitions**:
```
active → paused → active
       ↓
     cancelled (terminal)
```

### 6. KnowledgeDocument

Represents AI-generated structured documentation for a repository.

**Fields**:
- `id` (SERIAL, PRIMARY KEY): Unique identifier
- `repo_id` (INTEGER, NOT NULL, UNIQUE, FOREIGN KEY): References `repositories.id`
- `content` (TEXT, NOT NULL): Markdown-formatted knowledge graph
- `generated_at` (TIMESTAMPTZ, DEFAULT NOW()): When document was generated
- `last_edited_at` (TIMESTAMPTZ): When user last edited the document
- `version` (INTEGER, DEFAULT 1): Document version (increments on regenerate)

**Validation Rules**:
- `repo_id` must be unique (one knowledge doc per repository)
- `content` must not be empty
- `content` should be valid Markdown (relaxed validation, warn on parse errors)
- `version` > 0

**Relationships**:
- One knowledge document → One repository (1:1)

**Indexes**:
- Primary key on `id`
- Unique index on `repo_id`

**Content Structure** (Markdown sections):
```markdown
# Repository: [repo name]

## Design & Architecture
[AI-generated analysis of architectural patterns]

## Dependencies
[List of dependencies extracted from manifest files]

## Build Process
[How to build, test, run the project]

## Licensing
[License type, attribution requirements]
```

## Database Schema (SQL)

```sql
-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Repositories table
CREATE TABLE repositories (
    id SERIAL PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    last_indexed_at TIMESTAMPTZ,
    file_count INTEGER DEFAULT 0 CHECK (file_count >= 0),
    symbol_count INTEGER DEFAULT 0 CHECK (symbol_count >= 0),
    status TEXT DEFAULT 'pending' CHECK (status IN ('pending', 'indexing', 'completed', 'failed')),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_repositories_status ON repositories(status);

-- Files table
CREATE TABLE files (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    content_hash TEXT NOT NULL CHECK (length(content_hash) = 64),
    language TEXT NOT NULL CHECK (language IN ('python', 'javascript', 'typescript', 'rust', 'go', 'c', 'cpp', 'java')),
    last_parsed_at TIMESTAMPTZ,
    symbol_count INTEGER DEFAULT 0 CHECK (symbol_count >= 0),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(repo_id, path)
);

CREATE INDEX idx_files_repo_id ON files(repo_id);
CREATE INDEX idx_files_content_hash ON files(content_hash);

-- Symbols table
CREATE TABLE symbols (
    id SERIAL PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (name != ''),
    symbol_type TEXT NOT NULL CHECK (symbol_type IN ('function', 'class', 'method', 'struct', 'enum', 'trait', 'interface', 'variable', 'constant')),
    line_number INTEGER NOT NULL CHECK (line_number > 0),
    code_snippet TEXT CHECK (length(code_snippet) <= 500),
    documentation TEXT CHECK (length(documentation) <= 5000),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_symbols_file_id ON symbols(file_id);
CREATE INDEX idx_symbols_name ON symbols(name);
CREATE INDEX idx_symbols_type ON symbols(symbol_type);

-- Embeddings table (pgvector)
CREATE TABLE embeddings (
    id SERIAL PRIMARY KEY,
    symbol_id INTEGER NOT NULL UNIQUE REFERENCES symbols(id) ON DELETE CASCADE,
    vector vector(384) NOT NULL,
    generated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_embeddings_symbol_id ON embeddings(symbol_id);
CREATE INDEX ON embeddings USING hnsw (vector vector_cosine_ops);

-- Scheduled tasks table
CREATE TABLE scheduled_tasks (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    interval_hours INTEGER NOT NULL CHECK (interval_hours > 0 AND interval_hours <= 8760),
    next_run_at TIMESTAMPTZ NOT NULL,
    last_run_at TIMESTAMPTZ,
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'paused', 'cancelled')),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_scheduled_tasks_repo_id ON scheduled_tasks(repo_id);
CREATE INDEX idx_scheduled_tasks_next_run ON scheduled_tasks(next_run_at) WHERE status = 'active';
CREATE INDEX idx_scheduled_tasks_status ON scheduled_tasks(status);

-- Knowledge documents table
CREATE TABLE knowledge_documents (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER NOT NULL UNIQUE REFERENCES repositories(id) ON DELETE CASCADE,
    content TEXT NOT NULL CHECK (content != ''),
    generated_at TIMESTAMPTZ DEFAULT NOW(),
    last_edited_at TIMESTAMPTZ,
    version INTEGER DEFAULT 1 CHECK (version > 0)
);

CREATE INDEX idx_knowledge_documents_repo_id ON knowledge_documents(repo_id);
```

## Migrations

Migrations are managed via sqlx and stored in `migrations/` directory:

1. **001_init.sql**: Create base tables (repositories, files, symbols)
2. **002_pgvector.sql**: Enable pgvector extension, create embeddings table with HNSW index
3. **003_schedules.sql**: Create scheduled_tasks table
4. **004_knowledge.sql**: Create knowledge_documents table

Each migration is idempotent (uses `IF NOT EXISTS`, `CREATE OR REPLACE`) and reversible (includes DOWN migration).

## Data Lifecycle

### Indexing Flow
1. User runs `cudgel index /path/to/repo`
2. Create or update `repositories` row (status: `indexing`)
3. List git-tracked files, create/update `files` rows
4. For each changed file (content_hash different):
   - Parse with tree-sitter
   - Delete old `symbols` (CASCADE deletes `embeddings`)
   - Insert new `symbols`
   - Generate `embeddings` via Ollama
5. Update `repositories` (status: `completed`, file_count, symbol_count)

### Query Flow
1. User runs `cudgel query "search term"`
2. Generate query embedding via Ollama
3. Execute pgvector similarity search on `embeddings` table
4. Join with `symbols`, `files`, `repositories` to get full context
5. Return top N results sorted by similarity

### Scheduling Flow
1. User runs `cudgel index --schedule hourly /path/to/repo`
2. Create `repositories` row if not exists
3. Create `scheduled_tasks` row with `interval_hours=1`, `next_run_at=NOW() + 1 hour`
4. Start orchestrator daemon if not running
5. Daemon polls `scheduled_tasks` every 60s for tasks where `next_run_at <= NOW()`
6. Execute indexing, update `next_run_at += interval_hours`

### Knowledge Generation Flow
1. User runs `cudgel knowledge`
2. Query indexed data: dependencies, architectural patterns, file structure
3. Call Ollama with prompt + aggregated data
4. Parse LLM response into markdown sections
5. Insert/update `knowledge_documents` row
6. Open content in `$EDITOR`
7. On editor close, update `last_edited_at`, increment `version`

## Cleanup and Maintenance

**Cascade Deletes**:
- Deleting a `repository` CASCADE deletes all `files`, `symbols`, `embeddings`, `scheduled_tasks`, `knowledge_documents`
- Deleting a `file` CASCADE deletes all `symbols` and their `embeddings`
- Deleting a `symbol` CASCADE deletes its `embedding`

**Orphan Prevention**:
- All foreign keys have `ON DELETE CASCADE`
- No orphaned embeddings (enforced by UNIQUE constraint on `symbol_id`)

**Index Maintenance**:
- Run `ANALYZE` after bulk inserts (>1000 rows) to update statistics
- Rebuild HNSW index if degraded: `REINDEX INDEX embeddings_vector_idx`
- Monitor index size: `SELECT pg_size_pretty(pg_relation_size('embeddings_vector_idx'))`

**Data Retention**:
- No automatic cleanup (user controls data by unscheduling/deleting repos)
- Provide `cudgel clean` command to remove unscheduled repos older than N days

## Summary

The data model follows normalized relational design with PostgreSQL as single source of truth:
- 6 core entities with clear relationships
- Referential integrity via foreign keys with cascade deletes
- Validation constraints at database level
- pgvector extension for efficient similarity search
- Migration-based schema evolution for versioning

All entities align with constitution principles (PostgreSQL exclusivity, ACID transactions, no file-based persistence).
