//! Orchestrator daemon for scheduled indexing tasks
//!
//! The orchestrator runs as a background daemon, periodically checking for scheduled
//! indexing tasks and executing them at the appropriate times.

use crate::{Config, Result};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

/// Get the path to the orchestrator PID file
fn pid_file_path() -> PathBuf {
    crate::config::xdg_state_home().join("cudgel/orchestrator.pid")
}

/// Get the path to the orchestrator log file
fn log_file_path() -> PathBuf {
    crate::config::xdg_state_home().join("cudgel/orchestrator.log")
}

/// Check if the orchestrator daemon is running
pub fn is_running() -> Result<Option<u32>> {
    let pid_path = pid_file_path();

    if !pid_path.exists() {
        return Ok(None);
    }

    let pid_str = fs::read_to_string(&pid_path)?;
    let pid: u32 = pid_str
        .trim()
        .parse()
        .map_err(|e| crate::Error::Other(format!("Invalid PID file: {}", e)))?;

    // Check if process is actually running
    #[cfg(unix)]
    {
        use nix::sys::signal::kill;
        use nix::unistd::Pid;

        match kill(Pid::from_raw(pid as i32), None) {
            Ok(_) => Ok(Some(pid)), // Process exists
            Err(_) => {
                // PID file exists but process doesn't - clean up stale PID file
                let _ = fs::remove_file(&pid_path);
                Ok(None)
            }
        }
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, just return the PID from the file
        // TODO: Implement proper process checking for Windows
        Ok(Some(pid))
    }
}

/// Start the orchestrator daemon
pub fn start_daemon(_config: &Config) -> Result<()> {
    // Check if already running
    if let Some(pid) = is_running()? {
        return Err(crate::Error::Other(format!(
            "Orchestrator is already running (PID: {})",
            pid
        )));
    }

    // Ensure directories exist
    if let Some(parent) = pid_file_path().parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = log_file_path().parent() {
        fs::create_dir_all(parent)?;
    }

    // Spawn daemon process
    let exe = std::env::current_exe()?;
    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path())?;

    let child = Command::new(exe)
        .arg("orchestrator")
        .arg("run-daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file))
        .spawn()?;

    // Write PID file
    fs::write(pid_file_path(), child.id().to_string())?;

    info!("Orchestrator daemon started with PID: {}", child.id());
    Ok(())
}

/// Stop the orchestrator daemon
pub fn stop_daemon() -> Result<()> {
    let pid = match is_running()? {
        Some(pid) => pid,
        None => {
            return Err(crate::Error::Other(
                "Orchestrator is not running".to_string(),
            ))
        }
    };

    // Send SIGTERM to daemon
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        kill(Pid::from_raw(pid as i32), Signal::SIGTERM).map_err(|e| {
            crate::Error::Other(format!("Failed to stop orchestrator: {}", e))
        })?;

        // Wait for process to exit (up to 5 seconds)
        for _ in 0..50 {
            if is_running()?.is_none() {
                fs::remove_file(pid_file_path())?;
                info!("Orchestrator daemon stopped");
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        // If still running after 5 seconds, force kill
        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
        fs::remove_file(pid_file_path())?;
        warn!("Orchestrator daemon force-killed");
    }

    #[cfg(not(unix))]
    {
        // TODO: Implement for Windows
        return Err(crate::Error::Other(
            "Stop daemon not implemented for this platform".to_string(),
        ));
    }

    Ok(())
}

/// Restart the orchestrator daemon
pub fn restart_daemon(config: &Config) -> Result<()> {
    if is_running()?.is_some() {
        stop_daemon()?;
    }
    start_daemon(config)
}

/// Run the orchestrator polling loop (called by daemon process)
pub async fn run_polling_loop(config: Config) -> Result<()> {
    use crate::database::Database;
    use crate::indexer::Indexer;
    use std::path::Path;
    use std::sync::Arc;

    info!("Orchestrator daemon starting polling loop");

    let config = Arc::new(config);
    let db = Arc::new(Database::new(&config).await?);

    // Polling interval: 60 seconds
    let mut interval = time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        // Get due tasks
        match db.get_due_tasks().await {
            Ok(tasks) => {
                if tasks.is_empty() {
                    continue;
                }

                info!("Found {} due tasks", tasks.len());

                for task in tasks {
                    info!(
                        "Executing scheduled task {} for repo {}",
                        task.id, task.repo_id
                    );

                    // Get repository info
                    let repo = match db.get_repository(task.repo_id).await? {
                        Some(r) => r,
                        None => {
                            warn!(
                                "Repository {} not found, skipping task {}",
                                task.repo_id, task.id
                            );
                            continue;
                        }
                    };

                    // Create indexer for this task
                    let mut indexer = match Indexer::new(config.clone(), db.clone()) {
                        Ok(i) => i,
                        Err(e) => {
                            error!("Failed to create indexer: {}", e);
                            continue;
                        }
                    };

                    // Execute indexing
                    match indexer.index_repository(Path::new(&repo.path)).await {
                        Ok((_repo_id, _stats)) => {
                            info!("Successfully indexed repository: {}", repo.name);

                            // Update task execution times
                            let now = chrono::Utc::now().naive_utc();
                            let next_run =
                                now + chrono::Duration::hours(task.interval_hours as i64);

                            if let Err(e) = db.update_task_execution(task.id, now, next_run).await {
                                error!("Failed to update task execution: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to index repository {}: {}", repo.name, e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to get due tasks: {}", e);
            }
        }
    }
}
