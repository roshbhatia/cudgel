# Progress Indicators - Quick Reference

## TL;DR
Use **indicatif 0.18** with tokio feature. Already have 0.17, just need to upgrade and add tokio feature.

## Three Progress Types

### 1. Download Bar (for known-size downloads)
```rust
let pb = reporter.download_progress(total_bytes, "📦 Downloading model");
while let Some(chunk) = stream.next().await {
    pb.inc(chunk.len() as u64);
}
pb.finish_with_message("✓ Downloaded");
```
**Output**: `⠋ [00:01:23] [##########>---------] 52.3 MB/100 MB (628 KB/s, 1.2m)`

### 2. Spinner (for unknown duration)
```rust
let pb = reporter.spinner("🔧 Initializing database");
do_work().await?;
pb.finish_with_message("✓ Initialized");
```
**Output**: `⠹ Initializing database...`

### 3. Progress Bar (for known item count)
```rust
let pb = reporter.progress_bar(total_files, "📂 Indexing files");
for file in files {
    process(file)?;
    pb.inc(1);
}
pb.finish();
```
**Output**: `[00:00:42] [##########----------] 500/1000 Processing...`

## CI/Non-TTY Behavior
Automatically detected. No visual progress, just messages:
```
📦 Downloading model
✓ Downloaded
🔧 Initializing database
✓ Initialized
```

## Dependencies to Add
```toml
indicatif = { version = "0.18", features = ["tokio"] }
```

## Template Placeholders
Common placeholders for custom templates:

| Placeholder | Description | Example |
|------------|-------------|---------|
| `{bytes}` | Current bytes (binary) | `52.3 MB` |
| `{total_bytes}` | Total bytes (binary) | `100 MB` |
| `{bytes_per_sec}` | Transfer speed | `628 KB/s` |
| `{eta}` | Estimated time remaining | `1.2m` |
| `{elapsed_precise}` | Elapsed time HH:MM:SS | `00:01:23` |
| `{bar:N}` | Progress bar N chars wide | `[##########>---------]` |
| `{wide_bar}` | Auto-sized progress bar | (fills terminal) |
| `{spinner}` | Animated spinner | `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` |
| `{msg}` | Custom message | `Processing file.rs` |
| `{pos}` | Current position | `500` |
| `{len}` | Total length | `1000` |
| `{percent}` | Percentage complete | `50%` |

## Common Patterns

### Update message during operation
```rust
let pb = reporter.spinner("Step 1");
pb.set_message("Step 2");
pb.set_message("Step 3");
pb.finish_with_message("✓ Done");
```

### Silent progress (for tests)
```rust
use indicatif::ProgressDrawTarget;
pb.set_draw_target(ProgressDrawTarget::hidden());
```

### Multi-step operation
```rust
let reporter = Arc::new(ProgressReporter::new());

// Step 1
let pb1 = reporter.download_progress(100_000_000, "Download");
download(&pb1).await?;
pb1.finish_with_message("✓ Downloaded");

// Step 2
let pb2 = reporter.spinner("Initialize");
init().await?;
pb2.finish_with_message("✓ Initialized");
```

## Error Handling
Always finish progress bars, even on error:
```rust
let pb = reporter.spinner("Processing");
match do_work().await {
    Ok(_) => pb.finish_with_message("✓ Success"),
    Err(e) => {
        pb.finish_with_message("✗ Failed");
        return Err(e);
    }
}
```

Or use `finish_and_clear()` to remove the bar on failure:
```rust
if let Err(e) = do_work().await {
    pb.finish_and_clear();
    return Err(e);
}
```

## When NOT to Use Progress
- Operations < 1 second
- Non-interactive scripts (automatic fallback handles this)
- Already have detailed tracing logs for debugging
- Operations with unpredictable or highly variable duration

## Testing
```rust
#[tokio::test]
async fn test_with_progress() {
    std::env::set_var("CI", "true"); // Force non-interactive
    let reporter = ProgressReporter::new();
    let pb = reporter.spinner("Test");
    pb.finish(); // Won't panic
}
```

## Full Examples
See:
- `/docs/progress-indicators-research.md` - Detailed research
- `/docs/progress-indicators-example.rs` - Full implementation examples
- `/docs/progress-indicators-summary.md` - Integration guide
