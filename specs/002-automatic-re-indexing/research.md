# Research: Automatic Re-indexing

**Feature**: 002-automatic-re-indexing  
**Date**: 2025-11-19  
**Status**: Complete

## Overview

Research conducted to inform implementation decisions for the automatic re-indexing orchestrator daemon. This document consolidates findings on daemon management, PID file handling, graceful shutdown, and background task scheduling.

---

## 1. Daemon Process Management

### Decision: Foreground Service (No Daemonization)

**Rationale**: Modern best practice (Rust 2021+) strongly favors running services in foreground, letting platform process managers (systemd/launchd) handle process management.

**Implementation Approach**:
- Run orchestrator as foreground tokio process
- Write logs to stdout/stderr (captured by process managers)
- Use PID file only for `cudgel orchestrator status` command
- No fork/detach logic needed

**Alternatives Considered**:
- ❌ **Traditional daemonization** (`daemonize` crate): Unmaintained, incompatible with tokio async
- ❌ **Manual fork/setsid**: Unnecessary complexity, only for legacy systems
- ✅ **Foreground + process managers**: Simpler, better debugging, automatic restart, centralized logging

**Required Crates**:
```toml
tokio = { version = "1", features = ["full", "signal"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

**Key Patterns**:
```rust
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let shutdown = setup_shutdown_signal();
    
    loop {
        tokio::select! {
            _ = shutdown.recv() => break,
            _ = tokio::time::sleep(Duration::from_secs(60)) => {
                process_scheduled_tasks().await?;
            }
        }
    }
    Ok(())
}
```

**Platform Integration**:
- **Linux**: systemd unit file (`Type=simple`, `Restart=on-failure`)
- **macOS**: launchd plist (`KeepAlive=true`, `RunAtLoad=true`)

---

## 2. PID File Handling

### Decision: XDG + Atomic Creation + Advisory Lock

**PID File Location**:
- **Primary**: `$HOME/.local/var/run/cudgel/orchestrator.pid` (XDG fallback)
- **Alternative**: `$XDG_RUNTIME_DIR/cudgel/orchestrator.pid` (if available)

**File Format**: Plain ASCII `<pid>\n` (e.g., `12345\n`)

**Locking Strategy**:
1. **Atomic creation**: `File::options().create_new(true)` detects existing files
2. **Advisory lock**: `file.lock()` held for daemon lifetime
3. **Stale detection**: `file.try_lock()` succeeds = stale, fails (WouldBlock) = running

**Implementation Pattern**:
```rust
pub struct PidLock {
    file: File,
    path: PathBuf,
}

impl PidLock {
    pub fn acquire(path: PathBuf) -> Result<Self> {
        // Auto-removes stale PID files
        if path.exists() {
            if Self::is_running(&path)? {
                return Err(Error::AlreadyRunning);
            }
            fs::remove_file(&path)?;
        }
        
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .open(&path)?;
        
        file.lock_exclusive()?;
        write!(file, "{}\n", std::process::id())?;
        
        Ok(Self { file, path })
    }
    
    pub fn is_running(path: &Path) -> Result<bool> {
        let file = File::open(path)?;
        Ok(file.try_lock_exclusive().is_err())
    }
}

impl Drop for PidLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
```

**Error Handling**:
| Error | Detection | User Message |
|-------|-----------|--------------|
| Already running | `try_lock()` fails | "Orchestrator already running (PID X). Use 'cudgel orchestrator stop'" |
| Stale file | `try_lock()` succeeds | Auto-remove + optional warning |
| Permission denied | Create fails | "Cannot create PID file: Permission denied" |
| Invalid PID | Parse error | Treat as stale, remove + retry |

---

## 3. Graceful Shutdown

### Decision: 3-Phase Architecture (Detect → Notify → Complete)

**Signal Handling**:
```rust
async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    
    #[cfg(unix)]
    let mut terminate = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate()
    ).expect("Failed to install SIGTERM handler");
    
    tokio::select! {
        _ = ctrl_c => {},
        #[cfg(unix)]
        _ = terminate.recv() => {},
    }
}
```

**Shutdown Coordination**:
1. **Phase 1 (Detection)**: `tokio::select!` catches SIGTERM/SIGINT
2. **Phase 2 (Notification)**: `broadcast::channel` notifies all tasks
3. **Phase 3 (Completion)**: `mpsc::channel` tracks task completion

**Pattern**:
```rust
// Setup
let (notify_shutdown, _) = broadcast::channel(1);
let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);

// Task coordination
let mut shutdown_rx = notify_shutdown.subscribe();
let _shutdown_complete = shutdown_complete_tx.clone();

// Shutdown sequence
drop(notify_shutdown); // Broadcast to all tasks
drop(shutdown_complete_tx); // Signal we're waiting

// Wait with timeout
match timeout(Duration::from_secs(30), shutdown_complete_rx.recv()).await {
    Ok(_) => info!("Graceful shutdown"),
    Err(_) => warn!("Forced termination after timeout"),
}

// Cleanup
drop(pid_lock); // Removes PID file via Drop
```

**In-Flight Work Handling**:
```rust
loop {
    tokio::select! {
        _ = interval.tick() => {
            if shutdown.is_shutdown() { break; }
            // Execute long-running task
        }
        _ = shutdown.recv() => { break; }
    }
}
```

**Timeout Strategy**:
- **Per-task timeout**: 5 minutes (prevents individual task hangs)
- **Graceful shutdown timeout**: 30 seconds (allows finishing current task)
- **Forced termination**: After 30s timeout expires

**Resource Cleanup Order**:
1. Stop accepting new work
2. Finish in-flight tasks (with timeout)
3. Close database connections
4. Remove PID file (via Drop)
5. Flush logs

---

## 4. Background Task Scheduling

### Decision: 60s Polling + Optimistic Locking + Parallel Execution

**Polling Interval**: 60 seconds with exponential backoff when idle
- Base: 60s (sufficient for hourly+ schedules)
- Max: 180s (reduces DB load when no work)

**Database Query Pattern**:
```sql
-- Find due tasks
SELECT * FROM scheduled_tasks
WHERE next_run_at <= NOW()
  AND enabled = true
  AND status = 'idle'
ORDER BY next_run_at ASC
LIMIT 100;

-- Claim task (optimistic locking prevents duplicate execution)
UPDATE scheduled_tasks
SET status = 'running', 
    version = version + 1,
    started_at = NOW()
WHERE id = $1 AND version = $2 AND status = 'idle'
RETURNING *;
```

**Required Indexes**:
```sql
CREATE INDEX idx_tasks_next_run ON scheduled_tasks(next_run_at) 
WHERE enabled = true;

CREATE INDEX idx_tasks_status ON scheduled_tasks(status);
```

**Schedule Calculation**:
```rust
enum ScheduleType {
    Hourly,
    Daily,    // Midnight UTC
    Weekly,   // Monday midnight UTC
}

fn calculate_next_run(schedule: ScheduleType, last_run: DateTime<Utc>) -> DateTime<Utc> {
    match schedule {
        ScheduleType::Hourly => last_run + Duration::hours(1),
        ScheduleType::Daily => (last_run + Duration::days(1))
            .with_hour(0).unwrap()
            .with_minute(0).unwrap(),
        ScheduleType::Weekly => (last_run + Duration::weeks(1))
            .with_weekday(Weekday::Mon).unwrap()
            .with_hour(0).unwrap(),
    }
}
```

**Concurrency Strategy**: Parallel per repository, sequential per task
```rust
for task in due_tasks {
    let db = Arc::clone(&db);
    tokio::spawn(async move {
        if let Ok(Some(locked_task)) = claim_task(&db, task.id, task.version).await {
            execute_indexing(&db, locked_task).await;
        }
    });
}
```

**Failure/Retry Handling**: Exponential backoff + dead letter queue
```rust
struct RetryPolicy {
    max_retries: i32,           // Default: 5
    retry_count: i32,
    error_message: Option<String>,
}

// Backoff: 1min, 2min, 4min, 8min, 16min
let backoff = Duration::minutes(2_i64.pow(retry_count as u32));

// After max retries, move to failed_tasks table
```

**Time Handling**:
- **Always UTC**: Store all timestamps as `TIMESTAMPTZ` in PostgreSQL
- **Database as source**: Use `SELECT NOW() AT TIME ZONE 'UTC'` (avoids clock skew)
- **DST-agnostic**: All calculations in UTC (no spring-forward gaps)

**Exactly-Once Execution**:
1. Query tasks where `next_run_at <= NOW()`
2. Claim with optimistic lock (`UPDATE WHERE version = $old`)
3. If claim succeeds, execute indexing
4. Update `last_run_at`, calculate `next_run_at`, reset `status = 'idle'`

---

## Implementation Summary

**Key Technologies**:
- `tokio` with signal handling (SIGTERM/SIGINT)
- `broadcast::channel` for shutdown notification
- `mpsc::channel` for completion tracking
- `chrono` for UTC time handling
- File advisory locks for PID management

**Architecture Decisions**:
1. **Foreground service** (no daemonization)
2. **PID file for status only** (not process lifecycle)
3. **3-phase shutdown** (detect → notify → complete)
4. **60s polling** with optimistic locking
5. **Parallel execution** per repository
6. **Exponential backoff** for retries
7. **Always UTC** time handling

**No New Dependencies Required**: All patterns implementable with existing dependencies (tokio, chrono, postgres).

---

## References

- Tokio mini-redis shutdown patterns: https://github.com/tokio-rs/mini-redis
- FHS/XDG specifications for PID file locations
- PostgreSQL optimistic locking patterns
- Brandur Leach's transactionally-staged job drains
