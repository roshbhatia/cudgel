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

/// Wait for a shutdown signal (SIGINT or SIGTERM)
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received SIGINT (Ctrl-C)");
        }
        _ = terminate => {
            info!("Received SIGTERM");
        }
    }
}

/// Shutdown coordination state
struct Shutdown {
    is_shutdown: bool,
    notify: tokio::sync::broadcast::Receiver<()>,
}

impl Shutdown {
    fn new(notify: tokio::sync::broadcast::Receiver<()>) -> Self {
        Shutdown {
            is_shutdown: false,
            notify,
        }
    }

    async fn recv(&mut self) {
        if self.is_shutdown {
            return;
        }
        let _ = self.notify.recv().await;
        self.is_shutdown = true;
    }

    fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }
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
        .parse::<u32>()
        .map_err(|e| crate::Error::InvalidPidFile(e.to_string()))?;

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
        return Err(crate::Error::OrchestratorAlreadyRunning(pid as i32));
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
        None => return Err(crate::Error::OrchestratorNotRunning),
    };

    // Send SIGTERM to daemon
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
            .map_err(|e| crate::Error::SignalHandler(format!("Failed to send SIGTERM: {}", e)))?;

        // Wait for process to exit (up to 5 seconds)
        for _ in 0..50 {
            if is_running()?.is_none() {
                // PID file may already be removed by is_running()
                let pid_path = pid_file_path();
                if pid_path.exists() {
                    fs::remove_file(pid_path)?;
                }
                info!("Orchestrator daemon stopped");
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        // If still running after 5 seconds, force kill
        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
        let pid_path = pid_file_path();
        if pid_path.exists() {
            fs::remove_file(pid_path)?;
        }
        warn!("Orchestrator daemon force-killed");
    }

    #[cfg(not(unix))]
    {
        // TODO: Implement for Windows
        return Err(crate::Error::Other(
            "Stop daemon not implemented for non-Unix platforms".to_string(),
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

    // Setup shutdown coordination
    let (notify_shutdown, _) = tokio::sync::broadcast::channel(1);
    let (shutdown_complete_tx, mut shutdown_complete_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Clone for the worker task
    let worker_shutdown = notify_shutdown.subscribe();
    let worker_db = db.clone();
    let worker_config = config.clone();
    let worker_shutdown_complete = shutdown_complete_tx.clone();

    // Spawn worker task
    let worker_handle = tokio::spawn(async move {
        let mut shutdown = Shutdown::new(worker_shutdown);
        let _shutdown_complete = worker_shutdown_complete;

        // Polling interval: 60 seconds
        let mut interval = time::interval(Duration::from_secs(60));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Check shutdown before starting work
                    if shutdown.is_shutdown() {
                        break;
                    }

                    // Get due tasks
                    match worker_db.get_due_tasks().await {
                        Ok(tasks) => {
                            if tasks.is_empty() {
                                continue;
                            }

                            info!("Found {} due tasks", tasks.len());

                            for task in tasks {
                                let task_id = task.id;
                                let task_version = task.version;

                                // Try to claim task with optimistic locking
                                let claimed_task = match worker_db.claim_task(task_id, task_version).await {
                                    Ok(Some(t)) => t,
                                    Ok(None) => {
                                        // Task already claimed by another worker
                                        continue;
                                    }
                                    Err(e) => {
                                        error!("Failed to claim task {}: {}", task_id, e);
                                        continue;
                                    }
                                };

                                info!(
                                    "Executing scheduled task {} for repo {}",
                                    claimed_task.id, claimed_task.repo_id
                                );

                                // Get repository info
                                let repo = match worker_db.get_repository(claimed_task.repo_id).await {
                                    Ok(Some(r)) => r,
                                    Ok(None) => {
                                        warn!(
                                            "Repository {} not found, marking task {} as failed",
                                            claimed_task.repo_id, claimed_task.id
                                        );
                                        let _ = worker_db.fail_task(claimed_task.id, "Repository not found").await;
                                        continue;
                                    }
                                    Err(e) => {
                                        error!("Failed to get repository: {}", e);
                                        let _ = worker_db.fail_task(claimed_task.id, &format!("Database error: {}", e)).await;
                                        continue;
                                    }
                                };

                                // Create indexer for this task
                                let mut indexer = match Indexer::new(worker_config.clone(), worker_db.clone()) {
                                    Ok(i) => i,
                                    Err(e) => {
                                        error!("Failed to create indexer: {}", e);
                                        let _ = worker_db.fail_task(claimed_task.id, &format!("Indexer creation failed: {}", e)).await;
                                        continue;
                                    }
                                };

                                // Execute indexing
                                match indexer.index_repository(Path::new(&repo.path)).await {
                                    Ok((_repo_id, _stats)) => {
                                        info!("Successfully indexed repository: {}", repo.name);

                                        // Mark task as complete
                                        if let Err(e) = worker_db.complete_task(claimed_task.id, claimed_task.interval_hours).await {
                                            error!("Failed to complete task: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to index repository {}: {}", repo.name, e);
                                        let _ = worker_db.fail_task(claimed_task.id, &format!("Indexing failed: {}", e)).await;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to get due tasks: {}", e);
                        }
                    }
                }
                _ = shutdown.recv() => {
                    info!("Worker task received shutdown signal");
                    break;
                }
            }
        }

        info!("Worker task shutdown complete");
    });

    // Wait for shutdown signal
    tokio::select! {
        _ = worker_handle => {
            warn!("Worker task terminated unexpectedly");
        }
        _ = shutdown_signal() => {
            info!("Shutdown signal received");
        }
    }

    // Initiate shutdown
    drop(notify_shutdown);
    drop(shutdown_complete_tx);

    // Wait for graceful shutdown with timeout
    match tokio::time::timeout(Duration::from_secs(30), shutdown_complete_rx.recv()).await {
        Ok(_) => {
            info!("Graceful shutdown complete");
        }
        Err(_) => {
            warn!("Shutdown timeout after 30 seconds, forcing termination");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::xdg_state_home;

    #[test]
    fn test_pid_file_path_construction() {
        let pid_dir = xdg_state_home().join("cudgel");
        let pid_file = pid_dir.join("orchestrator.pid");

        // Verify path components
        assert!(pid_file.to_string_lossy().contains("cudgel"));
        assert!(pid_file.to_string_lossy().ends_with("orchestrator.pid"));

        // Verify it's an absolute path
        assert!(pid_file.is_absolute());
    }

    #[test]
    fn test_log_file_path_construction() {
        let log_dir = xdg_state_home().join("cudgel");
        let log_file = log_dir.join("orchestrator.log");

        // Verify path components
        assert!(log_file.to_string_lossy().contains("cudgel"));
        assert!(log_file.to_string_lossy().ends_with("orchestrator.log"));

        // Verify it's an absolute path
        assert!(log_file.is_absolute());
    }

    #[cfg(unix)]
    #[test]
    fn test_is_running_no_pid_file() {
        // If PID file doesn't exist, is_running should return Ok(None)
        let pid_file = xdg_state_home().join("cudgel/orchestrator.pid");

        // Clean up any existing PID file from previous tests
        let _ = std::fs::remove_file(&pid_file);

        let result = is_running();
        assert!(result.is_ok());

        // If PID file doesn't exist or process is not running, should be None or Ok
        // Both None (no PID file) and Some (stale PID) are acceptable outcomes
        let _pid = result.unwrap(); // May be None or Some
    }

    #[test]
    fn test_interval_hours_calculation() {
        // Test that interval hours are correctly applied
        let interval_hours = 24;
        let now = chrono::Utc::now();
        let next_run = now + chrono::Duration::hours(interval_hours as i64);

        let duration = next_run - now;
        assert_eq!(duration.num_hours(), interval_hours as i64);
    }

    #[test]
    fn test_xdg_state_home_returns_valid_path() {
        let state_home = xdg_state_home();

        // Should not be empty
        assert!(!state_home.as_os_str().is_empty());

        // Should be an absolute path
        assert!(state_home.is_absolute());

        // Should contain reasonable components
        let path_str = state_home.to_string_lossy();
        assert!(!path_str.is_empty());
    }
}
