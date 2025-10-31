# Cudgel Architecture

## Overview

Cudgel is a code indexing tool that uses tree-sitter for parsing, PostgreSQL with pgvector for storage and semantic search, and provides a clean CLI interface.

## Core Components

### 1. Parser (`src/parser.rs`)
- **Purpose**: Parse source code into AST using tree-sitter
- **Languages**: Python, JavaScript/JSX, TypeScript/TSX, Rust, Go, C/C++, Java
- **Output**: Symbols (functions, classes, structs) with location information

### 2. Indexer (`src/indexer.rs`)
- **Purpose**: Orchestrate repository indexing
- **Process**:
  1. Walk directory tree
  2. Filter files by language
  3. Parse each file
  4. Extract symbols
  5. Generate embeddings
  6. Store in database
- **Stats**: Tracks files processed, symbols found, errors

### 3. Database (`src/database.rs`)
- **Backend**: PostgreSQL 17 on port 54321
- **Extensions**: pgvector for vector similarity search
- **Schema**:
  - `repositories`: Indexed repos
  - `files`: Source files with content and hash
  - `ast_nodes`: Complete AST tree structure
  - `symbols`: Functions/classes with embeddings (384-dim vectors)
  - `references`: Graph edges for symbol relationships
  - `code_chunks`: Code segments with embeddings

### 4. Embeddings (`src/embeddings.rs`)
- **Current**: Dummy embeddings (random vectors)
- **Production**: ONNX runtime with sentence-transformers
- **Dimension**: 384 (compatible with all-MiniLM-L6-v2)

### 5. Query Engine (`src/query.rs`)
- **Method**: Vector similarity search using cosine distance
- **Index**: IVFFlat for approximate nearest neighbor
- **Returns**: Ranked results with similarity scores

### 6. Graph Query (`src/graph.rs`)
- **Purpose**: Explore code relationships and call graphs
- **Features**:
  - Configurable depth traversal
  - Bidirectional relationships
  - Graph visualization data

### 7. CLI (`src/main.rs`)
- **Framework**: clap with derive macros
- **Commands**:
  - `index`: Index a repository
  - `query`: Search indexed code
  - `graph`: Explore relationships
  - `init-db`: Initialize database schema
  - `lsp`: Start LSP server

## Data Flow

```
Source Code
    ↓
[Parser] → AST Nodes + Symbols
    ↓
[Embeddings] → Vector Representations
    ↓
[Database] → PostgreSQL + pgvector
    ↓
[Query Engine] → Similarity Search
    ↓
Results
```

## Database Connection

- **Port**: 54321 (non-standard to avoid conflicts)
- **Location**: `~/.local/share/cudgel/postgres`
- **Configuration**: Hardcoded in `src/config.rs`
- **Connection Pool**: deadpool-postgres with 10 connections

## Storage

All data stored in PostgreSQL:
- **File content**: SHA256-hashed for deduplication
- **Vectors**: 384-dimensional float arrays
- **Indexes**: B-tree for standard columns, IVFFlat for vectors

## Security

- Local-only design (localhost)
- No authentication (trusted local user)
- No network exposure
- Data stored in user's home directory

## Performance

- **Async**: Tokio runtime throughout
- **Parallel**: Concurrent file processing
- **Batching**: 100 symbols per transaction
- **Vector Search**: Approximate (IVFFlat) for speed

## Future Enhancements

1. Real ONNX embeddings
2. Temporal workflows for scheduled indexing
3. LSP server for IDE integration
4. Cross-repository search
5. Symbol usage analytics
