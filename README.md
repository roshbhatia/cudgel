# Cudgel

A powerful code indexing tool that combines tree-sitter parsing, PostgreSQL/pgvector embeddings, Temporal workflows, and LSP integration for intelligent code search and analysis.

## Features

- **Tree-sitter Parsing**: Multi-language AST parsing with support for Python, JavaScript, TypeScript, Rust, Go, C/C++, Java, and more
- **Vector Embeddings**: Semantic code search using sentence transformers and pgvector
- **Graph Relationships**: Track and query code relationships (references, call graphs) as a graph database
- **Temporal Workflows**: Schedule and automate code indexing workflows
- **LSP Integration**: Language Server Protocol support for IDE integration
- **Natural Language Queries**: Search code using natural language descriptions
- **Rich CLI**: Beautiful command-line interface with syntax highlighting and visualizations

## Architecture

```
┌─────────────┐
│   CLI/LSP   │
└──────┬──────┘
       │
┌──────▼──────────────────────────────┐
│        Cudgel Core Engine           │
│  ┌──────────┐    ┌──────────────┐  │
│  │ Tree-    │───▶│  Indexer     │  │
│  │ sitter   │    └──────┬───────┘  │
│  └──────────┘           │           │
│                         │           │
│  ┌──────────┐    ┌──────▼───────┐  │
│  │Embedding │───▶│  Query       │  │
│  │Generator │    │  Engine      │  │
│  └──────────┘    └──────────────┘  │
└──────────────┬──────────────────────┘
               │
    ┌──────────▼──────────┐
    │   PostgreSQL +      │
    │     pgvector        │
    │                     │
    │  ┌──────────────┐   │
    │  │ AST Nodes    │   │
    │  ├──────────────┤   │
    │  │ Symbols      │   │
    │  ├──────────────┤   │
    │  │ References   │   │
    │  ├──────────────┤   │
    │  │ Embeddings   │   │
    │  └──────────────┘   │
    └─────────────────────┘
```

## Installation

### Prerequisites

1. **Rust 1.70+** (for the CLI)
2. **PostgreSQL 14+** with pgvector extension
3. **Python 3.10+** (optional, for Python library)
4. **Temporal Server** (optional, for scheduled indexing)

### Install Rust

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Install PostgreSQL and pgvector

```bash
# Ubuntu/Debian
sudo apt-get install postgresql postgresql-contrib
sudo apt-get install postgresql-14-pgvector

# macOS (Homebrew)
brew install postgresql@14
brew install pgvector

# Start PostgreSQL
sudo systemctl start postgresql  # Linux
brew services start postgresql@14  # macOS
```

### Install Temporal (Optional)

```bash
# Using Docker
docker run -d -p 7233:7233 temporalio/auto-setup:latest

# Or install Temporal CLI
brew install temporal  # macOS
```

### Install Cudgel

```bash
# Clone the repository
git clone https://github.com/your-org/cudgel.git
cd cudgel

# Build and install the Rust CLI
cargo build --release
cargo install --path .

# Or use it directly
cargo run --release -- --help

# Optional: Install Python library
pip install -e .
```

## Configuration

Create a `.env` file in your project directory:

```bash
# Database Configuration
CUDGEL_DB_HOST=localhost
CUDGEL_DB_PORT=5432
CUDGEL_DB_NAME=cudgel
CUDGEL_DB_USER=cudgel
CUDGEL_DB_PASSWORD=your_secure_password

# Temporal Configuration (optional)
CUDGEL_TEMPORAL_HOST=localhost:7233
CUDGEL_TEMPORAL_NAMESPACE=default
CUDGEL_TEMPORAL_TASK_QUEUE=cudgel-indexing

# Embedding Model
CUDGEL_EMBEDDING_MODEL=sentence-transformers/all-MiniLM-L6-v2
CUDGEL_EMBEDDING_DIMENSION=384

# LSP Server
CUDGEL_LSP_PORT=6010
```

### Database Setup

```bash
# Create PostgreSQL database and user
sudo -u postgres psql

postgres=# CREATE USER cudgel WITH PASSWORD 'your_secure_password';
postgres=# CREATE DATABASE cudgel OWNER cudgel;
postgres=# \c cudgel
cudgel=# CREATE EXTENSION vector;
cudgel=# \q

# Initialize cudgel schema
cudgel init-db
```

## Usage

The Cudgel CLI is written in Rust for performance and reliability.

### Index a Repository

```bash
# Index the current directory
cudgel index .

# Index a specific repository
cudgel index /path/to/repo

# Index with a custom name (coming soon)
cudgel index /path/to/repo --name "my-project"
```

### Query Code with Natural Language

```bash
# Search for symbols and code
cudgel query "function that handles user authentication"

# Search only symbols
cudgel query "authentication handler" --type symbols

# Search only code chunks
cudgel query "parse JWT token" --type code

# Limit results
cudgel query "database connection" --limit 5

# Filter by repository
cudgel query "error handling" --repo /path/to/repo

# Output as JSON
cudgel query "API endpoints" --json-output
```

### Explore Graph Relationships

```bash
# Get references for a symbol
cudgel graph authenticate_user

# Get call graph (what this function calls)
cudgel graph process_request --type calls --direction outgoing

# Get reverse call graph (what calls this function)
cudgel graph save_user --type calls --direction incoming

# Get bidirectional call graph
cudgel graph handle_request --type calls --direction both

# Traverse deeper
cudgel graph UserService --depth 3

# Output as JSON
cudgel graph login --json-output
```

### Start LSP Server

```bash
# Start LSP server for IDE integration
cudgel lsp

# Specify host and port
cudgel lsp --host 0.0.0.0 --port 6010
```

## LSP Integration

### VS Code

Create `.vscode/settings.json`:

```json
{
  "cudgel.lsp.enabled": true,
  "cudgel.lsp.serverPath": "cudgel",
  "cudgel.lsp.port": 6010
}
```

### Neovim

Using `nvim-lspconfig`:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.cudgel then
  configs.cudgel = {
    default_config = {
      cmd = {'cudgel', 'lsp'},
      filetypes = {'python', 'javascript', 'typescript', 'rust', 'go'},
      root_dir = lspconfig.util.root_pattern('.git'),
    },
  }
end

lspconfig.cudgel.setup{}
```

## Temporal Workflows

### Start Temporal Worker

```python
import asyncio
from cudgel.temporal_workflows import start_temporal_worker

asyncio.run(start_temporal_worker())
```

### Schedule Repository Indexing

```python
import asyncio
from cudgel.temporal_workflows import schedule_repository_indexing, schedule_periodic_indexing

# One-time indexing
async def index_once():
    workflow_id = await schedule_repository_indexing("/path/to/repo", "my-repo")
    print(f"Scheduled workflow: {workflow_id}")

# Periodic indexing (every 24 hours)
async def index_periodically():
    workflow_id = await schedule_periodic_indexing("/path/to/repo", interval_hours=24)
    print(f"Scheduled periodic workflow: {workflow_id}")

asyncio.run(index_once())
```

## API Usage

### Rust API

```rust
use cudgel::{Config, Database, Indexer, QueryEngine, GraphQuery};
use std::sync::Arc;
use std::path::Path;

#[tokio::main]
async fn main() -> cudgel::Result<()> {
    let config = Arc::new(Config::from_env()?);

    // Index a repository
    let db = Arc::new(Database::new(&config).await?);
    let mut indexer = Indexer::new(config.clone(), db.clone())?;
    let repo_id = indexer.index_repository(Path::new("/path/to/repo")).await?;

    // Query code
    let query_engine = QueryEngine::new(config.clone(), db.clone())?;
    let results = query_engine.search_symbols("database connection", 10, None).await?;

    // Query graph relationships
    let graph_query = GraphQuery::new(db);
    let call_graph = graph_query.get_references("process_request", None, 2).await?;

    Ok(())
}
```

### Python API (Optional)

The Python library is still available for embedding generation and analysis:

```python
import asyncio
from pathlib import Path
from cudgel.config import get_config
from cudgel.indexer import CodeIndexer
from cudgel.query import CodeQuery
from cudgel.graph import GraphQuery

async def main():
    config = get_config()

    # Index a repository
    indexer = CodeIndexer(config)
    await indexer.initialize()
    repo_id = await indexer.index_repository(Path("/path/to/repo"))
    await indexer.close()

    # Query code
    query_engine = CodeQuery(config)
    await query_engine.initialize()
    results = await query_engine.search("database connection", limit=10)
    print(results)
    await query_engine.close()

    # Query graph relationships
    graph = GraphQuery(config)
    await graph.initialize()
    call_graph = await graph.get_call_graph("process_request")
    print(call_graph)
    await graph.close()

asyncio.run(main())
```

## Database Schema

### Tables

- **repositories**: Indexed repositories
- **files**: Source files with content and metadata
- **ast_nodes**: AST tree structure with parent-child relationships
- **symbols**: Functions, classes, variables with embeddings
- **references**: Graph edges between symbols
- **code_chunks**: Code segments with embeddings for semantic search

### Indexes

- Vector indexes on embeddings (IVFFlat) for fast similarity search
- B-tree indexes on relationships for graph traversal
- Composite indexes for common queries

## Performance

- **Indexing Speed**: ~100-500 files/minute (depends on file size)
- **Query Latency**: <100ms for semantic search (with warm cache)
- **Graph Traversal**: <50ms for depth-3 queries
- **Storage**: ~2-5MB per 1000 LOC (including embeddings)

## Supported Languages

- Python
- JavaScript/JSX
- TypeScript/TSX
- Rust
- Go
- C/C++
- Java
- C#
- Ruby
- PHP
- Swift
- Kotlin

More languages can be added via tree-sitter grammars.

## Development

### Rust Development

```bash
# Build
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy

# Build release
cargo build --release
```

### Python Development (Optional)

```bash
# Install development dependencies
pip install -e ".[dev]"

# Run tests
pytest

# Format code
black src/

# Lint
ruff check src/

# Type check
mypy src/
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

MIT License - see LICENSE file for details

## Acknowledgments

- [tree-sitter](https://tree-sitter.github.io/) for AST parsing
- [pgvector](https://github.com/pgvector/pgvector) for vector similarity search
- [Temporal](https://temporal.io/) for workflow orchestration
- [sentence-transformers](https://www.sbert.net/) for embeddings
- [pygls](https://github.com/openlawlibrary/pygls) for LSP implementation
