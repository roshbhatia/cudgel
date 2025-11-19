# Progress Indicator Research for Cudgel CLI

## Executive Summary

**Recommendation**: Use `indicatif` v0.18.3 with tokio integration for all progress indicators in the cudgel deps command.

**Rationale**:
- Most mature and actively maintained (5k+ stars, MIT license)
- Native tokio async support via optional feature flag
- Excellent terminal compatibility with automatic TTY detection
- Comprehensive formatting options for bytes, duration, and throughput
- Built-in graceful fallback for CI/non-TTY environments

---

## 1. Popular Crates Comparison

### indicatif (Recommended)
- **Version**: 0.18.3 (latest stable)
- **License**: MIT
- **Stars**: 5,000+
- **Maintenance**: Active (console-rs organization)
- **Features**:
  - Progress bars with percentage, bytes, ETA
  - Spinners for indeterminate operations
  - Multi-progress bar support
  - Automatic TTY detection
  - Rich templating system
  - Human-readable formatters (bytes, duration, count)

### pbr
- **Version**: 1.1.1
- **License**: MIT
- **Last Update**: Less active than indicatif
- **Features**: Basic progress bars
- **Assessment**: Less feature-rich, no native async support

### Others Considered
- **progressing**: Minimal features, not suitable for async
- **console**: Terminal abstraction library (used by indicatif internally)

---

## 2. Integration with Tokio Async Runtime

### Setup
Add to `Cargo.toml`:
```toml
[dependencies]
indicatif = { version = "0.18", features = ["tokio"] }
tokio = { version = "1", features = ["rt", "time", "macros"] }
```

### Tokio Async Pattern
```rust
use indicatif::ProgressBar;
use tokio::time::{interval, Duration};

async fn download_with_progress(total_bytes: u64) -> Result<()> {
    let pb = ProgressBar::new(total_bytes);
    pb.set_style(ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
    )?
    .progress_chars("#>-"));

    // Download chunks asynchronously
    let mut downloaded = 0;
    while downloaded < total_bytes {
        // Async download operation
        let chunk = download_chunk().await?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("✓ Download complete");
    Ok(())
}
```

### Key Points
- `ProgressBar` is `Send + Sync` - safe to share across async tasks
- Use `pb.inc(n)` or `pb.set_position(n)` from async contexts
- No need for explicit locking - indicatif handles thread safety internally
- Compatible with `tokio::spawn` and multi-threaded runtimes

---

## 3. Progress Bar vs Spinner vs Percentage Updates

### Progress Bar (Determinate Operations)
**Use for**: Model downloads, file operations with known size

```rust
use indicatif::{ProgressBar, ProgressStyle};

fn download_progress(total_bytes: u64) -> ProgressBar {
    let pb = ProgressBar::new(total_bytes);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
        )
        .unwrap()
        .progress_chars("#>-")
    );
    pb
}
```

**Output**:
```
⠋ [00:01:23] [##########>---------] 52.3 MB/100 MB (628 KB/s, 1.2m)
```

### Spinner (Indeterminate Operations)
**Use for**: Database initialization, schema creation, unknown duration tasks

```rust
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

fn spinner_for_task(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}
```

**Output**:
```
⠹ Initializing database...
```

### Percentage Updates
Built into progress bar templates using `{percent}` or `{percent_precise}`:

```rust
ProgressStyle::with_template(
    "[{elapsed_precise}] {bar:40.cyan/blue} {percent}% {msg}"
)
```

---

## 4. Terminal Compatibility Considerations

### Automatic TTY Detection
indicatif automatically detects non-TTY environments (CI, pipes, redirects) and disables progress rendering:

```rust
use indicatif::ProgressBar;

// Automatically handles TTY detection
let pb = ProgressBar::new(1000);
// If stdout is not a TTY, progress bar is hidden
// Regular println! messages still work
```

### Explicit TTY Control
```rust
use indicatif::{ProgressBar, ProgressDrawTarget};

let pb = ProgressBar::new(1000);

// Force hidden (for CI environments)
if std::env::var("CI").is_ok() {
    pb.set_draw_target(ProgressDrawTarget::hidden());
}

// Force visible (for testing)
pb.set_draw_target(ProgressDrawTarget::stderr());
```

### CI/Non-TTY Best Practices
1. **Fallback Messages**: Always call `pb.finish_with_message()` to provide final status
2. **Logging Integration**: Use `indicatif-log-bridge` to coordinate with log crate
3. **Environment Detection**: Check `CI` env var for explicit non-interactive mode
4. **Graceful Degradation**: Progress bars automatically become no-ops in non-TTY

```rust
fn create_progress_bar(len: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    
    // In CI, log start message
    if is_ci_environment() {
        eprintln!("{}", message);
    } else {
        pb.set_style(/* rich style */);
    }
    
    pb
}

fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok() || 
    !console::Term::stderr().is_term()
}
```

---

## 5. User Experience Best Practices

### Downloads (100MB Model)
```rust
use indicatif::{ProgressBar, ProgressStyle};

async fn download_model(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template(
            "Downloading model\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
        )?
        .progress_chars("#>-")
    );

    let mut downloaded = Vec::new();
    let mut stream = response.bytes_stream();
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        downloaded.extend_from_slice(&chunk);
        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("✓ Model downloaded successfully");
    Ok(downloaded)
}
```

**UX Considerations**:
- Show spinner while waiting for server response
- Switch to progress bar once content-length is known
- Display human-readable bytes (52.3 MB vs 54894621 bytes)
- Show transfer speed and ETA
- Final success message visible after completion

### Database Operations (1-10 seconds)
```rust
async fn initialize_database(db: &Database) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap()
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    
    pb.set_message("Connecting to database...");
    db.connect().await?;
    
    pb.set_message("Initializing schema...");
    db.initialize_schema().await?;
    
    pb.set_message("Creating indexes...");
    db.create_indexes().await?;
    
    pb.finish_with_message("✓ Database initialized");
    Ok(())
}
```

**UX Considerations**:
- Use spinner for unknown duration
- Update message to show current step
- Keep user informed of progress
- Clear final status message

### Schema Creation (1-2 seconds)
```rust
async fn create_schema() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message("Creating database schema...");
    
    // Execute schema creation
    execute_schema_sql().await?;
    
    pb.finish_and_clear(); // Fast operation, don't need final message
    Ok(())
}
```

---

## 6. Recommended Implementation for Cudgel

### Cargo.toml Dependencies
```toml
[dependencies]
indicatif = { version = "0.18", features = ["tokio"] }
tokio = { version = "1", features = ["rt-multi-thread", "time", "macros"] }
reqwest = { version = "0.12", features = ["stream"] }
console = "0.16" # For terminal detection
```

### Progress Module Structure
```rust
// src/progress.rs

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub struct ProgressReporter {
    multi: Option<indicatif::MultiProgress>,
}

impl ProgressReporter {
    pub fn new() -> Self {
        let multi = if console::Term::stderr().is_term() {
            Some(indicatif::MultiProgress::new())
        } else {
            None
        };
        Self { multi }
    }

    pub fn download_bar(&self, total_bytes: u64) -> ProgressBar {
        let pb = match &self.multi {
            Some(m) => m.add(ProgressBar::new(total_bytes)),
            None => ProgressBar::hidden(),
        };

        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
            )
            .unwrap()
            .progress_chars("#>-")
        );

        pb
    }

    pub fn spinner(&self, message: &str) -> ProgressBar {
        let pb = match &self.multi {
            Some(m) => m.add(ProgressBar::new_spinner()),
            None => ProgressBar::hidden(),
        };

        pb.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap()
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(100));

        pb
    }
}
```

### Usage in deps Command
```rust
// src/orchestrator.rs

use crate::progress::ProgressReporter;

pub async fn setup_dependencies() -> Result<()> {
    let reporter = ProgressReporter::new();

    // 1. Download model (~100MB, 5+ minutes)
    let download_pb = reporter.download_bar(100_000_000);
    download_model(&download_pb).await?;
    download_pb.finish_with_message("✓ Model downloaded");

    // 2. Initialize database (1-10 seconds)
    let db_pb = reporter.spinner("Initializing database...");
    initialize_database().await?;
    db_pb.finish_with_message("✓ Database initialized");

    // 3. Create schema (1-2 seconds)
    let schema_pb = reporter.spinner("Creating schema...");
    create_schema().await?;
    schema_pb.finish_with_message("✓ Schema created");

    Ok(())
}
```

---

## 7. Additional Resources

### Official Documentation
- [indicatif docs.rs](https://docs.rs/indicatif/latest/indicatif/)
- [GitHub repository](https://github.com/console-rs/indicatif)
- [Examples directory](https://github.com/console-rs/indicatif/tree/main/examples)

### Related Crates
- `indicatif-log-bridge`: Integration with log crate
- `tracing-indicatif`: Integration with tracing crate
- `console`: Terminal detection and styling (used internally)

### Community Examples
- [cargo](https://github.com/rust-lang/cargo): Uses indicatif for downloads
- [rustup](https://github.com/rust-lang/rustup): Uses indicatif for installations
- [tokio-console](https://github.com/tokio-rs/console): Progress monitoring for async

---

## 8. Testing Considerations

### Unit Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use indicatif::ProgressDrawTarget;

    #[tokio::test]
    async fn test_download_progress() {
        let pb = ProgressBar::new(1000);
        pb.set_draw_target(ProgressDrawTarget::hidden());
        
        // Test progress updates without visual output
        pb.inc(500);
        assert_eq!(pb.position(), 500);
    }
}
```

### Integration Testing in CI
```bash
# Set CI environment variable
export CI=true

# Progress bars automatically hidden, only final messages printed
cargo run -- deps

# Output in CI:
# Downloading model...
# ✓ Model downloaded
# Initializing database...
# ✓ Database initialized
# ✓ Schema created
```

---

## Conclusion

**indicatif** is the clear choice for cudgel's progress indication needs:

1. ✅ Native tokio async support
2. ✅ Automatic TTY detection and graceful fallback
3. ✅ Rich formatting for downloads (bytes, speed, ETA)
4. ✅ Spinner support for indeterminate operations
5. ✅ Production-ready (used by cargo, rustup, and other major Rust tools)
6. ✅ Excellent documentation and active maintenance

**Next Steps**:
1. Add indicatif dependency with tokio feature
2. Implement `ProgressReporter` module
3. Integrate into `deps` command for:
   - Model download progress bar
   - Database initialization spinner
   - Schema creation spinner
4. Test in both TTY and CI environments
