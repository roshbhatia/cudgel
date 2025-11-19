# CLI Interface Contract: Automatic Re-indexing

**Feature**: 002-automatic-re-indexing  
**Date**: 2025-11-19  
**Status**: Complete

## Overview

This document defines the command-line interface contract for the automatic re-indexing feature. All commands follow Cudgel's existing CLI conventions using `clap`.

---

## Commands

### 1. Schedule Indexing

**Command**: `cudgel index --schedule <FREQUENCY> <PATH>`

**Purpose**: Schedule automatic periodic indexing for a repository.

**Arguments**:
- `<PATH>` (required): Path to repository to schedule
  - Type: String (filesystem path)
  - Validation: Must be a valid directory path
  - Converted to canonical absolute path

**Options**:
- `--schedule <FREQUENCY>` (required): Scheduling frequency
  - Type: String enum
  - Valid values: `hourly`, `daily`, `weekly`
  - Case-insensitive

**Examples**:
```bash
cudgel index --schedule hourly /path/to/repo
cudgel index --schedule daily ~/projects/myapp
cudgel index --schedule weekly .
```

**Success Output**:
```
✓ Scheduled hourly indexing for /absolute/path/to/repo
  Next run: 2025-11-19 15:00:00 UTC
  Orchestrator status: running (PID 12345)
```

**Error Cases**:

| Error | Exit Code | Message |
|-------|-----------|---------|
| Repository not found | 1 | `Error: Repository not indexed. Run 'cudgel index <path>' first` |
| Invalid path | 1 | `Error: Path does not exist: <path>` |
| Invalid frequency | 1 | `Error: Invalid frequency '<freq>'. Must be: hourly, daily, weekly` |
| Already scheduled | 1 | `Error: Repository already scheduled (<frequency>). Use --unschedule first` |
| Database error | 1 | `Error: Database connection failed. Is PostgreSQL running on port 54321?` |

**Side Effects**:
1. Inserts row in `scheduled_tasks` table
2. Starts orchestrator daemon if not running
3. Calculates `next_run_at` based on frequency

---

### 2. Unschedule Indexing

**Command**: `cudgel index --unschedule <PATH>`

**Purpose**: Remove automatic indexing schedule for a repository.

**Arguments**:
- `<PATH>` (required): Path to repository to unschedule
  - Type: String (filesystem path)
  - Validation: Must be a valid directory path
  - Converted to canonical absolute path

**Examples**:
```bash
cudgel index --unschedule /path/to/repo
cudgel index --unschedule ~/projects/myapp
```

**Success Output**:
```
✓ Removed schedule for /absolute/path/to/repo
```

**Error Cases**:

| Error | Exit Code | Message |
|-------|-----------|---------|
| Repository not found | 1 | `Error: Repository not indexed: <path>` |
| Not scheduled | 1 | `Error: Repository not scheduled: <path>` |
| Invalid path | 1 | `Error: Path does not exist: <path>` |
| Database error | 1 | `Error: Database connection failed. Is PostgreSQL running on port 54321?` |

**Side Effects**:
1. Deletes row from `scheduled_tasks` table
2. Does NOT stop orchestrator daemon (may have other scheduled tasks)

---

### 3. Start Orchestrator

**Command**: `cudgel orchestrator start`

**Purpose**: Start the orchestrator daemon in the background.

**Arguments**: None

**Options**: None

**Examples**:
```bash
cudgel orchestrator start
```

**Success Output**:
```
✓ Orchestrator started (PID 12345)
  PID file: /Users/user/.local/var/run/cudgel/orchestrator.pid
  Log file: /Users/user/.local/state/cudgel/orchestrator.log
  Polling interval: 60 seconds
```

**Error Cases**:

| Error | Exit Code | Message |
|-------|-----------|---------|
| Already running | 1 | `Error: Orchestrator already running (PID 12345)` |
| PID file locked | 1 | `Error: Cannot acquire PID lock. Another instance may be running` |
| Permission denied | 1 | `Error: Cannot create PID file: Permission denied` |
| Database error | 1 | `Error: Database connection failed. Is PostgreSQL running on port 54321?` |

**Side Effects**:
1. Creates PID file with exclusive lock
2. Forks background process (daemon)
3. Starts 60-second polling loop
4. Creates log file

**Process Behavior**:
- Runs as background daemon (detached from terminal)
- Polls database every 60 seconds
- Executes due tasks in parallel (per repository)
- Handles SIGTERM/SIGINT for graceful shutdown

---

### 4. Stop Orchestrator

**Command**: `cudgel orchestrator stop`

**Purpose**: Stop the orchestrator daemon gracefully.

**Arguments**: None

**Options**: None

**Examples**:
```bash
cudgel orchestrator stop
```

**Success Output**:
```
✓ Orchestrator stopped (PID 12345)
  Graceful shutdown: completed in 3s
```

**Error Cases**:

| Error | Exit Code | Message |
|-------|-----------|---------|
| Not running | 1 | `Error: Orchestrator not running` |
| Stale PID file | 0 | `Warning: Removed stale PID file` |
| Kill failed | 1 | `Error: Failed to stop orchestrator (PID 12345): Permission denied` |
| Timeout | 1 | `Error: Orchestrator did not stop within 30 seconds. Forced termination` |

**Side Effects**:
1. Sends SIGTERM to orchestrator process
2. Waits up to 30 seconds for graceful shutdown
3. Sends SIGKILL if timeout expires
4. Removes PID file

**Graceful Shutdown Behavior**:
1. Stop accepting new tasks
2. Finish current task (if any, with timeout)
3. Close database connections
4. Remove PID file
5. Flush logs

---

### 5. Restart Orchestrator

**Command**: `cudgel orchestrator restart`

**Purpose**: Restart the orchestrator daemon (stop + start).

**Arguments**: None

**Options**: None

**Examples**:
```bash
cudgel orchestrator restart
```

**Success Output**:
```
✓ Orchestrator stopped (PID 12345)
✓ Orchestrator started (PID 12346)
  PID file: /Users/user/.local/var/run/cudgel/orchestrator.pid
```

**Error Cases**: Same as stop + start

**Side Effects**:
1. Executes `stop` command
2. Executes `start` command
3. New PID assigned

---

### 6. Orchestrator Status

**Command**: `cudgel orchestrator status`

**Purpose**: Check orchestrator status and list scheduled tasks.

**Arguments**: None

**Options**: None

**Examples**:
```bash
cudgel orchestrator status
```

**Success Output (Running)**:
```
Orchestrator Status: RUNNING (PID 12345)
  Started: 2025-11-19 14:00:00 UTC
  Uptime: 2h 15m
  PID file: /Users/user/.local/var/run/cudgel/orchestrator.pid
  Log file: /Users/user/.local/state/cudgel/orchestrator.log

Scheduled Tasks (3):
  1. /path/to/repo1
     Frequency: hourly
     Last run: 2025-11-19 15:00:00 UTC (1h ago)
     Next run: 2025-11-19 16:00:00 UTC (in 15m)
     Status: ✓ Success

  2. /path/to/repo2
     Frequency: daily
     Last run: 2025-11-18 00:00:00 UTC (yesterday)
     Next run: 2025-11-19 00:00:00 UTC (tomorrow)
     Status: ✓ Success

  3. /path/to/repo3
     Frequency: weekly
     Last run: Never
     Next run: 2025-11-20 00:00:00 UTC (Mon)
     Status: ⏳ Pending
```

**Success Output (Not Running)**:
```
Orchestrator Status: NOT RUNNING

Scheduled Tasks (3):
  [same format as above]

To start orchestrator: cudgel orchestrator start
```

**Error Cases**:

| Error | Exit Code | Message |
|-------|-----------|---------|
| Database error | 1 | `Error: Database connection failed. Is PostgreSQL running on port 54321?` |
| Stale PID file | 0 | `Warning: Orchestrator not running (stale PID file removed)` |

**Side Effects**: None (read-only command)

---

## CLI Structure

### Clap Command Definitions

```rust
#[derive(Parser)]
#[command(name = "cudgel")]
#[command(about = "Semantic code search and indexing tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Index {
        /// Path to repository
        path: PathBuf,
        
        /// Schedule automatic indexing
        #[arg(long, value_name = "FREQUENCY")]
        schedule: Option<String>,
        
        /// Remove scheduled indexing
        #[arg(long)]
        unschedule: bool,
        
        // ... existing flags
    },
    
    Orchestrator {
        #[command(subcommand)]
        action: OrchestratorAction,
    },
}

#[derive(Subcommand)]
enum OrchestratorAction {
    /// Start the orchestrator daemon
    Start,
    
    /// Stop the orchestrator daemon
    Stop,
    
    /// Restart the orchestrator daemon
    Restart,
    
    /// Show orchestrator status and scheduled tasks
    Status,
}
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (with user-friendly message) |
| 2 | Usage error (invalid arguments) |

---

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `CUDGEL_PID_DIR` | Override PID file directory | `~/.local/var/run/cudgel` |
| `CUDGEL_LOG_DIR` | Override log file directory | `~/.local/state/cudgel` |
| `CUDGEL_DB_URL` | PostgreSQL connection string | `postgresql://localhost:54321/cudgel` |

---

## File Locations

| File | Location | Purpose |
|------|----------|---------|
| PID file | `~/.local/var/run/cudgel/orchestrator.pid` | Process ID for daemon management |
| Log file | `~/.local/state/cudgel/orchestrator.log` | Orchestrator daemon logs |
| Database | PostgreSQL on `localhost:54321` | Persistent storage |

---

## Validation Rules

### Path Validation
1. Path MUST exist on filesystem
2. Path MUST be a directory (not a file)
3. Path is converted to canonical absolute path
4. Repository MUST be indexed before scheduling

### Frequency Validation
1. MUST be one of: `hourly`, `daily`, `weekly`
2. Case-insensitive matching
3. Invalid values produce actionable error message

### PID File Validation
1. PID MUST be positive integer
2. Process MUST exist (check via `/proc/<pid>` or `kill -0`)
3. Stale PID files automatically removed

---

## Backwards Compatibility

**Existing Commands**: No breaking changes to existing `cudgel index <path>` command.

**New Flags**: `--schedule` and `--unschedule` are optional additions.

**New Subcommand**: `orchestrator` is a new top-level subcommand.

---

## Testing Contract

### Unit Tests
```rust
#[test]
fn test_parse_schedule_flag() {
    let args = vec!["cudgel", "index", "--schedule", "hourly", "/path"];
    let cli = Cli::parse_from(args);
    assert_eq!(cli.schedule, Some("hourly".to_string()));
}

#[test]
fn test_invalid_frequency() {
    let args = vec!["cudgel", "index", "--schedule", "invalid", "/path"];
    let result = Cli::try_parse_from(args);
    assert!(result.is_err());
}
```

### Integration Tests
```bash
# Schedule task
cudgel index --schedule hourly ./test-repo
assert_exit_code 0
assert_stdout_contains "Scheduled hourly indexing"

# Check status
cudgel orchestrator status
assert_exit_code 0
assert_stdout_contains "test-repo"
assert_stdout_contains "hourly"

# Unschedule
cudgel index --unschedule ./test-repo
assert_exit_code 0
```

---

## Summary

**New Commands**: 6 total
- 2 index subcommand flags (`--schedule`, `--unschedule`)
- 4 orchestrator subcommands (`start`, `stop`, `restart`, `status`)

**Key Properties**:
- All commands return actionable error messages
- Status command provides full visibility
- Graceful error handling for all failure modes
- Backwards compatible with existing CLI

**No External APIs**: This is a CLI-only interface (no REST/GraphQL).
