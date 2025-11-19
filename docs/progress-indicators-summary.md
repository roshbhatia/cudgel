# Progress Indicators - Implementation Summary

## Current State

Cudgel already has `indicatif = "0.17"` in `Cargo.toml` (line 18) and uses:
- **Logging**: `tracing` crate with structured logging
- **Output**: Mix of `println!` and `eprintln!` for user feedback
- **No progress indicators**: Currently no visual progress for long operations

## Recommended Upgrade & Implementation

### 1. Update Dependencies

```toml
# In Cargo.toml, update line 18:
indicatif = { version = "0.18", features = ["tokio"] }

# Already have these (no changes needed):
tokio = { version = "1.40", features = ["full"] }
tracing = "0.1"
```

**Why upgrade to 0.18?**
- Better tokio async integration
- Improved TTY detection
- Bug fixes and performance improvements
- Breaking changes are minimal (mostly internal)

### 2. Create Progress Module

Add new file: `src/progress.rs`

Key features:
- Automatic TTY detection (graceful CI fallback)
- Coordinated multi-progress management
- Three progress types:
  1. **Download bar**: bytes, speed, ETA for model downloads
  2. **Spinner**: indeterminate operations (DB init)
  3. **Progress bar**: known-length operations

### 3. Integration Points

#### A. Model Download (orchestrator.rs or new deps command)
```rust
use crate::progress::ProgressReporter;

async fn download_embedding_model() -> Result<()> {
    let reporter = Arc::new(ProgressReporter::new());
    let pb = reporter.download_progress(100_000_000, "📦 Downloading embedding model");
    
    // Stream download with progress
    while let Some(chunk) = stream.next().await {
        pb.inc(chunk.len() as u64);
    }
    
    pb.finish_with_message("✓ Model downloaded");
    Ok(())
}
```

#### B. Database Initialization (database.rs:179-181)
```rust
// Replace existing println! statements:
let pb = reporter.spinner("🔧 Initializing database schema");
self.initialize_schema().await?;
pb.finish_with_message("✓ Database initialized");
```

#### C. Indexing Progress (indexer.rs:423-437)
```rust
// Replace existing println! statements:
let pb = reporter.progress_bar(total_files, "📂 Indexing repository");
for file in files {
    index_file(file).await?;
    pb.inc(1);
}
pb.finish_with_message("✓ Indexing complete");
```

### 4. Compatibility with Existing Code

**Good news**: indicatif integrates well with existing patterns:

1. **Tracing integration**: 
   - Use `tracing-indicatif` crate for automatic coordination
   - Or simply ensure progress bars call `finish()` before logging

2. **println! replacement**:
   - Keep structured tracing logs for debugging
   - Replace user-facing println! with progress indicators
   - Use `reporter.println()` for messages during progress

3. **CI/Non-TTY**:
   - Automatic detection via `console::Term::stderr().is_term()`
   - Falls back to simple eprintln! messages
   - No visual clutter in logs or pipes

### 5. Usage Patterns by Operation Type

| Operation | Duration | Type | Implementation |
|-----------|----------|------|----------------|
| Model download | 5+ min | Determinate | Download bar with bytes/speed/ETA |
| DB initialization | 1-10s | Indeterminate | Spinner with step messages |
| Schema creation | 1-2s | Indeterminate | Spinner (or skip if too fast) |
| File indexing | Varies | Determinate | Progress bar with file count |
| Query execution | <1s | None | No progress needed |

### 6. Testing Strategy

**Unit Tests**:
```rust
#[tokio::test]
async fn test_progress_with_hidden_output() {
    let pb = ProgressBar::new(100);
    pb.set_draw_target(ProgressDrawTarget::hidden());
    pb.inc(50);
    assert_eq!(pb.position(), 50);
}
```

**Integration Tests**:
```bash
# Set CI mode
export CI=true
cargo test

# Progress bars hidden, only final messages shown:
# ✓ Model downloaded
# ✓ Database initialized
```

### 7. Migration Path

**Phase 1: Add Progress Module** (Low risk)
- Create `src/progress.rs` with ProgressReporter
- Update Cargo.toml to indicatif 0.18 with tokio feature
- Add unit tests

**Phase 2: Integrate in New Features** (Medium risk)
- Use progress indicators in new `deps` command
- Test thoroughly in both TTY and CI environments

**Phase 3: Retrofit Existing Commands** (Optional)
- Replace println! in indexer.rs with progress bars
- Add spinners to database operations
- Maintain backward compatibility with logs

### 8. Quick Win: Deps Command

The most impactful place to add progress indicators first:

```rust
// In src/main.rs or new src/commands/deps.rs
async fn deps_command() -> Result<()> {
    let reporter = Arc::new(ProgressReporter::new());
    
    // 1. Model download (most visible improvement)
    let pb = reporter.download_progress(100_000_000, "📦 Downloading embedding model");
    download_model(&pb).await?;
    pb.finish_with_message("✓ Model downloaded");
    
    // 2. Database setup
    let pb = reporter.spinner("🔧 Setting up database");
    setup_database().await?;
    pb.finish_with_message("✓ Database ready");
    
    Ok(())
}
```

**Expected UX improvement**:

Before:
```
Downloading model...
[5 minutes of silence]
Done
```

After (TTY):
```
📦 Downloading embedding model
⠋ [00:01:23] [##########>---------] 52.3 MB/100 MB (628 KB/s, 1.2m)
✓ Model downloaded
```

After (CI):
```
📦 Downloading embedding model
✓ Model downloaded
```

## Next Steps

1. ✅ **Research complete** (this document)
2. [ ] Update Cargo.toml to indicatif 0.18 with tokio feature
3. [ ] Create src/progress.rs module
4. [ ] Implement deps command with progress indicators
5. [ ] Test in both TTY and CI environments
6. [ ] Consider retrofitting existing commands (optional)

## References

- Full research: `/docs/progress-indicators-research.md`
- Implementation examples: `/docs/progress-indicators-example.rs`
- indicatif docs: https://docs.rs/indicatif/latest/indicatif/
- indicatif GitHub: https://github.com/console-rs/indicatif
