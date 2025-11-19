# Quickstart Guide: cudgel deps

**Feature**: 004-auto-deps-management  
**Phase**: 1 (Design)  
**Date**: 2025-11-19

## Overview

The `cudgel deps` command automates all dependency setup for cudgel, eliminating manual configuration steps. This guide shows common usage patterns.

---

## Installation

First-time setup installs all required dependencies automatically:

```bash
cudgel deps
```

**What it does**:
- Downloads ONNX embedding model from HuggingFace Hub (~100 MB)
- Starts PostgreSQL on port 45678
- Initializes database schema (tables, indexes, extensions)
- Verifies all dependencies are ready

**Expected output**:
```
Checking dependencies...
  ✗ PostgreSQL not running
  ✗ Database schema not initialized
  ✗ ONNX embedding model not found

Starting PostgreSQL...
  ⠹ Initializing database...
  ✓ PostgreSQL started (port 45678)

Initializing database schema...
  ⠹ Creating tables and indexes...
  ✓ Schema initialized (6 tables, 1 index)

Downloading embedding model...
[00:02:15] ======================================= 100 MB/100 MB (740 KB/s, done)

⠹ Verifying model integrity...
  ✓ Model integrity verified

✓ All dependencies satisfied (2m 47s)

You're ready to use cudgel!
Try: cudgel index ./src
```

**Duration**: 2-5 minutes (depending on network speed)

---

## Validation

Check if all dependencies are properly installed:

```bash
cudgel deps --check
```

**What it does**:
- Validates PostgreSQL is running
- Verifies database schema is initialized
- Checks ONNX model files exist and are loadable
- No modifications to system

**Expected output** (all satisfied):
```
Checking dependencies...

Component                  Status    Details
─────────────────────────  ────────  ────────────────────────────────
PostgreSQL                 ✓ OK      Running on port 45678
Database Schema            ✓ OK      Version 1, 6 tables
ONNX Embedding Model       ✓ OK      100.2 MB, loadable

✓ All dependencies satisfied (0.3s)
```

**Expected output** (with issues):
```
Checking dependencies...

Component                  Status    Details
─────────────────────────  ────────  ────────────────────────────────
PostgreSQL                 ✗ MISSING Not running
Database Schema            ⚠ UNKNOWN Cannot verify (DB not running)
ONNX Embedding Model       ✓ OK      100.2 MB, loadable

✗ 1 dependency missing, 1 unknown

To fix:
  Run: cudgel deps
```

**Duration**: <2 seconds

---

## Detailed Diagnostics

Get verbose information about dependency status:

```bash
cudgel deps --check --verbose
```

**What it does**:
- Shows file paths for all components
- Displays version numbers
- Shows environment variables (XDG_*)
- Lists process IDs
- Shows log file locations

**Expected output**:
```
Checking dependencies (verbose mode)...

PostgreSQL
  Status: ✓ RUNNING
  Host: localhost
  Port: 45678
  Data Directory: /Users/alice/.local/share/cudgel/postgres
  PID: 12345
  Version: PostgreSQL 15.3
  Log File: /Users/alice/.local/state/cudgel/postgres.log

Database Schema
  Status: ✓ INITIALIZED
  Database: cudgel
  Tables: repositories, files, symbols, embeddings, scheduled_tasks, call_graph_edges
  Indexes: embeddings_hnsw_idx
  Extensions: vector

ONNX Embedding Model
  Status: ✓ OK
  Model ID: sentence-transformers/all-MiniLM-L6-v2
  Location: /Users/alice/.local/share/cudgel/models/all-MiniLM-L6-v2/
  Files:
    - model.onnx (100,230,144 bytes)
    - tokenizer.json (5,123,456 bytes)
    - config.json (1,234 bytes)
  Verification: ONNX session loads successfully

Environment
  XDG_DATA_HOME: /Users/alice/.local/share
  XDG_STATE_HOME: /Users/alice/.local/state
  XDG_CACHE_HOME: /Users/alice/.cache
  CUDGEL_POSTGRES_PORT: 45678

✓ All dependencies satisfied (0.5s)
```

**Use case**: Troubleshooting setup issues or verifying custom XDG paths

---

## Cleanup

Remove downloaded models to free disk space (keeps database):

```bash
cudgel deps --clean
```

**What it does**:
- Removes ONNX model files from `XDG_DATA_HOME/cudgel/models/`
- Removes temporary cache files
- Shows amount of disk space freed
- **Preserves database and indexed code**

**Expected output**:
```
⚠ This will remove downloaded models (~100 MB)
   Database will be preserved.

Proceed? (y/N): y

Cleaning up dependencies...
  ✓ Removed model files (100.2 MB freed)
  ✓ Removed cache files (2.3 MB freed)

✓ Cleanup complete (102.5 MB freed)

To reinstall: cudgel deps
```

**Use case**: Free disk space when not actively using cudgel

---

## Complete Reset

Remove all cudgel data including database:

```bash
cudgel deps --clean --all
```

**What it does**:
- Stops PostgreSQL gracefully
- Removes model files
- Removes database data directory
- Removes all XDG directories (data, state, cache)
- **WARNING: This deletes all indexed code data**

**Expected output**:
```
⚠ WARNING: This will remove ALL cudgel data including:
   - Downloaded models (~100 MB)
   - PostgreSQL database (~50 MB)
   - Indexed code data
   - Scheduled tasks
   - All logs

   This action cannot be undone.

Type 'DELETE' to proceed: DELETE

Cleaning up all dependencies...
  ⠹ Stopping PostgreSQL...
  ✓ PostgreSQL stopped
  ✓ Removed model files (100.2 MB)
  ✓ Removed database (48.7 MB)
  ✓ Removed state files (1.2 MB)

✓ Complete cleanup done (150.1 MB freed)

To start fresh: cudgel deps
```

**Use case**: Complete uninstall or fresh start

**Safety**: In non-TTY environments (scripts), requires `--force` flag

---

## Common Workflows

### Fresh Installation
```bash
# Clone repository
git clone https://github.com/roshbhatia/cudgel.git
cd cudgel

# Build and install
cargo build --release
cargo install --path .

# Setup dependencies (one command!)
cudgel deps

# Start using cudgel
cudgel index ./src
```

### Verify Setup After Update
```bash
# Pull latest changes
git pull

# Rebuild
cargo build --release

# Verify dependencies still satisfied
cudgel deps --check

# If issues found, reinstall
cudgel deps
```

### Troubleshooting
```bash
# Get detailed diagnostics
cudgel deps --check --verbose

# If corruption suspected, reinstall
cudgel deps --clean
cudgel deps
```

### Development Workflow
```bash
# During development, check status frequently
cudgel deps --check  # Fast, <2 seconds

# Clean up between testing different models
cudgel deps --clean
# ... test with different model ...
cudgel deps
```

---

## Environment Customization

### Custom PostgreSQL Port

```bash
export CUDGEL_POSTGRES_PORT=54321
cudgel deps
```

### Custom XDG Directories

```bash
export XDG_DATA_HOME=/mnt/ssd/local/share
export XDG_STATE_HOME=/mnt/ssd/local/state
export XDG_CACHE_HOME=/mnt/ssd/cache
cudgel deps --check --verbose  # Verify paths used
```

---

## Integration with Other Commands

All cudgel commands automatically check dependencies on startup:

```bash
# If dependencies not ready
$ cudgel index ./src
✗ Dependencies not ready

Missing:
  - ONNX embedding model

Run: cudgel deps

# After fixing
$ cudgel deps
✓ All dependencies satisfied (1m 30s)

$ cudgel index ./src
Indexing repository at ./src...
✓ Indexed 42 files, 1,234 symbols (3.2s)
```

**Commands that require dependencies**:
- `cudgel index` - Requires database + models
- `cudgel query` - Requires database + models
- `cudgel orchestrator start` - Requires database + models
- `cudgel graph` - Requires database

**Commands that work without dependencies**:
- `cudgel --version`
- `cudgel --help`
- `cudgel deps` (self-contained)

---

## Exit Codes

Useful for scripting and CI/CD:

| Code | Meaning | Example |
|------|---------|---------|
| 0 | Success | All dependencies satisfied or installation complete |
| 1 | Missing | `cudgel deps --check` found missing dependencies |
| 2 | Cancelled | User declined cleanup confirmation |
| 3 | Failed | Installation or cleanup operation failed |
| 4 | Invalid | Invalid flag combination |

**Example CI script**:
```bash
#!/bin/bash
set -e

# Check if dependencies ready
if ! cudgel deps --check; then
  echo "Installing dependencies..."
  cudgel deps
fi

# Proceed with CI tasks
cudgel index ./src
cudgel query "authentication" --format json > results.json
```

---

## Troubleshooting

### Port Already In Use

```
✗ PostgreSQL: Failed to start database

Details:
  Port 45678 is already in use by another process (PID: 9876)

Troubleshooting:
  - Check what's using the port: lsof -i :45678
  - Stop the other process or change CUDGEL_POSTGRES_PORT
  - Retry with: cudgel deps
```

**Solution**:
```bash
# Option 1: Stop conflicting process
kill 9876
cudgel deps

# Option 2: Use different port
export CUDGEL_POSTGRES_PORT=54321
cudgel deps
```

### Download Failed

```
✗ ONNX Model: Download interrupted

Details:
  Network connection lost after 45.2 MB / 100 MB

Troubleshooting:
  - Check your internet connection
  - Retry with: cudgel deps (will resume from 45.2 MB)
  - If issue persists, try verbose mode: cudgel deps --verbose
```

**Solution**:
```bash
# Downloads are resumable - just retry
cudgel deps
```

### Insufficient Disk Space

```
✗ ONNX Model: Insufficient disk space

Details:
  Required: 100 MB
  Available: 45 MB

Troubleshooting:
  - Free up disk space
  - Or change XDG_DATA_HOME to different partition
  - Retry with: cudgel deps
```

**Solution**:
```bash
# Clean up other data or move to larger partition
export XDG_DATA_HOME=/mnt/large-disk/local/share
cudgel deps
```

---

## FAQ

### Q: How much disk space does cudgel need?

**A**: Approximately 150-200 MB:
- ONNX embedding model: ~100 MB
- PostgreSQL database: ~50 MB (grows with indexed code)
- State/logs: ~5 MB

### Q: Can I use my own PostgreSQL instance?

**A**: Currently, cudgel manages its own PostgreSQL instance on port 45678 (or custom via `CUDGEL_POSTGRES_PORT`). Future versions may support external instances.

### Q: Is internet connection required after initial setup?

**A**: No. After `cudgel deps` completes successfully, cudgel works fully offline. Internet is only needed for:
- Initial model download
- Pulling updates (`git pull`)

### Q: What happens if I run `cudgel deps` multiple times?

**A**: Safe to run multiple times - operations are idempotent:
- Already-downloaded models are verified and skipped
- Already-running PostgreSQL is detected
- Already-initialized schema is detected

### Q: How do I update to a newer embedding model?

**A**: 
```bash
# Remove old model
cudgel deps --clean

# Download new model (when supported in future version)
cudgel deps --model sentence-transformers/all-mpnet-base-v2
```

### Q: Can I run multiple cudgel instances?

**A**: Yes, but each needs its own PostgreSQL port:
```bash
# Instance 1 (default)
export CUDGEL_POSTGRES_PORT=45678
cudgel deps

# Instance 2
export CUDGEL_POSTGRES_PORT=45679
export XDG_DATA_HOME=/tmp/cudgel-instance-2/share
cudgel deps
```

---

## Next Steps

After running `cudgel deps` successfully:

1. **Index your first repository**:
   ```bash
   cudgel index ./src
   ```

2. **Run a semantic search**:
   ```bash
   cudgel query "how does authentication work?"
   ```

3. **Start the orchestrator** (if using background indexing):
   ```bash
   cudgel orchestrator start
   ```

4. **Explore the call graph**:
   ```bash
   cudgel graph --symbol main --depth 2
   ```

For more information, see the main [README.md](../../../README.md).

---

## Implementation Status

**Phase**: Design (Phase 1)  
**Status**: Specification complete, implementation pending

This document describes the *intended* behavior. Implementation tasks will be generated via `/speckit.tasks` command.
