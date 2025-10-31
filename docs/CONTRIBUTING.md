# Contributing to Cudgel

Thanks for your interest in contributing to Cudgel! This guide will help you get started.

## Development Setup

### Prerequisites

- **Rust 1.70+** - [Install Rust](https://rustup.rs/)
- **Docker** - [Install Docker](https://docs.docker.com/get-docker/)
- **Task** (optional) - [Install Task](https://taskfile.dev/)
- **Nix** (optional) - [Install Nix](https://nixos.org/download.html)

### Quick Start

#### Using Nix (Recommended)

```bash
# Enter development shell with all dependencies
nix-shell

# Install git hooks
task install-hooks

# Run tests
task test
```

#### Without Nix

```bash
# Install Task
go install github.com/go-task/task/v3/cmd/task@latest
# or: brew install go-task

# Install git hooks
task install-hooks

# Build and test
task build
task test
```

## Development Workflow

### Using Task (Recommended)

We use [Task](https://taskfile.dev/) as our task runner. See available tasks:

```bash
task --list
```

Common workflows:

```bash
# Quick dev cycle (format + check)
task quick

# Full pre-commit checks (format + lint + test)
task pre-commit

# Run CI locally
task ci

# Build release binary
task build-release
```

### Manual Commands

If you prefer not to use Task:

```bash
# Format code
cargo fmt

# Run linter
cargo clippy --all-targets -- -D warnings

# Run tests
cargo test

# Build
cargo build

# Build release
cargo build --release
```

## Git Workflow

### Branches

- `main` - Stable, production-ready code
- `develop` - Integration branch for features
- `feature/*` - Feature branches
- `fix/*` - Bug fix branches

### Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: Add new feature
fix: Fix bug
docs: Update documentation
style: Format code
refactor: Refactor code
test: Add tests
chore: Update dependencies
```

Examples:
```
feat: Add --schedule flag to index command
fix: Auto-initialize database schema on first connection
docs: Update QUICKSTART with zero-config approach
test: Add integration tests for service auto-start
```

### Pre-commit Hooks

We use pre-commit hooks to ensure code quality:

```bash
# Install hooks (one-time setup)
task install-hooks

# Or manually with pre-commit
pre-commit install

# Run hooks manually
pre-commit run --all-files
```

The hooks will:
1. Format code with `cargo fmt`
2. Run `cargo clippy` (no warnings allowed)
3. Run tests (on push)

### Pull Request Process

1. **Fork and clone** the repository
2. **Create a branch** from `develop`:
   ```bash
   git checkout -b feature/my-feature develop
   ```
3. **Make your changes** with clear, focused commits
4. **Run pre-commit checks**:
   ```bash
   task pre-commit
   ```
5. **Push your branch** and create a Pull Request
6. **Ensure CI passes** - All GitHub Actions workflows must pass
7. **Request review** - Wait for maintainer review
8. **Address feedback** and update your PR

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration_tests

# Or with Task
task test-integration
```

Integration tests require PostgreSQL. They will automatically skip if PostgreSQL is unavailable.

### Test Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --out Html
```

## Code Quality

### Formatting

```bash
# Format all code
cargo fmt

# Check formatting
cargo fmt -- --check
```

### Linting

```bash
# Run clippy
cargo clippy --all-targets -- -D warnings

# Fix auto-fixable issues
cargo clippy --fix
```

### Security Audit

```bash
# Install cargo-audit
cargo install cargo-audit

# Run audit
cargo audit
```

## Documentation

### Code Documentation

- Add doc comments to all public APIs
- Include examples in doc comments when possible
- Run `cargo doc` to verify documentation builds

```bash
# Generate and open docs
cargo doc --open --no-deps
```

### User Documentation

- Update `README.md` for user-facing changes
- Update `CLAUDE.md` for development guidance
- Update `QUICKSTART.md` for quick start changes

## Architecture

### Key Modules

- **`src/parser.rs`** - Tree-sitter AST parsing
- **`src/indexer.rs`** - Repository indexing orchestration
- **`src/database.rs`** - PostgreSQL + pgvector operations
- **`src/services.rs`** - Auto-managed Docker services
- **`src/query.rs`** - Natural language code search
- **`src/graph.rs`** - Code relationship analysis

### Design Principles

1. **Zero-config**: No manual setup required
2. **Auto-management**: Services start automatically
3. **Good enough > perfect**: Pragmatic over extensible
4. **Local-only**: No remote database support needed
5. **Fail fast**: Clear error messages

## Release Process

Releases are automated via GitHub Actions:

1. **Update version** in `Cargo.toml`
2. **Update CHANGELOG.md** with changes
3. **Commit and tag**:
   ```bash
   git commit -m "chore: Release v0.2.0"
   git tag v0.2.0
   git push origin main
   git push origin v0.2.0
   ```
4. **GitHub Actions** will automatically:
   - Build release binaries for all platforms
   - Create GitHub Release
   - Publish to crates.io (if configured)

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/roshbhatia/cudgel/issues)
- **Discussions**: [GitHub Discussions](https://github.com/roshbhatia/cudgel/discussions)
- **Code**: Read `CLAUDE.md` for detailed technical guidance

## Code of Conduct

Be respectful, professional, and constructive. We're all here to build something useful together.
