# Cudgel

A zero-configuration code indexing tool that combines tree-sitter parsing, PostgreSQL/pgvector embeddings, Temporal workflows, and LSP integration for intelligent code search and analysis.

**No setup required** - just run `cudgel index .` and everything auto-starts! 🚀

## Features

- 🔍 **Natural Language Search**: Find code using plain English queries
- 🌳 **Tree-sitter Parsing**: Multi-language AST parsing (Python, JS/TS, Rust, Go, C/C++, Java)
- 🔗 **Code Relationships**: Track and query references and call graphs
- ⏰ **Auto-Scheduling**: Periodic re-indexing with `--schedule` flag
- 🚀 **Zero Config**: No env files, no manual database setup
- 🐳 **Auto-Services**: PostgreSQL + Temporal auto-start via Docker
- 💻 **Rich CLI**: Beautiful terminal UI with progress bars and tables

## Quick Start

### Prerequisites

- **Rust 1.70+** - [Install Rust](https://rustup.rs/)
- **Docker** - [Install Docker](https://docs.docker.com/get-docker/)

That's it! Everything else is automatic.

### Installation

```bash
git clone https://github.com/roshbhatia/cudgel.git
cd cudgel
cargo install --path .
```

### Usage

**Index a repository** (auto-starts services):
```bash
cudgel index /path/to/repo
```

**Index with scheduled re-indexing:**
```bash
cudgel index . --schedule hourly   # every hour
cudgel index . --schedule daily    # every 24 hours
cudgel index . --schedule 6        # every 6 hours
```

**Search your code:**
```bash
cudgel query "function that handles authentication"
cudgel query "error handling" --limit 5
```

**Explore relationships:**
```bash
cudgel graph authenticate_user --depth 2
```

See [QUICKSTART.md](QUICKSTART.md) for a complete guide.

## Architecture

```
┌─────────────┐
│   CLI       │  Your commands
└──────┬──────┘
       │
┌──────▼──────────────────────────┐
│    Auto-Service Manager         │
│  (Docker Compose)               │
│   ├─ PostgreSQL + pgvector      │
│   └─ Temporal                   │
└──────┬──────────────────────────┘
       │
┌──────▼──────────────────────────┐
│    Cudgel Core Engine           │
│  ┌──────────┐    ┌───────────┐ │
│  │ Tree-    │───▶│  Indexer  │ │
│  │ sitter   │    └─────┬─────┘ │
│  └──────────┘          │        │
│  ┌──────────┐    ┌─────▼─────┐ │
│  │Embedding │───▶│  Database │ │
│  │Generator │    │  (auto-   │ │
│  └──────────┘    │   init)   │ │
│                  └─────┬─────┘ │
│  ┌──────────┐    ┌─────▼─────┐ │
│  │  Query   │───▶│   Graph   │ │
│  │  Engine  │    │   Query   │ │
│  └──────────┘    └───────────┘ │
└─────────────────────────────────┘
```

## How It Works

1. **First Run**: When you run `cudgel index`, it automatically:
   - Starts PostgreSQL + Temporal via Docker (~30 seconds first time)
   - Initializes the database schema
   - Indexes your code

2. **Parsing**: Uses tree-sitter to parse source files into ASTs

3. **Symbol Extraction**: Extracts functions, classes, methods, etc. from ASTs

4. **Embeddings**: Generates vector embeddings for semantic search

5. **Storage**: Stores everything in PostgreSQL with pgvector for fast similarity search

6. **Scheduling** (optional): Uses Temporal workflows for periodic re-indexing

## Supported Languages

- Python (`.py`)
- JavaScript/JSX (`.js`, `.jsx`)
- TypeScript/TSX (`.ts`, `.tsx`)
- Rust (`.rs`)
- Go (`.go`)
- C/C++ (`.c`, `.cpp`, `.h`, `.hpp`)
- Java (`.java`)

## Development

### Quick Setup

```bash
# Using Task (recommended)
task --list           # See available tasks
task build            # Build project
task test             # Run tests
task pre-commit       # Run all checks

# Using cargo directly
cargo build           # Build
cargo test            # Test
cargo clippy          # Lint
cargo fmt             # Format
```

### Development Tools

We provide comprehensive dev tooling:

- **Nix Shell** (`shell.nix`) - Reproducible dev environment
- **Task** (`Taskfile.yml`) - Task automation (build, test, lint, etc.)
- **Pre-commit Hooks** (`.pre-commit-config.yaml`) - Automatic quality checks
- **GitHub Actions** (`.github/workflows/`) - CI/CD pipelines

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed development guidelines.

### With Nix

```bash
nix-shell              # Enter dev shell
task install-hooks     # Setup git hooks
task build            # Build
task test             # Test
```

### Pre-commit Hooks

```bash
# Install hooks (one-time)
task install-hooks

# Hooks will automatically run on commit:
# - Format code (cargo fmt)
# - Lint (cargo clippy)
# - Run tests (on push)
```

## Project Structure

```
cudgel/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── config.rs        # Configuration (hardcoded local defaults)
│   ├── services.rs      # Auto-managed Docker services
│   ├── database.rs      # PostgreSQL + pgvector operations
│   ├── indexer.rs       # Repository indexing orchestration
│   ├── parser.rs        # Tree-sitter parsing
│   ├── embeddings.rs    # Vector embedding generation
│   ├── query.rs         # Natural language search
│   ├── graph.rs         # Code relationship analysis
│   ├── lsp.rs           # LSP server implementation
│   ├── temporal.rs      # Temporal workflow integration
│   └── error.rs         # Error types
├── tests/
│   └── integration_tests.rs
├── Taskfile.yml         # Task automation
├── shell.nix            # Nix development environment
├── .pre-commit-config.yaml
├── .github/workflows/   # CI/CD
├── CLAUDE.md            # Development guide for AI assistants
├── CONTRIBUTING.md      # Contribution guidelines
└── QUICKSTART.md        # Quick start guide
```

## CLI Commands

```bash
# Index repositories
cudgel index <path>                    # Index once
cudgel index <path> --schedule hourly  # With periodic re-indexing

# Search code
cudgel query "search term"             # Basic search
cudgel query "term" --limit 10         # Limit results
cudgel query "term" --json             # JSON output

# Explore relationships
cudgel graph <symbol>                  # Show relationships
cudgel graph <symbol> --depth 3        # Deep traversal
cudgel graph <symbol> --json           # JSON output

# LSP server (for IDE integration)
cudgel lsp                             # Start LSP server

# Database management (optional)
cudgel init-db                         # Manual schema init
cudgel schedule <path>                 # Standalone scheduling
```

## Configuration

**No configuration needed!** Everything uses hardcoded local defaults:

- PostgreSQL: `localhost:5432`, user/pass/db = `cudgel`
- Temporal: `localhost:7233`
- Embeddings: Dummy vectors (384D) for development

All services auto-start via Docker when first needed.

## Testing

```bash
# Run all tests
cargo test

# Run specific tests
cargo test test_name

# Integration tests (requires PostgreSQL)
cargo test --test integration_tests

# With Task
task test              # All tests
task test-unit         # Unit tests only
task test-integration  # Integration tests only
```

## CI/CD

We use GitHub Actions for continuous integration:

- **CI Pipeline** (`.github/workflows/ci.yml`):
  - Runs on push/PR
  - Tests on Linux and macOS
  - Runs fmt, clippy, and tests
  - Security audit

- **Release Pipeline** (`.github/workflows/release.yml`):
  - Triggers on version tags (`v*`)
  - Builds binaries for all platforms
  - Creates GitHub releases
  - Publishes to crates.io

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for:

- Development setup
- Code style guidelines
- Testing requirements
- Pull request process

Quick contribution workflow:

```bash
# 1. Fork and clone
git clone https://github.com/YOUR_USERNAME/cudgel.git
cd cudgel

# 2. Setup development environment
task install-hooks

# 3. Create a feature branch
git checkout -b feature/my-feature

# 4. Make changes and test
task pre-commit

# 5. Commit and push
git commit -m "feat: Add my feature"
git push origin feature/my-feature

# 6. Create Pull Request on GitHub
```

## Roadmap

- [ ] Production-ready ONNX embeddings
- [ ] More language support (Ruby, PHP, Swift, Kotlin)
- [ ] VSCode extension
- [ ] Web UI for browsing code
- [ ] Incremental indexing (only changed files)
- [ ] Cross-repository search
- [ ] AI-powered code explanations

## License

[MIT License](LICENSE) - See LICENSE file for details

## Credits

Built with:
- [tree-sitter](https://tree-sitter.github.io/) - Incremental parsing
- [PostgreSQL](https://www.postgresql.org/) + [pgvector](https://github.com/pgvector/pgvector) - Vector database
- [Temporal](https://temporal.io/) - Workflow orchestration
- [Rust](https://www.rust-lang.org/) - Systems programming language

## Links

- **Documentation**: [QUICKSTART.md](QUICKSTART.md), [CLAUDE.md](CLAUDE.md)
- **Issues**: [GitHub Issues](https://github.com/roshbhatia/cudgel/issues)
- **Discussions**: [GitHub Discussions](https://github.com/roshbhatia/cudgel/discussions)

---

Made with ❤️ and Claude Code
