# CLI Interface Contract: cudgel deps

**Feature**: 004-auto-deps-management  
**Phase**: 1 (Design)  
**Date**: 2025-11-19

## Overview

This document defines the command-line interface contract for the `cudgel deps` command. This contract is technology-agnostic and specifies the expected behavior, inputs, outputs, and exit codes.

---

## Command: `cudgel deps`

### Purpose

Automatically install, validate, and manage all dependencies required for cudgel to function properly.

### Syntax

```bash
cudgel deps [FLAGS]
```

### Flags

| Flag | Long Form | Type | Default | Description |
|------|-----------|------|---------|-------------|
| `-c` | `--check` | Boolean | false | Validate dependencies without installing/modifying system |
| | `--clean` | Boolean | false | Remove downloaded models and temporary files |
| | `--all` | Boolean | false | (With --clean) Remove all data including database |
| `-v` | `--verbose` | Boolean | false | Show detailed diagnostic information |
| `-h` | `--help` | Boolean | false | Show help message |

### Flag Combinations

| Combination | Behavior |
|-------------|----------|
| (no flags) | Install missing dependencies, show progress |
| `--check` | Validate all dependencies, report status, no modifications |
| `--clean` | Remove models, keep database |
| `--clean --all` | Remove models, stop database, remove all data directories |
| `--check --verbose` | Detailed validation with paths and versions |
| `--verbose` | Detailed installation progress with diagnostic info |

**Invalid Combinations**:
- `--check --clean` → Error: "Cannot combine --check with --clean"
- `--all` without `--clean` → Warning: "--all has no effect without --clean"

---

## Behavior Specification

### Mode 1: Install (Default - No Flags)

**Purpose**: Ensure all dependencies are installed and configured.

**Behavior**:
1. Check if PostgreSQL is running
   - If not: Start PostgreSQL
   - If start fails: Show error with troubleshooting steps
2. Check if database schema is initialized
   - If not: Initialize schema (tables, indexes, extensions)
   - If init fails: Show error with SQL details
3. Check if ONNX model files exist
   - If not: Download from HuggingFace Hub with progress indicator
   - If download fails: Show error with retry instruction
4. Verify all dependencies are satisfied
5. Show success summary

**Expected Output** (all satisfied):
```
Checking dependencies...
  ✓ PostgreSQL running (port 45678)
  ✓ Database schema initialized
  ✓ ONNX embedding model present

✓ All dependencies satisfied (0.8s)
```

**Expected Output** (with installation):
```
Checking dependencies...
  ✓ PostgreSQL running (port 45678)
  ✓ Database schema initialized
  ✗ ONNX embedding model not found

Downloading embedding model...
[00:01:23] =========>----------------------- 45.2 MB/100 MB (3.2 MB/s, 17s)

⠹ Verifying model integrity...
  ✓ Model integrity verified

✓ All dependencies satisfied (1m 28s)
```

**Exit Code**: 0 (success), 1 (dependency missing after install), 3 (installation failed)

---

### Mode 2: Check (`--check`)

**Purpose**: Validate dependencies without modifying system.

**Behavior**:
1. Check if PostgreSQL is running
2. Check if database schema is initialized
3. Check if ONNX model files exist
4. Verify file integrity (size, loadability)
5. Show detailed status table

**Expected Output** (all satisfied):
```
Checking dependencies...

Component                  Status    Details
─────────────────────────  ────────  ────────────────────────────────
PostgreSQL                 ✓ OK      Running on port 45678
Database Schema            ✓ OK      Version 1, 6 tables
ONNX Embedding Model       ✓ OK      100.2 MB, loadable

✓ All dependencies satisfied (0.3s)
```

**Expected Output** (with issues):
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

**Exit Code**: 0 (all satisfied), 1 (one or more missing/corrupted)

---

### Mode 3: Check Verbose (`--check --verbose`)

**Purpose**: Validate dependencies with detailed diagnostic information.

**Behavior**: Same as Mode 2, but includes:
- File paths
- Version numbers
- Process IDs
- Log file locations
- Environment variables (XDG_*)

**Expected Output**:
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

**Exit Code**: 0 (all satisfied), 1 (one or more missing/corrupted)

---

### Mode 4: Clean (`--clean`)

**Purpose**: Remove downloaded models and temporary files (keeps database).

**Behavior**:
1. Confirm operation (if TTY)
2. Remove model files from XDG_DATA_HOME/cudgel/models/
3. Remove temporary cache files
4. Show summary of freed space

**Expected Output**:
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

**Non-TTY Behavior**: Skip confirmation, proceed automatically

**Exit Code**: 0 (success), 2 (user cancelled), 3 (cleanup failed)

---

### Mode 5: Clean All (`--clean --all`)

**Purpose**: Remove all data including models, database, and XDG directories.

**Behavior**:
1. Confirm operation with explicit warning (if TTY)
2. Stop PostgreSQL gracefully
3. Remove model files
4. Remove database data directory
5. Remove all XDG directories (data, state, cache)
6. Show summary

**Expected Output**:
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

**Non-TTY Behavior**: Require explicit `--force` flag (safety measure)

**Exit Code**: 0 (success), 2 (user cancelled), 3 (cleanup failed)

---

## Exit Codes

| Code | Meaning | When Returned |
|------|---------|---------------|
| 0 | Success | All dependencies satisfied (check mode) OR installation completed successfully (install mode) OR cleanup completed successfully (clean mode) |
| 1 | Dependency Missing | One or more dependencies missing or corrupted (check mode only) |
| 2 | User Cancelled | User declined confirmation prompt (clean modes) |
| 3 | Operation Failed | Installation failed (install mode) OR cleanup failed (clean mode) |
| 4 | Invalid Usage | Invalid flag combination or missing required arguments |

---

## Error Message Format

All error messages MUST follow this format:

```
✗ [Component Name]: [Specific Error]

Details:
  [Technical details if helpful]

Troubleshooting:
  - [Step 1 to fix]
  - [Step 2 to fix]
  - [Step 3 to fix]

For more help: https://github.com/roshbhatia/cudgel/issues
```

**Example**:
```
✗ PostgreSQL: Failed to start database

Details:
  Port 45678 is already in use by another process (PID: 9876)

Troubleshooting:
  - Check what's using the port: lsof -i :45678
  - Stop the other process or change CUDGEL_POSTGRES_PORT
  - Retry with: cudgel deps

For more help: https://github.com/roshbhatia/cudgel/issues
```

---

## Progress Indicator Requirements

### Download Operations (>5 seconds expected)

**Format**: Progress bar with:
- Elapsed time
- Current bytes / total bytes
- Download speed
- Estimated time remaining

**Example**:
```
[00:01:23] =========>----------------------- 45.2 MB/100 MB (3.2 MB/s, 17s)
```

### Indeterminate Operations (unknown duration)

**Format**: Spinner with message

**Example**:
```
⠹ Initializing database schema...
```

### TTY Detection

- TTY: Rich progress bars with colors and animation
- Non-TTY: Simple line-by-line output or silent

---

## Environment Variables

| Variable | Purpose | Default | Valid Values |
|----------|---------|---------|--------------|
| `CUDGEL_POSTGRES_PORT` | PostgreSQL server port | 45678 | 1-65535 |
| `CUDGEL_POSTGRES_HOST` | PostgreSQL server host | localhost | Valid hostname |
| `XDG_DATA_HOME` | Data directory base | ~/.local/share | Absolute path |
| `XDG_STATE_HOME` | State directory base | ~/.local/state | Absolute path |
| `XDG_CACHE_HOME` | Cache directory base | ~/.cache | Absolute path |

---

## Integration with Other Commands

All cudgel commands that require dependencies MUST check dependency satisfaction on startup:

```
$ cudgel index ./src
✗ Dependencies not ready

Missing:
  - ONNX embedding model

Run: cudgel deps
```

**Behavior**:
1. Lightweight check (< 100ms overhead)
2. If missing: Show error and exit with code 1
3. If satisfied: Proceed with command

**Commands Affected**:
- `cudgel index` (requires database + models)
- `cudgel query` (requires database + models)
- `cudgel orchestrator start` (requires database + models)
- `cudgel graph` (requires database)

**Commands NOT Affected**:
- `cudgel deps` (self-contained)
- `cudgel --version`
- `cudgel --help`

---

## Acceptance Criteria

### Functional Requirements (from spec.md)

| Requirement | Test Method |
|-------------|-------------|
| FR-001: Provide `cudgel deps` command | Run `cudgel deps --help` → shows usage |
| FR-002: Auto-download ONNX model | Run on fresh system → model appears in XDG_DATA_HOME |
| FR-003: Store in XDG directories | Set XDG_DATA_HOME → verify files written there |
| FR-004: Auto-start PostgreSQL | Run on fresh system with DB stopped → DB starts |
| FR-005: Auto-initialize schema | Run on fresh DB → tables created |
| FR-006: Provide --check flag | Run `cudgel deps --check` → validation only |
| FR-007: Provide --clean flag | Run `cudgel deps --clean` → models removed |
| FR-008: Provide --clean --all flag | Run with confirmation → all data removed |
| FR-009: Eliminate manual instructions | Check error messages → none contain setup steps |
| FR-010: Verify checksums | Corrupt file → detected on validation |
| FR-011: Progress indicators | Monitor TTY output → progress bars visible |
| FR-012: Detect missing prerequisites | Uninstall PostgreSQL → error with install instructions |
| FR-013: Commands validate deps | Run `cudgel index` with missing deps → actionable error |
| FR-014: Handle partial downloads | Interrupt download → resume on retry |
| FR-015: Respect XDG variables | Set all XDG_* vars → verify paths used |

### Success Criteria (from spec.md)

| Criterion | Measurement Method |
|-----------|-------------------|
| SC-001: <5 min setup | Time fresh install → measure elapsed time |
| SC-002: Zero manual steps | Fresh system → run `cudgel deps` only |
| SC-003: <2 sec validation | Run `cudgel deps --check` → measure time |
| SC-004: Error messages updated | Grep codebase for setup instructions → none found |
| SC-005: XDG compliance | Test with custom XDG vars → verify paths |
| SC-006: 100% actionable errors | Review all error messages → all have steps |
| SC-007: Retry without intervention | Fail install → re-run succeeds |
| SC-008: Disk space check | Fill disk → error before download attempt |

---

## Non-Functional Requirements

### Performance

- Dependency check (all satisfied): < 2 seconds
- Model download: < 5 minutes (assuming 10 Mbps connection)
- Database initialization: < 10 seconds
- Schema initialization: < 5 seconds

### Reliability

- Idempotent operations: Running `cudgel deps` multiple times must be safe
- Atomic operations: Failed installations must not corrupt existing setup
- Resumable downloads: Network interruptions must not require starting over

### Usability

- Progress feedback for all operations > 5 seconds
- Clear error messages with specific remediation steps
- Confirmation prompts for destructive operations (--clean --all)
- Graceful degradation in non-TTY environments

### Security

- HTTPS-only downloads from HuggingFace Hub
- No execution of untrusted code
- File permissions preserved (user-only for database files)
- No credential storage (uses PostgreSQL trust authentication locally)

---

## Examples

### Example 1: First-Time Setup
```bash
$ cudgel deps
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

### Example 2: Status Check
```bash
$ cudgel deps --check
Checking dependencies...

Component                  Status    Details
─────────────────────────  ────────  ────────────────────────────────
PostgreSQL                 ✓ OK      Running on port 45678
Database Schema            ✓ OK      Version 1, 6 tables
ONNX Embedding Model       ✓ OK      100.2 MB, loadable

✓ All dependencies satisfied (0.3s)
```

### Example 3: Cleanup
```bash
$ cudgel deps --clean
⚠ This will remove downloaded models (~100 MB)
   Database will be preserved.

Proceed? (y/N): y

Cleaning up dependencies...
  ✓ Removed model files (100.2 MB freed)

✓ Cleanup complete (100.2 MB freed)

To reinstall: cudgel deps
```

---

## Implementation Notes

This contract is technology-agnostic. Implementation details:
- Command parsing: clap crate (existing in cudgel)
- Progress bars: indicatif crate (existing, upgrade to 0.18)
- Model download: hf-hub crate (new dependency)
- Database management: Shell out to existing scripts (scripts/start-postgres.sh)

See research.md for detailed technical decisions.
