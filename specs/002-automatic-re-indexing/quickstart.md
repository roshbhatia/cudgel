# Quickstart: Automatic Re-indexing

**Feature**: 002-automatic-re-indexing  
**Date**: 2025-11-19

## Overview

Automatically keep your code indexes up-to-date with scheduled re-indexing. The orchestrator daemon runs in the background and executes incremental indexing at your specified intervals.

---

## Prerequisites

1. **PostgreSQL running** on port 54321
   ```bash
   # Start PostgreSQL (if not running)
   ./scripts/start-postgres.sh
   ```

2. **Repository already indexed**
   ```bash
   cudgel index /path/to/repo
   ```

---

## Quick Start (5 minutes)

### 1. Schedule Automatic Indexing

```bash
# Schedule hourly indexing for a repository
cudgel index --schedule hourly ~/projects/myapp

# Output:
# ✓ Scheduled hourly indexing for /Users/user/projects/myapp
#   Next run: 2025-11-19 16:00:00 UTC
#   Orchestrator status: running (PID 12345)
```

### 2. Check Status

```bash
cudgel orchestrator status

# Output:
# Orchestrator Status: RUNNING (PID 12345)
#   Started: 2025-11-19 15:00:00 UTC
#   Uptime: 1h 5m
#
# Scheduled Tasks (1):
#   1. /Users/user/projects/myapp
#      Frequency: hourly
#      Last run: 2025-11-19 15:00:00 UTC (1h ago)
#      Next run: 2025-11-19 16:00:00 UTC (in 5m)
#      Status: ✓ Success
```

### 3. Verify Automatic Execution

Wait for the next scheduled time (or make code changes and wait):

```bash
# After 1 hour, check status again
cudgel orchestrator status

# You should see:
#   Last run: 2025-11-19 16:00:00 UTC (just now)
#   Next run: 2025-11-19 17:00:00 UTC (in 1h)
```

### 4. Remove Schedule (Optional)

```bash
cudgel index --unschedule ~/projects/myapp

# Output:
# ✓ Removed schedule for /Users/user/projects/myapp
```

---

## Common Workflows

### Schedule Multiple Repositories

```bash
# Hourly for active development
cudgel index --schedule hourly ~/projects/frontend

# Daily for dependencies
cudgel index --schedule daily ~/projects/backend

# Weekly for archived projects
cudgel index --schedule weekly ~/projects/legacy
```

### Check What's Running

```bash
cudgel orchestrator status
```

### View Orchestrator Logs

```bash
tail -f ~/.local/state/cudgel/orchestrator.log
```

### Restart Orchestrator

```bash
# Useful after configuration changes
cudgel orchestrator restart
```

---

## Schedule Types

| Type | Frequency | Use Case |
|------|-----------|----------|
| `hourly` | Every 60 minutes | Active development repos |
| `daily` | Every day at midnight UTC | Stable projects, dependencies |
| `weekly` | Every Monday at midnight UTC | Archived/legacy projects |

---

## Orchestrator Commands

### Start
```bash
cudgel orchestrator start
```
- Starts daemon in background
- Creates PID file at `~/.local/var/run/cudgel/orchestrator.pid`
- Logs to `~/.local/state/cudgel/orchestrator.log`

### Stop
```bash
cudgel orchestrator stop
```
- Graceful shutdown (finishes current task)
- Removes PID file
- 30-second timeout before forced termination

### Restart
```bash
cudgel orchestrator restart
```
- Equivalent to `stop` + `start`
- New PID assigned

### Status
```bash
cudgel orchestrator status
```
- Shows orchestrator state (running/stopped)
- Lists all scheduled tasks with next run times
- Shows last execution status

---

## Troubleshooting

### Orchestrator Won't Start

**Error**: `Error: Orchestrator already running (PID 12345)`

**Solution**: Check if actually running:
```bash
cudgel orchestrator status
```

If stale, manually remove PID file:
```bash
rm ~/.local/var/run/cudgel/orchestrator.pid
cudgel orchestrator start
```

---

### Repository Not Found

**Error**: `Error: Repository not indexed. Run 'cudgel index <path>' first`

**Solution**: Index the repository first:
```bash
cudgel index /path/to/repo
cudgel index --schedule hourly /path/to/repo
```

---

### Already Scheduled

**Error**: `Error: Repository already scheduled (hourly). Use --unschedule first`

**Solution**: Remove existing schedule and re-schedule:
```bash
cudgel index --unschedule /path/to/repo
cudgel index --schedule daily /path/to/repo
```

---

### Database Connection Failed

**Error**: `Error: Database connection failed. Is PostgreSQL running on port 54321?`

**Solution**: Start PostgreSQL:
```bash
./scripts/start-postgres.sh

# Verify it's running
psql -p 54321 -d cudgel -c "SELECT 1"
```

---

### Task Not Executing

**Check 1**: Is orchestrator running?
```bash
cudgel orchestrator status
# Should show "RUNNING"
```

**Check 2**: Has next run time passed?
```bash
cudgel orchestrator status
# Check "Next run" timestamp
```

**Check 3**: Check logs for errors:
```bash
tail -50 ~/.local/state/cudgel/orchestrator.log
```

---

## Testing the Feature

### Manual Test Workflow

```bash
# 1. Index a small test repository
mkdir -p /tmp/test-repo
cd /tmp/test-repo
echo "fn main() {}" > main.rs
cudgel index .

# 2. Schedule hourly indexing
cudgel index --schedule hourly .

# 3. Check orchestrator status
cudgel orchestrator status
# Should show task scheduled with next run time

# 4. Make a code change
echo "fn helper() {}" >> main.rs

# 5. Wait for next run (or manually trigger re-index)
# In production, wait 1 hour. For testing, modify next_run_at in database:
psql -p 54321 -d cudgel -c \
  "UPDATE scheduled_tasks SET next_run_at = NOW() - INTERVAL '1 second' WHERE repository_id = (SELECT id FROM repositories WHERE path = '/tmp/test-repo')"

# 6. Wait 60 seconds for orchestrator to poll
sleep 60

# 7. Verify execution
cudgel orchestrator status
# Should show "Last run: [recent timestamp]"

# 8. Cleanup
cudgel index --unschedule /tmp/test-repo
```

---

## Best Practices

1. **Start with hourly** for active development repositories
2. **Use daily** for dependencies and stable projects
3. **Use weekly** for archived or rarely-changed code
4. **Monitor logs** (`~/.local/state/cudgel/orchestrator.log`) initially
5. **One schedule per repo** (constraint enforced by database)
6. **Orchestrator runs continuously** (survives terminal close)

---

## Advanced Usage

### Custom PID/Log Locations

```bash
export CUDGEL_PID_DIR=/custom/path/pid
export CUDGEL_LOG_DIR=/custom/path/logs
cudgel orchestrator start
```

### Integration with System Startup

**macOS (launchd)**:
Create `~/Library/LaunchAgents/com.cudgel.orchestrator.plist`:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.cudgel.orchestrator</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/cudgel</string>
        <string>orchestrator</string>
        <string>start</string>
    </array>
    <key>KeepAlive</key>
    <true/>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
```

```bash
launchctl load ~/Library/LaunchAgents/com.cudgel.orchestrator.plist
```

**Linux (systemd)**:
Create `/etc/systemd/system/cudgel-orchestrator.service`:
```ini
[Unit]
Description=Cudgel Orchestrator Daemon
After=postgresql.service

[Service]
Type=simple
ExecStart=/usr/local/bin/cudgel orchestrator start
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable cudgel-orchestrator
sudo systemctl start cudgel-orchestrator
```

---

## Performance Characteristics

- **Memory**: <50MB RAM when idle
- **CPU**: Minimal (only active during 60s polls and indexing)
- **Disk**: Log file grows ~1MB/day (automatic rotation at 10MB)
- **Network**: None (local-only)

---

## What's Next?

- **Phase 2**: See `tasks.md` for implementation breakdown
- **Testing**: See `tests/integration_tests.rs` after implementation
- **Monitoring**: Use `cudgel orchestrator status` and log files

---

## Summary

1. **Schedule**: `cudgel index --schedule <frequency> <path>`
2. **Check**: `cudgel orchestrator status`
3. **Remove**: `cudgel index --unschedule <path>`
4. **Logs**: `~/.local/state/cudgel/orchestrator.log`
5. **Control**: `start`, `stop`, `restart`, `status`

The orchestrator runs continuously in the background, executing incremental indexing at your specified intervals. No manual intervention required after setup.
