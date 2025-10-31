//! macOS LaunchAgent-based service management
//!
//! Manages PostgreSQL and Temporal as persistent macOS services using launchctl.
//! One-time setup creates Launch Agents that auto-start on login.

use crate::Result;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const POSTGRES_PLIST: &str = "com.cudgel.postgres.plist";
const TEMPORAL_PLIST: &str = "com.cudgel.temporal.plist";

pub struct MacOSServices {
    launch_agents_dir: PathBuf,
}

impl MacOSServices {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/unknown".to_string());
        let launch_agents_dir = PathBuf::from(home).join("Library/LaunchAgents");

        MacOSServices { launch_agents_dir }
    }

    /// One-time setup: install Homebrew services and create Launch Agents
    pub async fn setup(&self) -> Result<()> {
        println!(" Setting up cudgel services...");

        // Ensure Launch Agents directory exists
        fs::create_dir_all(&self.launch_agents_dir)?;

        // Check if Homebrew is installed
        if !self.is_homebrew_installed() {
            return Err(crate::Error::Other(
                "Homebrew is not installed. Please install Homebrew first: https://brew.sh".to_string()
            ));
        }

        // Install PostgreSQL via Homebrew if not installed
        self.ensure_postgres_installed().await?;

        // Setup PostgreSQL Launch Agent
        self.setup_postgres_agent().await?;

        // Setup Temporal Launch Agent (Docker-based)
        self.setup_temporal_agent().await?;

        // Initialize database
        self.init_database().await?;

        println!(" Services setup complete!");
        println!("\nServices will auto-start on login.");
        println!("You can also manage them with:");
        println!("  cudgel services start");
        println!("  cudgel services stop");
        println!("  cudgel services status");

        Ok(())
    }

    fn is_homebrew_installed(&self) -> bool {
        Command::new("brew")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn ensure_postgres_installed(&self) -> Result<()> {
        println!(" Checking PostgreSQL installation...");

        let output = Command::new("brew")
            .args(["list", "postgresql@16"])
            .output()?;

        if !output.status.success() {
            println!(" Installing PostgreSQL 16 via Homebrew...");
            let install = Command::new("brew")
                .args(["install", "postgresql@16"])
                .status()?;

            if !install.success() {
                return Err(crate::Error::Other(
                    "Failed to install PostgreSQL via Homebrew".to_string()
                ));
            }
        }

        println!(" PostgreSQL 16 installed");
        Ok(())
    }

    async fn setup_postgres_agent(&self) -> Result<()> {
        println!(" Creating PostgreSQL Launch Agent...");

        let plist_path = self.launch_agents_dir.join(POSTGRES_PLIST);

        // Get Homebrew prefix
        let brew_prefix = self.get_brew_prefix()?;
        let postgres_bin = format!("{}/opt/postgresql@16/bin/postgres", brew_prefix);
        let data_dir = format!("{}/var/postgresql@16", brew_prefix);

        let plist_content = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.cudgel.postgres</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>-D</string>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/cudgel-postgres.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/cudgel-postgres-error.log</string>
</dict>
</plist>"#, postgres_bin, data_dir);

        fs::write(&plist_path, plist_content)?;

        // Initialize database if needed
        if !PathBuf::from(&data_dir).exists() {
            println!("Initializing PostgreSQL database...");
            let initdb = format!("{}/opt/postgresql@16/bin/initdb", brew_prefix);
            Command::new(initdb)
                .arg("-D")
                .arg(&data_dir)
                .status()?;
        }

        // Load the service
        Command::new("launchctl")
            .args(["load", plist_path.to_str().unwrap()])
            .status()?;

        println!(" PostgreSQL Launch Agent created and loaded");
        Ok(())
    }

    async fn setup_temporal_agent(&self) -> Result<()> {
        println!(" Creating Temporal Launch Agent...");

        let plist_path = self.launch_agents_dir.join(TEMPORAL_PLIST);

        let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.cudgel.temporal</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/docker</string>
        <string>run</string>
        <string>--rm</string>
        <string>--name</string>
        <string>cudgel-temporal</string>
        <string>-p</string>
        <string>7233:7233</string>
        <string>temporalio/auto-setup:latest</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/cudgel-temporal.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/cudgel-temporal-error.log</string>
</dict>
</plist>"#;

        fs::write(&plist_path, plist_content)?;

        // Pull Temporal image first
        println!(" Pulling Temporal Docker image...");
        Command::new("docker")
            .args(["pull", "temporalio/auto-setup:latest"])
            .status()?;

        // Load the service
        Command::new("launchctl")
            .args(["load", plist_path.to_str().unwrap()])
            .status()?;

        println!(" Temporal Launch Agent created and loaded");
        Ok(())
    }

    async fn init_database(&self) -> Result<()> {
        println!("Initializing cudgel database...");

        // Wait for postgres to start
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        let brew_prefix = self.get_brew_prefix()?;
        let psql = format!("{}/opt/postgresql@16/bin/psql", brew_prefix);

        // Create user
        let _ = Command::new(&psql)
            .args(["postgres", "-c", "CREATE USER cudgel WITH PASSWORD 'cudgel';"])
            .output();

        // Create database
        let _ = Command::new(&psql)
            .args(["postgres", "-c", "CREATE DATABASE cudgel OWNER cudgel;"])
            .output();

        // Create pgvector extension
        let _ = Command::new(&psql)
            .args(["cudgel", "-c", "CREATE EXTENSION IF NOT EXISTS vector;"])
            .output();

        println!(" Database initialized");
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        println!("  Starting services...");

        let postgres_plist = self.launch_agents_dir.join(POSTGRES_PLIST);
        let temporal_plist = self.launch_agents_dir.join(TEMPORAL_PLIST);

        Command::new("launchctl")
            .args(["load", postgres_plist.to_str().unwrap()])
            .status()?;

        Command::new("launchctl")
            .args(["load", temporal_plist.to_str().unwrap()])
            .status()?;

        println!(" Services started");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        println!("  Stopping services...");

        let postgres_plist = self.launch_agents_dir.join(POSTGRES_PLIST);
        let temporal_plist = self.launch_agents_dir.join(TEMPORAL_PLIST);

        Command::new("launchctl")
            .args(["unload", postgres_plist.to_str().unwrap()])
            .status()?;

        Command::new("launchctl")
            .args(["unload", temporal_plist.to_str().unwrap()])
            .status()?;

        println!(" Services stopped");
        Ok(())
    }

    pub async fn status(&self) -> Result<String> {
        let mut status = String::new();

        // Check postgres
        let pg_status = Command::new("launchctl")
            .args(["list", "com.cudgel.postgres"])
            .output()?;

        status.push_str("PostgreSQL: ");
        status.push_str(if pg_status.status.success() {
            "running\n"
        } else {
            "stopped\n"
        });

        // Check temporal
        let temporal_status = Command::new("launchctl")
            .args(["list", "com.cudgel.temporal"])
            .output()?;

        status.push_str("Temporal:   ");
        status.push_str(if temporal_status.status.success() {
            "running\n"
        } else {
            "stopped\n"
        });

        Ok(status)
    }

    pub async fn is_running(&self) -> Result<bool> {
        let pg_status = Command::new("launchctl")
            .args(["list", "com.cudgel.postgres"])
            .output()?;

        Ok(pg_status.status.success())
    }

    pub async fn ensure_running(&self) -> Result<()> {
        if !self.is_running().await? {
            println!("Services not running. Please run 'cudgel setup' first.");
            return Err(crate::Error::Other(
                "Services not set up. Run 'cudgel setup' to initialize.".to_string()
            ));
        }
        Ok(())
    }

    fn get_brew_prefix(&self) -> Result<String> {
        let output = Command::new("brew")
            .args(["--prefix"])
            .output()?;

        if !output.status.success() {
            return Err(crate::Error::Other("Failed to get Homebrew prefix".to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub async fn remove(&self) -> Result<()> {
        println!("Removing cudgel services...");

        // Stop services first
        self.stop().await?;

        // Remove plist files
        let postgres_plist = self.launch_agents_dir.join(POSTGRES_PLIST);
        let temporal_plist = self.launch_agents_dir.join(TEMPORAL_PLIST);

        let _ = fs::remove_file(postgres_plist);
        let _ = fs::remove_file(temporal_plist);

        println!(" Services removed");
        println!("\nNote: PostgreSQL data is preserved in Homebrew.");
        println!("To completely remove PostgreSQL: brew uninstall postgresql@16");

        Ok(())
    }
}

impl Default for MacOSServices {
    fn default() -> Self {
        Self::new()
    }
}
