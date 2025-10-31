# Database Schema

## Connection Details

- **Host**: localhost
- **Port**: 54321 (cudgel-specific, avoids conflicts)
- **Database**: cudgel
- **User**: Your system username
- **Location**: `~/.local/share/cudgel/postgres`

## Tables

### repositories

Stores indexed repository metadata.

```sql
CREATE TABLE repositories (
    id SERIAL PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    indexed_at TIMESTAMP NOT NULL DEFAULT NOW(),
    commit_hash TEXT,
    metadata JSONB DEFAULT '{}'
);
```

### files

Source files with content and metadata.

```sql
CREATE TABLE files (
    id SERIAL PRIMARY KEY,
    repository_id INTEGER REFERENCES repositories(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    content TEXT NOT NULL,
    language TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    size INTEGER NOT NULL,
    indexed_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(repository_id, path)
);
```

### ast_nodes

Complete AST tree structure.

```sql
CREATE TABLE ast_nodes (
    id SERIAL PRIMARY KEY,
    file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
    parent_id INTEGER REFERENCES ast_nodes(id) ON DELETE CASCADE,
    node_type TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    start_column INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    end_column INTEGER NOT NULL,
    text TEXT
);

CREATE INDEX idx_ast_nodes_parent ON ast_nodes(parent_id);
CREATE INDEX idx_ast_nodes_file ON ast_nodes(file_id);
```

### symbols

Functions, classes, methods with embeddings.

```sql
CREATE TABLE symbols (
    id SERIAL PRIMARY KEY,
    file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    kind TEXT NOT NULL, -- 'function', 'class', 'struct', etc.
    line INTEGER NOT NULL,
    column INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    end_column INTEGER NOT NULL,
    signature TEXT,
    docstring TEXT,
    embedding vector(384), -- pgvector extension
    metadata JSONB DEFAULT '{}'
);

CREATE INDEX idx_symbols_embedding ON symbols
USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
```

### references

Graph edges representing symbol relationships.

```sql
CREATE TABLE "references" (
    id SERIAL PRIMARY KEY,
    from_symbol_id INTEGER REFERENCES symbols(id) ON DELETE CASCADE,
    to_symbol_id INTEGER REFERENCES symbols(id) ON DELETE CASCADE,
    reference_type TEXT NOT NULL, -- 'calls', 'imports', 'extends', etc.
    file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
    line INTEGER NOT NULL,
    "column" INTEGER NOT NULL,
    metadata JSONB DEFAULT '{}',
    UNIQUE(from_symbol_id, to_symbol_id, reference_type, line, "column")
);

CREATE INDEX idx_references_from ON "references"(from_symbol_id);
CREATE INDEX idx_references_to ON "references"(to_symbol_id);
```

### code_chunks

Code segments with embeddings for semantic search.

```sql
CREATE TABLE code_chunks (
    id SERIAL PRIMARY KEY,
    file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    content TEXT NOT NULL,
    embedding vector(384),
    metadata JSONB DEFAULT '{}'
);

CREATE INDEX idx_code_chunks_embedding ON code_chunks
USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
```

## Indexes

- **B-tree indexes**: For foreign keys, lookups
- **IVFFlat indexes**: For vector similarity search
  - `lists = 100`: Number of clusters for approximate search
  - `vector_cosine_ops`: Cosine similarity operator

## Vector Search

Uses pgvector extension for similarity search:

```sql
SELECT name, kind, file_path, line,
       1 - (embedding <=> $1) AS similarity
FROM symbols
WHERE embedding IS NOT NULL
ORDER BY embedding <=> $1
LIMIT 10;
```

- `<=>` operator: Cosine distance (1 - cosine similarity)
- Returns results ordered by similarity (highest first)

## Migrations

Schema is auto-initialized on first connection via `Database::initialize_schema()`.

No migration system currently - schema is stable.

## Backup/Restore

```bash
# Backup
pg_dump -h localhost -p 54321 -U $USER cudgel > cudgel_backup.sql

# Restore
psql -h localhost -p 54321 -U $USER cudgel < cudgel_backup.sql

# Or use task commands
task db-clean   # Remove all data
task setup      # Reinitialize
```

## Performance Tuning

Current settings in `postgresql.conf`:
- `shared_buffers = 128MB`
- `max_connections = 100`

For large repositories, consider:
- Increasing `shared_buffers`
- Tuning `work_mem` for sorts
- Adjusting IVFFlat `lists` parameter based on data size
