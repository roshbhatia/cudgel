# Feature Specification: Automatic Re-indexing

**Feature Branch**: `002-automatic-re-indexing`
**Created**: 2025-11-01
**Status**: Draft
**Parent Feature**: 001-init-code-indexing-tool

## Overview

This feature adds automatic re-indexing capabilities to Cudgel, allowing developers to schedule periodic updates to their code indexes without manual intervention. The orchestrator daemon manages scheduled tasks in the background.

## User Story - Schedule Automatic Re-indexing (Priority: P2)

A developer wants their actively-developed repository to stay indexed automatically, without manual re-indexing after every code change.

**Why this priority**: Automation improves the developer experience by keeping the index fresh. This builds on User Story 1 by adding scheduling capability.

**Independent Test**: Can be fully tested by running `cudgel index --schedule hourly /path/to/repo`, confirming the orchestrator daemon is running, waiting for the scheduled interval, and verifying the repository is re-indexed automatically.

### Acceptance Scenarios

1. **Given** a repository, **When** the developer runs `cudgel index --schedule hourly /path/to/repo`, **Then** the system stores the schedule in the database and starts the orchestrator daemon (if not already running) to execute indexing every hour.

2. **Given** a scheduled indexing job, **When** the orchestrator daemon reaches the scheduled time, **Then** the system automatically runs incremental indexing for the repository without user intervention.

3. **Given** multiple repositories with different schedules, **When** the orchestrator daemon is running, **Then** it manages all scheduled jobs concurrently and executes each according to its schedule.

4. **Given** the orchestrator daemon is running, **When** the developer runs `cudgel orchestrator status`, **Then** the system displays all scheduled jobs with their next run time and last execution status.

5. **Given** a scheduled job, **When** the developer runs `cudgel index --unschedule /path/to/repo`, **Then** the system removes the schedule from the database and stops auto-indexing that repository.

## Technical Design

### Database Schema

Uses existing `scheduled_tasks` table:
```sql
CREATE TABLE scheduled_tasks (
    id SERIAL PRIMARY KEY,
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    schedule_type VARCHAR(50) NOT NULL,
    schedule_value TEXT NOT NULL,
    last_run_at TIMESTAMPTZ,
    next_run_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(repository_id)
);
```

### CLI Commands

#### Schedule Management
```bash
# Add scheduled indexing
cudgel index --schedule hourly /path/to/repo
cudgel index --schedule daily /path/to/repo
cudgel index --schedule weekly /path/to/repo

# Remove scheduled indexing
cudgel index --unschedule /path/to/repo
```

#### Orchestrator Daemon
```bash
# Start orchestrator in background
cudgel orchestrator start

# Stop orchestrator
cudgel orchestrator stop

# Check orchestrator status
cudgel orchestrator status

# Restart orchestrator
cudgel orchestrator restart
```

### Architecture

**Orchestrator (`src/orchestrator.rs`)**:
- Runs as background daemon process
- Polling loop (60-second interval)
- Queries database for tasks where `next_run_at <= NOW()`
- Executes indexing via existing `Indexer` service
- Updates `last_run_at` and calculates `next_run_at`
- Graceful shutdown on SIGTERM/SIGINT

**Process Management**:
- PID file: `~/.local/state/cudgel/orchestrator.pid`
- Log file: `~/.local/state/cudgel/orchestrator.log`
- Daemon runs detached from terminal

**Schedule Types**:
- `hourly`: Every 60 minutes
- `daily`: Every 24 hours
- `weekly`: Every 7 days

### Implementation Components

1. **Database Operations** (`src/database.rs`):
   - `create_scheduled_task(repo_id, schedule_type)`
   - `delete_scheduled_task(repo_id)`
   - `get_scheduled_tasks()` - All scheduled tasks
   - `get_due_tasks()` - Tasks where next_run_at <= now
   - `update_task_execution(task_id, last_run, next_run)`

2. **Orchestrator Module** (`src/orchestrator.rs`):
   - `start_daemon()` - Fork and daemonize
   - `stop_daemon()` - Send SIGTERM to PID
   - `get_status()` - Read PID file and check process
   - `run_polling_loop()` - Main orchestrator loop
   - `execute_task(task)` - Run indexing for a task

3. **CLI Updates** (`src/main.rs`):
   - Add `--schedule` and `--unschedule` to Index command
   - Add `Orchestrator` subcommand with start/stop/status/restart

### Error Handling

- **Database unavailable**: Log error, retry on next poll
- **Repository not found**: Mark task as failed, log warning
- **Indexing fails**: Log error, don't update last_run_at
- **Multiple orchestrators**: Check PID file, refuse to start if running

### Logging

- Log to `~/.local/state/cudgel/orchestrator.log`
- Rotation: Keep last 10MB, rotate when >10MB
- Format: `[TIMESTAMP] [LEVEL] message`
- Levels: INFO, WARN, ERROR

## Testing Strategy

### Unit Tests
- Schedule interval calculations
- Task filtering logic (due vs not due)
- PID file read/write operations

### Integration Tests
- Create scheduled task via CLI
- Verify task stored in database
- Mock time and verify task execution
- Test daemon start/stop/status
- Test graceful shutdown

### Manual Testing
```bash
# Schedule a test repo
cudgel index --schedule hourly ./test-repo

# Start orchestrator
cudgel orchestrator start

# Check status
cudgel orchestrator status

# Wait and verify indexing occurred
sleep 3600
cudgel orchestrator status  # Should show updated last_run_at

# Cleanup
cudgel index --unschedule ./test-repo
cudgel orchestrator stop
```

## Dependencies

- No new external dependencies required
- Uses existing: tokio, chrono, tracing

## Implementation Checklist

- [ ] Add database operations for scheduled_tasks
- [ ] Implement orchestrator.rs with daemon management
- [ ] Add --schedule/--unschedule flags to index command
- [ ] Add orchestrator subcommand (start/stop/status/restart)
- [ ] Implement PID file management
- [ ] Add graceful shutdown handling
- [ ] Implement logging to file
- [ ] Write unit tests for scheduling logic
- [ ] Write integration tests for orchestrator
- [ ] Update CLAUDE.md with implementation status
- [ ] Update README with orchestrator documentation

## Success Metrics

- Orchestrator can run continuously for 24+ hours without crashing
- Scheduled tasks execute within 60 seconds of scheduled time
- Daemon uses <50MB RAM when idle
- All scheduled tasks complete successfully
