# Testing Guide

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_database_connection

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test integration_tests
```

## Test Structure

### Unit Tests

Located in each source file using `#[cfg(test)]` modules.

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_detects_language() {
        let parser = Parser::new();
        assert_eq!(parser.detect_language("test.py"), Some("python"));
    }
}
```

### Integration Tests

Located in `tests/` directory.

Current tests:
- `tests/integration_tests.rs`: End-to-end workflows

## Test Database

Tests use the same database configuration (port 54321).

**Important**: Tests may modify the database. Run `task db-clean && task setup` to reset.

## Test Coverage

Run with coverage reporting:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --out Html --output-dir coverage
```

## Writing Tests

### Testing Database Operations

```rust
#[tokio::test]
async fn test_insert_repository() {
    let config = Config::local();
    let db = Database::new(&config).await.unwrap();

    let repo_id = db.insert_repository("/test/repo", "test").await.unwrap();
    assert!(repo_id > 0);
}
```

### Testing Parser

```rust
#[test]
fn test_parse_python_function() {
    let parser = Parser::new();
    let symbols = parser.parse_file("test.py", "def foo(): pass").unwrap();
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "foo");
    assert_eq!(symbols[0].kind, "function");
}
```

### Testing CLI Commands

```rust
#[tokio::test]
async fn test_index_command() {
    let output = Command::new("cudgel")
        .args(["index", "."])
        .output()
        .await
        .unwrap();

    assert!(output.status.success());
}
```

## Continuous Integration

Tests run automatically on:
- Pull requests
- Commits to main branch

See `.github/workflows/` for CI configuration (if exists).

## Test Data

Test fixtures in `tests/fixtures/`:
- `sample.py`: Python test file
- `sample.rs`: Rust test file
- `sample.js`: JavaScript test file

## Mocking

For external dependencies:
- Use `mockall` crate for trait mocking
- Use test doubles for database in unit tests
- Integration tests use real database

## Performance Tests

```bash
# Benchmark with criterion
cargo bench
```

Benchmarks in `benches/` directory (if exists).

## Debugging Tests

```bash
# Run with debug logging
RUST_LOG=debug cargo test -- --nocapture

# Run specific test with backtrace
RUST_BACKTRACE=1 cargo test test_name -- --nocapture
```
