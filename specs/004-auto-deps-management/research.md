# Research: Automatic Dependency Management

**Feature**: 004-auto-deps-management  
**Phase**: 0 (Technical Research)  
**Date**: 2025-11-19

## Overview

This document consolidates research findings for implementing the `cudgel deps` command. All NEEDS CLARIFICATION items from Technical Context have been resolved with specific recommendations and rationale.

---

## Research Task 1: Model Download Methods

### Decision: **Use `hf-hub` Rust Crate** ✅

**Rationale**:
- Official HuggingFace client for Rust (2.4M+ downloads, actively maintained)
- Zero external prerequisites (no Python/uv required)
- Native async/await integration with tokio (already in cudgel stack)
- Built-in resumable downloads via HTTP Range requests
- Automatic retry with exponential backoff
- ETag-based integrity verification included
- Cross-platform (Windows, macOS, Linux) out of box
- Works offline after first download
- Minimal binary size increase (~200KB)

**Alternatives Considered**:

1. **Shell out to Python `optimum-cli`** ❌
   - Requires Python 3.8+ and uv as prerequisites
   - ~500MB of Python dependencies to install
   - Subprocess management complexity
   - Poor progress reporting (CLI output parsing)
   - 2-3 second Python import overhead
   - Cross-platform path escaping issues

2. **Hybrid (Direct HTTP + Python fallback)** ❌
   - Reimplements what `hf-hub` already provides
   - Manual retry, resume, cache, locking logic
   - More code to maintain and test
   - HuggingFace Hub API evolution requires manual tracking

**Implementation Pattern**:

```rust
// Cargo.toml
hf-hub = { version = "0.4", features = ["tokio"] }

// Download with automatic retry/resume
use hf_hub::api::tokio::Api;
use hf_hub::Repo;

pub async fn download_embedding_model(target_dir: &Path) -> Result<()> {
    let api = Api::new()?.cache_dir(target_dir);
    let repo = Repo::model("sentence-transformers/all-MiniLM-L6-v2".to_string());
    let model = api.repo(repo);
    
    // Download required files (atomic, resumable)
    let model_path = model.get("onnx/model.onnx").await?;
    let tokenizer_path = model.get("tokenizer.json").await?;
    let config_path = model.get("config.json").await?;
    
    // Copy from cache to target directory
    std::fs::copy(model_path, target_dir.join("model.onnx"))?;
    std::fs::copy(tokenizer_path, target_dir.join("tokenizer.json"))?;
    std::fs::copy(config_path, target_dir.join("config.json"))?;
    
    Ok(())
}
```

**Error Handling**: Three-layer approach
1. Network errors with actionable messages (check connection, firewall, disk space)
2. Pre-flight disk space check (need ~500MB free)
3. Automatic cleanup of partial downloads on failure

**Integrity Verification**: Three-layer strategy
1. HTTP ETag validation (automatic via `hf-hub`)
2. File size sanity check (expected ranges for each file)
3. Functional validation (attempt to load ONNX model and tokenizer)

**Recovery Strategy**: Automatic via `hf-hub`
- Downloads to `.tmp` file first, then atomic rename
- File locking prevents concurrent download corruption
- Resume support via HTTP Range requests
- Failed downloads cleaned up transparently
- User just re-runs `cudgel deps` - no manual intervention needed

---

## Research Task 2: XDG Directory Patterns

### Decision: **Keep Existing Manual Implementation** ✅

**Rationale**:
- Current implementation in `src/config.rs:276-320` is 100% XDG-compliant (spec 0.8)
- Thread-safe (std::env::var is thread-safe)
- Cross-platform (Linux/macOS working, Windows straightforward to add)
- Zero dependencies (45 lines vs. 1-3 new crates)
- Correct fallbacks already implemented (XDG_DATA_HOME → ~/.local/share, etc.)
- Full control over behavior

**Alternatives Considered**:

1. **`dirs` crate** ❌
   - Adds 2 dependencies
   - Partial XDG compliance (uses ~/Library on macOS for GUI apps)
   - Overkill for CLI tool needs

2. **`xdg` crate** ❌
   - Linux-only
   - Advanced features (XDG_DATA_DIRS search) not needed

3. **`etcetera` crate** ❌
   - Adds 3 dependencies
   - Runtime strategy switching (XDG vs Apple vs Windows) not needed
   - Excessive complexity

**Comparison Matrix**:

| Feature | Current | `dirs` | `xdg` | `etcetera` |
|---------|---------|--------|-------|-----------|
| XDG Compliance | ✅ Full | ⚠️ Partial | ✅ Full | ✅ Full |
| Thread Safety | ✅ | ✅ | ✅ | ✅ |
| Cross-Platform | Linux/macOS | All | Linux only | All |
| Dependencies | 0 | +2 | +1 | +3 |
| Complexity | Simple | Simple | Advanced | Complex |

**Current Implementation** (no changes needed):

```rust
// src/config.rs (existing, XDG-compliant)
fn xdg_data_home() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|home| PathBuf::from(home).join(".local/share"))
                .expect("HOME environment variable must be set")
        })
}

fn xdg_state_home() -> PathBuf {
    std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|home| PathBuf::from(home).join(".local/state"))
                .expect("HOME environment variable must be set")
        })
}

// Similar for xdg_cache_home(), xdg_config_home()
```

**Optional Enhancement**: Add XDG variable override tests

```rust
#[test]
fn test_xdg_variable_override() {
    std::env::set_var("XDG_DATA_HOME", "/custom/data");
    assert_eq!(xdg_data_home(), PathBuf::from("/custom/data"));
    std::env::remove_var("XDG_DATA_HOME");
}
```

---

## Research Task 3: PostgreSQL Process Management

### Decision: **Shell Out to Existing Scripts** ✅

**Rationale**:
- Reuses existing battle-tested infrastructure (scripts/start-postgres.sh, scripts/stop-postgres.sh)
- Scripts already handle initialization, database creation, pgvector extension
- Minimal code (~50 lines of Rust vs. 200+ for reimplementation)
- Easy to maintain (scripts can be tested independently)
- Reliable (all edge cases already handled)

**Detection Method**: Use `pg_isready` (10-50ms, official PostgreSQL tool)

**Comparison**:

| Method | Speed | Reliability | Verdict |
|--------|-------|-------------|---------|
| **pg_isready** | 10-50ms | ✅ High (official tool) | ✅ **BEST** |
| TCP port check | <10ms | ❌ Low (false positives) | ❌ |
| Process parsing | 100-500ms | ❌ Medium (fragile) | ❌ |
| Full connection | 100-500ms | ✅ High (but slow) | ❌ |

**Proposed Implementation**:

```rust
// src/deps/database.rs
pub struct PostgresManager {
    scripts_dir: PathBuf,
    port: u16,
}

impl PostgresManager {
    // Check if running (fast, no connection)
    pub fn is_running(&self) -> Result<bool> {
        Ok(Command::new("pg_isready")
            .arg("-p").arg(self.port.to_string())
            .arg("-h").arg("localhost")
            .status()?
            .success())
    }
    
    // Start (idempotent)
    pub fn start(&self) -> Result<()> {
        if self.is_running()? { 
            return Ok(()); 
        }
        
        let status = Command::new("bash")
            .arg(self.scripts_dir.join("start-postgres.sh"))
            .status()?;
            
        if !status.success() {
            return Err(Error::DatabaseStart(
                "Failed to start PostgreSQL. Check logs at ~/.local/share/cudgel/postgres.log"
            ));
        }
        
        Ok(())
    }
    
    // Stop (graceful, fast mode)
    pub fn stop(&self) -> Result<()> {
        if !self.is_running()? {
            return Ok(());
        }
        
        let status = Command::new("bash")
            .arg(self.scripts_dir.join("stop-postgres.sh"))
            .status()?;
            
        if !status.success() {
            return Err(Error::DatabaseStop("Failed to stop PostgreSQL"));
        }
        
        Ok(())
    }
}
```

**Key Design Decisions**:
- Idempotent operations (safe to call multiple times)
- Graceful shutdown (uses `pg_ctl stop -m fast` - clean, no data loss)
- Cross-platform (works on macOS/Linux via PATH)
- XDG compliant (respects `$XDG_DATA_HOME` for data directory)
- Port configuration (reads `$CUDGEL_POSTGRES_PORT`, default: 45678)

**Error Scenarios Handled**:

| Scenario | Detection | User Message |
|----------|-----------|--------------|
| PostgreSQL not installed | `which pg_ctl` | Platform-specific install instructions |
| Port already in use | Script exit code | Check with `lsof -i :45678` |
| Permission errors | I/O errors | Fix directory ownership |
| Startup timeout | Wait loop (30s) | Show tail of log file |

**Cross-Platform Considerations**:
- macOS: Homebrew PostgreSQL (or Nix)
- Linux: apt/yum PostgreSQL packages (or Nix)
- Socket path: Configured in scripts (`/tmp` for macOS compat)
- User: Uses current user (not `postgres` system user)

---

## Research Task 4: Progress Indicators

### Decision: **Use `indicatif` 0.18+ with Tokio Feature** ✅

**Rationale**:
- Already using indicatif v0.17 - simple upgrade path
- Native tokio async support (critical for model downloads)
- Automatic TTY detection (graceful CI fallback)
- Rich formatting (bytes, speed, ETA, spinners)
- Production-proven (used by cargo, rustup, bat)
- Active maintenance by console-rs organization

**Alternatives Considered**:

1. **`pbr` crate** ❌
   - No tokio async support
   - Last updated 2019 (unmaintained)

2. **`progressing` crate** ❌
   - Minimal features (no bytes formatting)
   - No tokio integration

3. **Simple stdout percentage** ❌
   - Poor UX (no speed/ETA)
   - Clutters output

**Comparison**:

| Feature | indicatif | pbr | progressing | stdout |
|---------|-----------|-----|-------------|--------|
| Tokio async | ✅ Native | ❌ | ❌ | N/A |
| TTY detection | ✅ Auto | ⚠️ Manual | ❌ | ❌ |
| Bytes formatting | ✅ | ✅ | ❌ | ❌ |
| Speed/ETA | ✅ | ✅ | ❌ | ❌ |
| Spinners | ✅ | ❌ | ✅ | ❌ |
| Multi-progress | ✅ | ❌ | ❌ | ❌ |
| Maintenance | ✅ Active | ❌ 2019 | ⚠️ Slow | N/A |

**Implementation Pattern**:

```rust
// Cargo.toml (upgrade from 0.17)
indicatif = { version = "0.18", features = ["tokio"] }

// Download progress (determinate)
use indicatif::{ProgressBar, ProgressStyle};

let pb = ProgressBar::new(100_000_000); // 100MB
pb.set_style(
    ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
        .progress_chars("=>-")
);
pb.set_message("📦 Downloading model.onnx");

// In download loop
pb.inc(chunk.len() as u64);

pb.finish_with_message("✓ Model downloaded");

// Database initialization (indeterminate)
let spinner = ProgressBar::new_spinner();
spinner.set_style(
    ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")?
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
);
spinner.set_message("Initializing database...");
spinner.enable_steady_tick(Duration::from_millis(100));

// ... operation ...

spinner.finish_with_message("✓ Database initialized");
```

**UX Requirements Met**:

| Requirement | Solution | Implementation |
|-------------|----------|----------------|
| Tokio async | ✅ Native support | Add `features = ["tokio"]` |
| Indeterminate ops | ✅ Spinners | `ProgressBar::new_spinner()` |
| Download progress | ✅ Bytes/speed/ETA | `{bytes}/{total_bytes} ({bytes_per_sec}, {eta})` |
| CI/Non-TTY | ✅ Auto fallback | Built-in TTY detection (silent in CI) |
| 100MB downloads | ✅ Optimized | Chunk-based updates with `pb.inc()` |
| Database ops | ✅ Spinners | Update message with `pb.set_message()` |

**Expected UX**:

```
Checking dependencies...
  ✓ PostgreSQL running
  ✗ ONNX model not found

Downloading embedding model...
[00:01:23] =========>----------------------- 45.2 MB/100 MB (3.2 MB/s, 17s)

⠹ Initializing database schema...
  ✓ Database schema initialized
  ✓ All dependencies satisfied
```

**Terminal Compatibility**:
- TTY: Rich progress bars with colors and animations
- Non-TTY (CI): Silent or simple line-by-line output
- Automatic detection via `indicatif::ProgressDrawTarget::stderr_with_hz()`

---

## Research Task 5: Checksum Verification

### Decision: **Three-Layer Verification Strategy** ✅

**Rationale**:
- Balances reliability with performance
- Catches corruption without excessive overhead
- Leverages `hf-hub` built-in verification

**Layer 1: HTTP ETag Validation** (Automatic)
- Provided by `hf-hub` automatically
- ETags in HuggingFace Hub are Git blob SHAs
- No code needed - handled transparently

**Layer 2: File Size Sanity Check** (Fast)
```rust
fn verify_file_sizes(model_dir: &Path) -> Result<()> {
    let files = [
        ("model.onnx", 90_000_000..110_000_000),      // ~100MB
        ("tokenizer.json", 2_000_000..8_000_000),     // ~5MB
        ("config.json", 500..2_000),                  // ~1KB
    ];
    
    for (filename, expected_range) in files {
        let path = model_dir.join(filename);
        let size = std::fs::metadata(&path)?.len();
        
        if !expected_range.contains(&size) {
            return Err(Error::CorruptedDownload {
                file: filename.to_string(),
                actual_size: size,
                expected_range: format!("{:?}", expected_range),
            });
        }
    }
    
    Ok(())
}
```

**Layer 3: Functional Validation** (Most Reliable)
```rust
fn verify_model_loadable(model_dir: &Path) -> Result<()> {
    // Test ONNX model loads
    let model_path = model_dir.join("model.onnx");
    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .commit_from_file(&model_path)
        .map_err(|e| Error::CorruptedModel(format!("ONNX load failed: {}", e)))?;
    
    // Verify input/output shapes match expected
    let inputs = session.inputs;
    assert_eq!(inputs.len(), 3, "Expected 3 inputs (input_ids, attention_mask, token_type_ids)");
    
    // Test tokenizer loads
    let tokenizer_path = model_dir.join("tokenizer.json");
    Tokenizer::from_file(&tokenizer_path)
        .map_err(|e| Error::CorruptedModel(format!("Tokenizer load failed: {}", e)))?;
    
    Ok(())
}
```

**Optional Layer 4: SHA-256 Checksum** (If Paranoid)
- Adds ~2-3 seconds for 100MB file
- Not recommended for initial implementation
- Can be added later if users report corruption issues

**Performance Impact**:
- Layer 1: 0ms (automatic during download)
- Layer 2: <10ms (file stat calls)
- Layer 3: 100-200ms (ONNX session creation)
- Total: ~200ms overhead (acceptable)

---

## Summary of Decisions

| Research Area | Decision | Implementation Effort | Risk |
|---------------|----------|---------------------|------|
| Model Download | Use `hf-hub` crate | 2-3 hours | Low |
| XDG Directories | Keep existing code | 0 hours | None |
| PostgreSQL Mgmt | Shell out to scripts | 1-2 hours | Low |
| Progress Indicators | Upgrade indicatif to 0.18 | 1-2 hours | Low |
| Checksum Verification | Three-layer strategy | 1 hour | Low |

**Total Implementation Estimate**: 5-8 hours for complete `cudgel deps` command

**Dependencies to Add**:
```toml
[dependencies]
hf-hub = { version = "0.4", features = ["tokio"] }
indicatif = { version = "0.18", features = ["tokio"] }  # upgrade from 0.17
```

**No Dependencies to Remove**: All decisions leverage existing patterns and minimize external dependencies.

---

## Next Steps

1. ✅ Research complete - all NEEDS CLARIFICATION resolved
2. ⏳ Phase 1: Generate data-model.md (Dependency, ModelArtifact, DatabaseInstance entities)
3. ⏳ Phase 1: Generate contracts/cli-interface.md (CLI contract for cudgel deps)
4. ⏳ Phase 1: Generate quickstart.md (usage examples)
5. ⏳ Update agent context (AGENTS.md)
6. ⏳ Re-verify Constitution Check
7. ⏳ User runs `/speckit.tasks` for implementation tasks
