# PRD: Reliability & Code Quality

## Overview
Ensure Cudgel is production-ready with excellent code quality, idiomatic Rust, and superior user experience.

## Goals
1. Make all code idiomatic and follow Rust best practices
2. Improve UX with better feedback and error handling
3. Ensure reliability for production use cases
4. Achieve comprehensive test coverage

## Non-Goals
- Adding new features (focus is on quality of existing features)
- Performance optimization (unless it affects reliability)

## Success Metrics
- Test coverage: >80%
- Clippy warnings: 0
- User-reported bugs: <1 per month
- Average time to index 1000 files: <10 seconds

## Detailed Requirements

### 1. Code Quality Audit

**Requirements:**
- All public APIs have documentation with examples
- Code follows Rust API Guidelines (https://rust-lang.github.io/api-guidelines/)
- No `unwrap()` or `expect()` in production code paths
- Consistent error handling using `Result<T, Error>`
- All TODOs are tracked as GitHub issues

**Acceptance Criteria:**
- [ ] `cargo doc` generates complete API documentation with no warnings
- [ ] `cargo clippy` passes with no warnings
- [ ] Code review checklist created and followed
- [ ] All public functions have doc examples that compile

### 2. Error Messages

**Requirements:**
- All errors include context about what failed
- Errors suggest next steps to fix the issue
- Database errors explain possible causes
- File system errors show the problematic path

**Examples:**
```rust
// Bad
Error: Connection failed

// Good
Error: Failed to connect to PostgreSQL on localhost:54321
Cause: Connection refused

This usually means PostgreSQL is not running.
Try: task db-start
```

**Acceptance Criteria:**
- [ ] User test: 5 users can resolve common errors without documentation
- [ ] All error types include helpful context
- [ ] Error messages tested with real users

### 3. UX Improvements

**Requirements:**
- Progress indicators for operations >1 second
- Colorful, well-formatted output
- Dry-run mode for all destructive operations
- Confirmation prompts for data deletion
- Smart defaults that "just work"

**Acceptance Criteria:**
- [ ] `cudgel index` shows progress bar with file count
- [ ] `cudgel db-clean` requires confirmation (unless `--force`)
- [ ] All commands support `--dry-run` flag
- [ ] Output tested on light and dark terminal themes
- [ ] Help text is clear and includes examples

### 4. Reliability

**Requirements:**
- Retry logic for transient database errors (3 attempts with exponential backoff)
- Graceful handling of repositories with >100k files
- Connection pool size tuned for typical workloads
- No panics in normal operation
- Proper cleanup on interrupt (Ctrl+C)

**Acceptance Criteria:**
- [ ] Successfully index a repository with 100k+ files
- [ ] Database connection failures retry automatically
- [ ] Ctrl+C during indexing cleans up partial data
- [ ] Memory usage stays below 1GB for 50k files
- [ ] No panics in production (logged and converted to errors)

### 5. Testing

**Requirements:**
- Unit tests for all core logic
- Integration tests for CLI commands
- Property-based tests for parser
- Performance benchmarks tracked over time
- Test fixtures for all supported languages

**Test Categories:**
- Unit: Parser, database operations, query engine
- Integration: Full indexing workflow, search accuracy
- Property-based: Parser always produces valid AST
- Performance: Index speed, query speed, memory usage

**Acceptance Criteria:**
- [ ] Test coverage >80% (measured by tarpaulin)
- [ ] All PRs include tests
- [ ] CI runs full test suite
- [ ] Benchmarks tracked in separate workflow
- [ ] Performance regression alerts configured

## Implementation Plan

### Phase 1: Code Audit (Week 1)
1. Run `cargo clippy` and fix all warnings
2. Add missing documentation
3. Replace `unwrap()`/`expect()` with proper error handling
4. Create code review checklist

### Phase 2: Error Handling (Week 2)
1. Audit all error types
2. Add context to each error variant
3. Implement helpful suggestions
4. Test with real users

### Phase 3: UX Polish (Week 3)
1. Add progress indicators
2. Implement dry-run mode
3. Improve help text
4. Add confirmation prompts

### Phase 4: Reliability (Week 4)
1. Add retry logic
2. Test with large repositories
3. Tune connection pool
4. Handle interrupts gracefully

### Phase 5: Testing (Week 5-6)
1. Write unit tests (target 80% coverage)
2. Add integration tests for all commands
3. Set up property-based testing
4. Create performance benchmarks

## Dependencies
- None (all internal improvements)

## Risks & Mitigation

**Risk**: Breaking changes while refactoring
**Mitigation**: Comprehensive test suite before changes, incremental refactors

**Risk**: Time estimates too optimistic
**Mitigation**: Break into smaller PRs, can extend timeline if needed

## Open Questions
- Should we add telemetry to track real-world usage patterns?
- What threshold of performance regression is acceptable?

## References
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Error Handling in Rust](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
