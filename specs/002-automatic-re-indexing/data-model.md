# Data Model: Automatic Re-indexing

**Feature**: 002-automatic-re-indexing  
**Date**: 2025-11-19  
**Status**: Complete

## Overview

Data model for automatic re-indexing feature, using the existing `scheduled_tasks` table in the PostgreSQL database. This feature does not introduce new entities but extends the usage of existing schema.

---

## Entities

### 1. ScheduledTask

**Purpose**: Represents a scheduled indexing job for a repository.

**Database Table**: `scheduled_tasks` (already exists)

**Schema**:
```sql
CREATE TABLE scheduled_tasks (
    id SERIAL PRIMARY KEY,
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    schedule_type VARCHAR(50) NOT NULL,      -- 'hourly', 'daily', 'weekly'
    schedule_value TEXT NOT NULL,             -- JSON metadata (currently unused)
    last_run_at TIMESTAMPTZ,                  -- When task last executed
    next_run_at TIMESTAMPTZ NOT NULL,         -- When task should run next
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    -- Extension fields (added by this feature)
    status VARCHAR(20) DEFAULT 'idle',        -- 'idle', 'running'
    version INTEGER DEFAULT 1,                -- Optimistic locking
    retry_count INTEGER DEFAULT 0,            -- Retry attempts
    error_message TEXT,                       -- Last error (if any)
    
    UNIQUE(repository_id)                     -- One schedule per repo
);
```

**Indexes**:
```sql
CREATE INDEX idx_tasks_next_run 
ON scheduled_tasks(next_run_at) 
WHERE status = 'idle';

CREATE INDEX idx_tasks_status 
ON scheduled_tasks(status);
```

**Fields**:

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | SERIAL | PRIMARY KEY | Unique task identifier |
| `repository_id` | INTEGER | NOT NULL, FOREIGN KEY | Reference to repositories table |
| `schedule_type` | VARCHAR(50) | NOT NULL | Schedule frequency: 'hourly', 'daily', 'weekly' |
| `schedule_value` | TEXT | NOT NULL | JSON metadata (reserved for future use) |
| `last_run_at` | TIMESTAMPTZ | NULLABLE | UTC timestamp of last successful execution |
| `next_run_at` | TIMESTAMPTZ | NOT NULL | UTC timestamp when task should run next |
| `created_at` | TIMESTAMPTZ | DEFAULT NOW() | UTC timestamp when task was created |
| `status` | VARCHAR(20) | DEFAULT 'idle' | Execution status: 'idle' or 'running' |
| `version` | INTEGER | DEFAULT 1 | Optimistic locking version counter |
| `retry_count` | INTEGER | DEFAULT 0 | Number of retry attempts (resets on success) |
| `error_message` | TEXT | NULLABLE | Last error message (if execution failed) |

**Relationships**:
- **Many-to-One**: `scheduled_tasks.repository_id` → `repositories.id`
- **Cascade Delete**: When repository is deleted, all its scheduled tasks are deleted

**Validation Rules**:
1. `schedule_type` MUST be one of: 'hourly', 'daily', 'weekly'
2. `next_run_at` MUST be in the future when task is created
3. `status` MUST be one of: 'idle', 'running'
4. `version` MUST increment on every claim/update
5. `retry_count` MUST be >= 0 and <= 5 (max retries)
6. Only one scheduled task per repository (enforced by UNIQUE constraint)

**State Transitions**:
```
idle ──[claim]──> running ──[success]──> idle
                   │                      │
                   └──[failure]──> idle   │
                                   (retry_count++)
                   │
                   └──[max_retries]──> DELETE
                                       (move to failed_tasks)
```

---

### 2. PidLock (Runtime Entity)

**Purpose**: Manages orchestrator daemon process lifecycle via PID file.

**Storage**: File system (`~/.local/var/run/cudgel/orchestrator.pid`)

**Format**: Plain text ASCII
```
<pid>\n
```

**Example**:
```
12345
```

**Fields** (in-memory representation):
```rust
pub struct PidLock {
    file: File,           // File handle with exclusive lock
    path: PathBuf,        // Path to PID file
}
```

**Lifecycle**:
1. **Creation**: Atomic creation on daemon start (`create_new(true)`)
2. **Lock**: Exclusive advisory lock held for daemon lifetime
3. **Cleanup**: Automatic removal via `Drop` on daemon shutdown

**Validation Rules**:
1. PID file MUST contain valid process ID (positive integer)
2. PID file MUST be removable when process is not running (stale detection)
3. PID file directory MUST be writable by user

---

## Database Operations

### ScheduledTask CRUD

**Create**:
```rust
async fn create_scheduled_task(
    &self,
    repo_id: i32,
    schedule_type: &str,
) -> Result<ScheduledTask> {
    let next_run = calculate_next_run(schedule_type, Utc::now());
    
    self.client.query_one(
        "INSERT INTO scheduled_tasks 
         (repository_id, schedule_type, schedule_value, next_run_at)
         VALUES ($1, $2, $3, $4)
         RETURNING *",
        &[&repo_id, &schedule_type, &"{}", &next_run]
    ).await
}
```

**Read (Due Tasks)**:
```rust
async fn get_due_tasks(&self) -> Result<Vec<ScheduledTask>> {
    self.client.query(
        "SELECT * FROM scheduled_tasks
         WHERE next_run_at <= NOW()
           AND status = 'idle'
         ORDER BY next_run_at ASC
         LIMIT 100",
        &[]
    ).await
}
```

**Update (Claim Task)**:
```rust
async fn claim_task(
    &self,
    task_id: i32,
    version: i32,
) -> Result<Option<ScheduledTask>> {
    let result = self.client.query_opt(
        "UPDATE scheduled_tasks
         SET status = 'running',
             version = version + 1
         WHERE id = $1 AND version = $2 AND status = 'idle'
         RETURNING *",
        &[&task_id, &version]
    ).await?;
    
    Ok(result)
}
```

**Update (Complete Task)**:
```rust
async fn complete_task(
    &self,
    task_id: i32,
    schedule_type: &str,
) -> Result<()> {
    let now = Utc::now();
    let next_run = calculate_next_run(schedule_type, now);
    
    self.client.execute(
        "UPDATE scheduled_tasks
         SET status = 'idle',
             last_run_at = $2,
             next_run_at = $3,
             retry_count = 0,
             error_message = NULL
         WHERE id = $1",
        &[&task_id, &now, &next_run]
    ).await?;
    
    Ok(())
}
```

**Delete**:
```rust
async fn delete_scheduled_task(&self, repo_id: i32) -> Result<()> {
    self.client.execute(
        "DELETE FROM scheduled_tasks WHERE repository_id = $1",
        &[&repo_id]
    ).await?;
    
    Ok(())
}
```

---

## Time Handling

**Timezone Policy**: All timestamps are stored and processed in UTC.

**Clock Source**: PostgreSQL database (`SELECT NOW() AT TIME ZONE 'UTC'`) to avoid clock skew.

**Schedule Calculation**:
```rust
fn calculate_next_run(schedule_type: &str, last_run: DateTime<Utc>) -> DateTime<Utc> {
    match schedule_type {
        "hourly" => last_run + Duration::hours(1),
        "daily" => (last_run + Duration::days(1))
            .with_hour(0).unwrap()
            .with_minute(0).unwrap(),
        "weekly" => (last_run + Duration::weeks(1))
            .with_weekday(Weekday::Mon).unwrap()
            .with_hour(0).unwrap()
            .with_minute(0).unwrap(),
        _ => panic!("Invalid schedule_type"),
    }
}
```

---

## Concurrency Control

**Optimistic Locking**: The `version` field prevents duplicate execution when multiple orchestrators claim the same task.

**Lock Acquisition**:
```sql
UPDATE scheduled_tasks
SET status = 'running', version = version + 1
WHERE id = $1 AND version = $2 AND status = 'idle'
```

**Guarantee**: Only one orchestrator can successfully claim a task (atomic check-and-set).

---

## Error Recovery

**Failed Task Handling**:
```rust
async fn handle_failure(
    &self,
    task_id: i32,
    error: &Error,
) -> Result<()> {
    let retry_count = self.increment_retry(&task_id).await?;
    
    if retry_count >= 5 {
        // Move to dead letter queue
        self.move_to_failed_tasks(task_id, error).await?;
    } else {
        // Schedule retry with exponential backoff
        let backoff = Duration::minutes(2_i64.pow(retry_count as u32));
        self.schedule_retry(task_id, backoff, error).await?;
    }
    
    Ok(())
}
```

**Stale Task Detection**: Tasks stuck in 'running' state for >1 hour are automatically reset to 'idle'.

---

## Migration Path

**Schema Extensions**: The following fields need to be added to the existing `scheduled_tasks` table:

```sql
ALTER TABLE scheduled_tasks
ADD COLUMN status VARCHAR(20) DEFAULT 'idle',
ADD COLUMN version INTEGER DEFAULT 1,
ADD COLUMN retry_count INTEGER DEFAULT 0,
ADD COLUMN error_message TEXT;

CREATE INDEX idx_tasks_next_run 
ON scheduled_tasks(next_run_at) 
WHERE status = 'idle';

CREATE INDEX idx_tasks_status 
ON scheduled_tasks(status);
```

**Backwards Compatibility**: Existing rows will get default values for new fields. No data migration needed.

---

## Summary

**Entities**:
1. **ScheduledTask** (database): Scheduled indexing jobs with retry logic
2. **PidLock** (filesystem): Daemon process management

**Key Properties**:
- UTC-only time handling (DST-agnostic)
- Optimistic locking for concurrency control
- Exponential backoff for retries
- Cascade deletion with repositories
- One schedule per repository (UNIQUE constraint)

**No New Tables Required**: Feature uses existing `scheduled_tasks` table with schema extensions.
