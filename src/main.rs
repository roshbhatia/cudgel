//! Cudgel CLI - Code indexing tool

use clap::{Parser, Subcommand};
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};
use cudgel::{
    config::Config, database::Database, graph::GraphQuery, indexer::Indexer, query::QueryEngine,
};
use std::path::PathBuf;
use std::sync::Arc;
// use syntect::{
//     easy::HighlightLines,
//     highlighting::{Style, ThemeSet},
//     parsing::SyntaxSet,
//     util::{as_24_bit_terminal_escaped, LinesWithEndings},
// };
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(name = "cudgel")]
#[command(version = "0.1.0")]
#[command(about = "A code indexing tool with tree-sitter and PostgreSQL/pgvector", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Index a code repository
    Index {
        /// Paths to index (supports glob patterns like ./**/*.rs)
        /// Use ./... for recursive indexing (Go-style)
        #[arg(required = true)]
        paths: Vec<String>,

        /// Repository name (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,

        /// Include only files matching these patterns (comma-separated globs)
        /// Example: --include "*.go,*.rs,*.py"
        #[arg(long, value_delimiter = ',')]
        include: Option<Vec<String>>,

        /// Exclude files matching these patterns (comma-separated globs)
        /// Example: --exclude "*.test.js,*_test.go"
        #[arg(long, value_delimiter = ',')]
        exclude: Option<Vec<String>>,

        /// Index only these languages (comma-separated)
        /// Supported: rust, python, javascript, typescript, go, c, cpp, java
        #[arg(short, long, value_delimiter = ',')]
        languages: Option<Vec<String>>,

        /// Dry run - show what would be indexed without actually indexing
        #[arg(long)]
        dry_run: bool,

        /// Schedule automatic re-indexing (hourly, daily, weekly, or hours as integer)
        /// Example: --schedule hourly or --schedule 6 (for every 6 hours)
        #[arg(long, conflicts_with = "unschedule")]
        schedule: Option<String>,

        /// Remove scheduled indexing for this repository
        #[arg(long, conflicts_with = "schedule")]
        unschedule: bool,
    },

    /// Query code using natural language
    Query {
        /// Natural language query
        query: String,

        /// Filter by repository path
        #[arg(short, long)]
        repo: Option<String>,

        /// Maximum number of results (1-1000)
        #[arg(short, long, default_value = "10", value_parser = validate_limit)]
        limit: i64,

        /// Output as compact JSON (single line)
        #[arg(short, long, conflicts_with_all = ["json_pretty", "minified"])]
        json: bool,

        /// Output as pretty-printed JSON (indented)
        #[arg(long, conflicts_with_all = ["json", "minified"])]
        json_pretty: bool,

        /// Output as minified JSON (optimized for LLMs)
        #[arg(short = 'm', long, conflicts_with_all = ["json", "json_pretty"])]
        minified: bool,
    },

    /// Show graph relationships for a symbol
    Graph {
        /// Symbol name
        symbol: String,

        /// Filter by repository path
        #[arg(short, long)]
        repo: Option<String>,

        /// Traversal depth (1-10)
        #[arg(short, long, default_value = "1", value_parser = validate_depth)]
        depth: usize,

        /// Graph type (references or calls)
        #[arg(short = 't', long, default_value = "references")]
        graph_type: String,

        /// Direction (incoming, outgoing, or both)
        #[arg(long, default_value = "both")]
        direction: String,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Initialize the database schema
    InitDb {
        /// Reset (drop and recreate) all tables - WARNING: Deletes all data
        #[arg(long)]
        reset: bool,
    },

    /// Manage the orchestrator daemon for scheduled indexing
    #[command(subcommand)]
    Orchestrator(OrchestratorCommand),

    /// Manage dependencies (model, database, schema)
    Deps {
        /// Check dependency status without installing
        #[arg(long)]
        check: bool,

        /// Clean downloaded models and temporary files
        #[arg(long, conflicts_with = "check")]
        clean_models: bool,

        /// Clean all data including database (WARNING: deletes all data)
        #[arg(long, conflicts_with_all = ["check", "clean_models"])]
        clean_all: bool,

        /// Show verbose diagnostic information
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Subcommand)]
enum OrchestratorCommand {
    /// Start the orchestrator daemon
    Start,

    /// Stop the orchestrator daemon
    Stop,

    /// Check orchestrator daemon status
    Status,

    /// Restart the orchestrator daemon
    Restart,

    /// Run the daemon (internal use only)
    #[command(hide = true)]
    RunDaemon,
}

/// Validate limit parameter is in range [1, 1000]
fn validate_limit(s: &str) -> Result<i64, String> {
    let value: i64 = s
        .parse()
        .map_err(|_| format!("'{}' is not a valid number", s))?;
    if value < 1 {
        Err("limit must be at least 1".to_string())
    } else if value > 1000 {
        Err("limit cannot exceed 1000".to_string())
    } else {
        Ok(value)
    }
}

/// Validate depth parameter is in range [1, 10]
fn validate_depth(s: &str) -> Result<usize, String> {
    let value: usize = s
        .parse()
        .map_err(|_| format!("'{}' is not a valid number", s))?;
    if value < 1 {
        Err("depth must be at least 1".to_string())
    } else if value > 10 {
        Err("depth cannot exceed 10".to_string())
    } else {
        Ok(value)
    }
}

#[tokio::main]
async fn main() -> cudgel::Result<()> {
    // Check if debug mode is enabled
    let debug_mode = std::env::var("CUDGEL_DEBUG")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    // Initialize tracing with appropriate log levels
    let env_filter = if debug_mode {
        // Debug mode: show all INFO logs including ONNX Runtime
        EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into())
    } else {
        // Normal mode: suppress ONNX Runtime logs, show INFO for cudgel
        EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into())
            .add_directive("ort=error".parse().unwrap())
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(env_filter)
        .init();

    let cli = Cli::parse();

    // Load and validate configuration
    let config = Config::local().inspect_err(|e| {
        eprintln!("{}", "Configuration Error:".bright_red().bold());
        eprintln!("{}", e);
    })?;
    let config = Arc::new(config);

    let result = match cli.command {
        Commands::Index {
            paths,
            name,
            include,
            exclude,
            languages,
            dry_run,
            schedule,
            unschedule,
        } => {
            cmd_index(
                config, paths, name, include, exclude, languages, dry_run, schedule, unschedule,
            )
            .await
        }
        Commands::Query {
            query,
            repo,
            limit,
            json,
            json_pretty,
            minified,
        } => cmd_query(config, query, repo, limit, json, json_pretty, minified).await,
        Commands::Graph {
            symbol,
            repo,
            depth,
            graph_type,
            direction,
            json,
        } => cmd_graph(config, symbol, repo, depth, graph_type, direction, json).await,
        Commands::InitDb { reset } => cmd_init_db(config, reset).await,
        Commands::Orchestrator(cmd) => cmd_orchestrator(config, cmd).await,
        Commands::Deps {
            check,
            clean_models,
            clean_all,
            verbose,
        } => cmd_deps(check, clean_models, clean_all, verbose).await,
    };

    // Convert errors to user-friendly messages
    if let Err(e) = result {
        use cudgel::Error;
        match &e {
            Error::PostgresNotRunning
            | Error::PgvectorNotInstalled
            | Error::SchemaNotInitialized
            | Error::RepositoryNotFound(_)
            | Error::UnsupportedLanguage(_)
            | Error::Embedding(_) => {
                eprintln!("\n{}", e.to_user_message());
            }
            _ => {
                eprintln!("\n{}: {}", "Error".bright_red().bold(), e);
            }
        }
        std::process::exit(1);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn cmd_index(
    config: Arc<Config>,
    paths: Vec<String>,
    _name: Option<String>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    languages: Option<Vec<String>>,
    dry_run: bool,
    schedule: Option<String>,
    unschedule: bool,
) -> cudgel::Result<()> {
    use cudgel::indexer::IndexFilter;

    // Validate and normalize paths
    let resolved_paths = resolve_index_paths(&paths)?;

    if resolved_paths.is_empty() {
        return Err(cudgel::Error::Other(
            "No valid paths found to index. Check your path patterns.".to_string(),
        ));
    }

    // Build filter configuration
    let filter = IndexFilter::new()
        .with_include_patterns(include.unwrap_or_default())
        .with_exclude_patterns(exclude.unwrap_or_default())
        .with_languages(languages.unwrap_or_default());

    // Validate filter
    filter.validate()?;

    if dry_run {
        println!(
            "{}",
            "DRY RUN - No changes will be made".bright_yellow().bold()
        );
    }

    println!("{}", "Indexing repository...".bright_blue().bold());
    println!("Paths: {}", resolved_paths.len());
    for path in &resolved_paths {
        println!("  - {}", path.display());
    }

    if let Some(patterns) = filter.include_patterns() {
        if !patterns.is_empty() {
            println!("Include patterns: {}", patterns.join(", "));
        }
    }

    if let Some(patterns) = filter.exclude_patterns() {
        if !patterns.is_empty() {
            println!("Exclude patterns: {}", patterns.join(", "));
        }
    }

    if let Some(langs) = filter.languages() {
        if !langs.is_empty() {
            println!("Languages: {}", langs.join(", "));
        }
    }

    if dry_run {
        // Dry run mode - just scan and report
        return cmd_index_dry_run_with_filter(&resolved_paths, &filter).await;
    }

    let db = Arc::new(Database::new(&config).await?);

    // Check database health
    if let Err(e) = db.health_check().await {
        // Convert to user-friendly error
        return Err(e.with_context());
    }

    let mut indexer = Indexer::new(config.clone(), db)?;

    // Use first path as the repository root for now
    // TODO: Support multiple repository roots
    let repo_path = &resolved_paths[0];

    let (repo_id, stats) = indexer
        .index_repository_with_filter(repo_path, &filter)
        .await?;

    println!(
        "\n{}",
        format!("Successfully indexed repository with ID: {}", repo_id)
            .bright_green()
            .bold()
    );

    // Display statistics
    println!("\n{}", "Indexing Statistics:".bright_cyan().bold());
    println!(
        "  Files: {} total, {} indexed, {} failed",
        stats.total_files, stats.indexed_files, stats.failed_files
    );
    println!("  Symbols: {} total", stats.total_symbols);

    if !stats.files_by_language.is_empty() {
        println!("\n  Files by language:");
        for (lang, count) in stats.files_by_language.iter() {
            println!("    {}: {}", lang, count);
        }
    }

    if !stats.symbols_by_kind.is_empty() {
        println!("\n  Symbols by kind:");
        for (kind, count) in stats.symbols_by_kind.iter() {
            println!("    {}: {}", kind, count);
        }
    }

    if !stats.errors.is_empty() {
        println!("\n  {} errors (showing first 10):", stats.errors.len());
        for error in &stats.errors {
            println!("    {}", error);
        }
    }

    // Handle scheduling
    if unschedule {
        let db = cudgel::database::Database::new(&config).await?;
        let deleted = db.delete_scheduled_task(repo_id).await?;
        if deleted > 0 {
            println!(
                "\n{}",
                "Removed scheduled indexing for this repository"
                    .bright_yellow()
                    .bold()
            );
        } else {
            println!(
                "\n{}",
                "No scheduled indexing found for this repository".bright_yellow()
            );
        }
    } else if let Some(schedule_str) = schedule {
        // Parse schedule string to interval in hours
        let interval_hours = match schedule_str.as_str() {
            "hourly" => 1,
            "daily" => 24,
            "weekly" => 168, // 7 * 24
            num => num.parse::<i32>().map_err(|_| {
                cudgel::Error::Other(format!(
                    "Invalid schedule: '{}'. Use 'hourly', 'daily', 'weekly', or a number of hours (1-8760)",
                    schedule_str
                ))
            })?,
        };

        // Validate interval
        if !(1..=8760).contains(&interval_hours) {
            return Err(cudgel::Error::Other(
                "Schedule interval must be between 1 and 8760 hours (1 year)".to_string(),
            ));
        }

        let db = cudgel::database::Database::new(&config).await?;
        db.create_scheduled_task(repo_id, interval_hours).await?;

        println!(
            "\n{}",
            format!(
                "Scheduled automatic re-indexing every {} hour{}",
                interval_hours,
                if interval_hours == 1 { "" } else { "s" }
            )
            .bright_green()
            .bold()
        );
        println!(
            "{}",
            "Start the orchestrator with 'cudgel orchestrator start' to enable scheduled indexing"
                .bright_cyan()
        );
    }

    Ok(())
}

/// Perform a dry run of indexing with file filtering - scan files and report what would be indexed
async fn cmd_index_dry_run_with_filter(
    paths: &[PathBuf],
    filter: &cudgel::indexer::IndexFilter,
) -> cudgel::Result<()> {
    use cudgel::parser::CodeParser;
    use std::collections::HashMap;

    println!("\n{}", "Scanning repository...".bright_cyan().bold());

    let mut files_by_language: HashMap<String, usize> = HashMap::new();
    let mut total_files_discovered = 0;
    let mut supported_files = 0;
    let mut unsupported_files = 0;
    let mut filtered_out = 0;

    for repo_path in paths {
        // Get git tracked files
        let output = std::process::Command::new("git")
            .args(["-C", &repo_path.to_string_lossy(), "ls-files"])
            .output()
            .map_err(|e| cudgel::Error::Other(format!("Failed to run git ls-files: {}", e)))?;

        if !output.status.success() {
            return Err(cudgel::Error::Other(
                "git ls-files failed - is this a git repository?".to_string(),
            ));
        }

        let files_str = String::from_utf8(output.stdout)
            .map_err(|e| cudgel::Error::Other(format!("Invalid UTF-8 from git: {}", e)))?;

        for line in files_str.lines() {
            let file_path = repo_path.join(line.trim());

            if !file_path.is_file() {
                continue;
            }

            total_files_discovered += 1;

            // Apply filter
            if !filter.should_index_file(&file_path) {
                filtered_out += 1;
                continue;
            }

            // Check language support
            if let Some(lang) = CodeParser::detect_language(&file_path) {
                *files_by_language.entry(lang).or_insert(0) += 1;
                supported_files += 1;
            } else {
                unsupported_files += 1;
            }
        }
    }

    println!("\n{}", "Dry Run Summary:".bright_green().bold());
    println!("  Total files discovered: {}", total_files_discovered);
    println!("  Filtered out by patterns/languages: {}", filtered_out);
    println!("  Supported files to index: {}", supported_files);
    println!("  Unsupported/skipped files: {}", unsupported_files);

    if !files_by_language.is_empty() {
        println!("\n  Files to index by language:");
        let mut sorted: Vec<_> = files_by_language.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (lang, count) in sorted {
            println!("    {}: {}", lang, count);
        }
    }

    if supported_files == 0 {
        println!(
            "\n{}",
            "WARNING: No files would be indexed. Check your filter settings."
                .bright_yellow()
                .bold()
        );
    } else {
        println!(
            "\n{}",
            format!(
                "Run without --dry-run to actually index {} files",
                supported_files
            )
            .bright_yellow()
        );
    }

    Ok(())
}

/// Minify query results for LLM consumption (token-efficient format)
fn minify_query_results(results: &[cudgel::query::SymbolResult]) -> cudgel::Result<String> {
    use serde_json::json;

    // Create minified representation with abbreviated keys
    let minified: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            let mut obj = json!({
                "p": r.path,             // path
                "l": r.start_line,       // line
                "n": r.name,             // name
                "k": r.kind,             // kind
                "s": r.similarity,       // similarity
            });

            // Only include optional fields if they have meaningful values
            if let Some(sig) = &r.signature {
                if !sig.is_empty() {
                    obj["g"] = json!(sig); // signature
                }
            }

            if let Some(doc) = &r.docstring {
                if !doc.is_empty() {
                    obj["d"] = json!(doc); // docstring
                }
            }

            obj
        })
        .collect();

    // Compact JSON (no whitespace)
    serde_json::to_string(&minified)
        .map_err(|e| cudgel::Error::Other(format!("Minification failed: {}", e)))
}

async fn cmd_query(
    config: Arc<Config>,
    query: String,
    repo: Option<String>,
    limit: i64,
    json: bool,
    json_pretty: bool,
    minified: bool,
) -> cudgel::Result<()> {
    // Validate query is not empty or whitespace only
    if query.trim().is_empty() {
        return Err(cudgel::Error::Other(
            "Query cannot be empty. Please provide a meaningful search term.".to_string(),
        ));
    }

    let db = Arc::new(Database::new(&config).await?);
    let query_engine = QueryEngine::new(config.clone(), db)?;

    let results = query_engine
        .search_symbols(&query, limit, repo.as_deref())
        .await?;

    // Determine output format
    if json || json_pretty || minified {
        let json_str = if minified {
            // Minified format: compact JSON with abbreviated keys
            minify_query_results(&results)?
        } else if json {
            // Compact JSON (single line)
            serde_json::to_string(&results)
                .map_err(|e| cudgel::Error::Other(format!("JSON serialization failed: {}", e)))?
        } else {
            // Pretty-printed JSON (indented)
            serde_json::to_string_pretty(&results)
                .map_err(|e| cudgel::Error::Other(format!("JSON serialization failed: {}", e)))?
        };

        println!("{}", json_str);
    } else {
        display_query_results(&results);
    }

    Ok(())
}

async fn cmd_graph(
    config: Arc<Config>,
    symbol: String,
    repo: Option<String>,
    depth: usize,
    _graph_type: String,
    _direction: String,
    json: bool,
) -> cudgel::Result<()> {
    // Validate symbol name is not empty
    if symbol.trim().is_empty() {
        return Err(cudgel::Error::Other(
            "Symbol name cannot be empty. Please provide a valid symbol name.".to_string(),
        ));
    }

    let db = Arc::new(Database::new(&config).await?);
    let graph_query = GraphQuery::new(db);

    let graph = graph_query
        .get_references(&symbol, repo.as_deref(), depth)
        .await?;

    if json {
        match serde_json::to_string_pretty(&graph) {
            Ok(json_str) => println!("{}", json_str),
            Err(e) => {
                eprintln!(
                    "{}: Failed to serialize graph to JSON: {}",
                    "Error".bright_red().bold(),
                    e
                );
                return Err(cudgel::Error::Other(format!(
                    "JSON serialization failed: {}",
                    e
                )));
            }
        }
    } else {
        display_graph(&graph);
    }

    Ok(())
}

/// Resolve index paths from user input, supporting glob patterns and ./... syntax
fn resolve_index_paths(patterns: &[String]) -> cudgel::Result<Vec<PathBuf>> {
    use glob::glob;
    use std::collections::HashSet;

    let mut resolved = HashSet::new();

    for pattern in patterns {
        // Handle ./... syntax (Go-style recursive)
        if pattern.ends_with("/...") || pattern == "./..." {
            let base = pattern.trim_end_matches("/...");
            let base_path = if base.is_empty() || base == "." {
                PathBuf::from(".")
            } else {
                PathBuf::from(base)
            };

            if !base_path.exists() {
                return Err(cudgel::Error::Other(format!(
                    "Path does not exist: {}",
                    base_path.display()
                )));
            }

            resolved.insert(base_path.canonicalize().map_err(|e| {
                cudgel::Error::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to canonicalize path: {}", e),
                ))
            })?);
            continue;
        }

        // Check if pattern contains glob characters
        if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
            // Use glob to expand pattern
            match glob(pattern) {
                Ok(paths) => {
                    let mut found_any = false;
                    for entry in paths {
                        match entry {
                            Ok(path) => {
                                found_any = true;
                                if path.is_dir() {
                                    if let Ok(canonical) = path.canonicalize() {
                                        resolved.insert(canonical);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to read glob entry: {}", e);
                            }
                        }
                    }
                    if !found_any {
                        eprintln!("Warning: Pattern '{}' matched no directories", pattern);
                    }
                }
                Err(e) => {
                    return Err(cudgel::Error::Other(format!(
                        "Invalid glob pattern '{}': {}",
                        pattern, e
                    )));
                }
            }
        } else {
            // Regular path
            let path = PathBuf::from(pattern);
            if !path.exists() {
                return Err(cudgel::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Path does not exist: {}", path.display()),
                )));
            }

            if !path.is_dir() {
                return Err(cudgel::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Path is not a directory: {}", path.display()),
                )));
            }

            resolved.insert(path.canonicalize().map_err(|e| {
                cudgel::Error::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to canonicalize path: {}", e),
                ))
            })?);
        }
    }

    let mut paths: Vec<PathBuf> = resolved.into_iter().collect();
    paths.sort();
    Ok(paths)
}

/// Prompt user for confirmation
fn confirm(prompt: &str) -> bool {
    use std::io::{self, Write};

    print!("{} [y/N]: ", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

async fn cmd_init_db(config: Arc<Config>, reset: bool) -> cudgel::Result<()> {
    if reset {
        println!(
            "{}",
            "⚠️  WARNING: Resetting database - ALL DATA WILL BE DELETED!"
                .bright_red()
                .bold()
        );
        println!("\nThis will:");
        println!("  • Delete all indexed repositories");
        println!("  • Delete all code symbols and embeddings");
        println!("  • Delete all relationships and metadata");
        println!("\nThis action cannot be undone!");

        if !confirm("\nAre you sure you want to reset the database?") {
            println!("{}", "Database reset cancelled.".yellow());
            return Ok(());
        }

        println!("\n{}", "Dropping all tables...".yellow());

        let db = Database::new(&config).await?;
        db.reset_schema().await?;

        println!(
            "{}",
            "Database schema reset successfully".bright_green().bold()
        );
    } else {
        println!("{}", "Initializing database schema...".bright_blue().bold());

        let db = Database::new(&config).await?;
        db.init_schema().await?;

        println!(
            "{}",
            "Database schema initialized successfully"
                .bright_green()
                .bold()
        );
    }

    Ok(())
}

async fn cmd_orchestrator(config: Arc<Config>, cmd: OrchestratorCommand) -> cudgel::Result<()> {
    use cudgel::orchestrator;

    match cmd {
        OrchestratorCommand::Start => {
            println!("{}", "Starting orchestrator daemon...".bright_blue().bold());
            orchestrator::start_daemon(&config)?;
            println!(
                "{}",
                "Orchestrator daemon started successfully"
                    .bright_green()
                    .bold()
            );
            println!(
                "Logs: {}",
                cudgel::config::xdg_state_home()
                    .join("cudgel/orchestrator.log")
                    .display()
            );
        }
        OrchestratorCommand::Stop => {
            println!("{}", "Stopping orchestrator daemon...".bright_blue().bold());
            orchestrator::stop_daemon()?;
            println!(
                "{}",
                "Orchestrator daemon stopped successfully"
                    .bright_green()
                    .bold()
            );
        }
        OrchestratorCommand::Status => {
            match orchestrator::is_running()? {
                Some(pid) => {
                    println!(
                        "{}",
                        format!("Orchestrator is running (PID: {})", pid)
                            .bright_green()
                            .bold()
                    );

                    // Display scheduled tasks
                    let db = cudgel::database::Database::new(&config).await?;
                    let tasks = db.get_scheduled_tasks().await?;

                    if tasks.is_empty() {
                        println!("\n{}", "No scheduled tasks".yellow());
                    } else {
                        println!("\n{}", "Scheduled Tasks:".bright_cyan().bold());
                        for task in tasks {
                            let repo = db.get_repository(task.repo_id).await?;
                            let repo_name = repo
                                .map(|r| r.name)
                                .unwrap_or_else(|| format!("Unknown (ID: {})", task.repo_id));
                            println!(
                                "  • {} - every {} hour{}, next run: {}",
                                repo_name,
                                task.interval_hours,
                                if task.interval_hours == 1 { "" } else { "s" },
                                task.next_run_at.format("%Y-%m-%d %H:%M:%S")
                            );
                        }
                    }
                }
                None => {
                    println!("{}", "Orchestrator is not running".bright_yellow().bold());
                }
            }
        }
        OrchestratorCommand::Restart => {
            println!(
                "{}",
                "Restarting orchestrator daemon...".bright_blue().bold()
            );
            orchestrator::restart_daemon(&config)?;
            println!(
                "{}",
                "Orchestrator daemon restarted successfully"
                    .bright_green()
                    .bold()
            );
        }
        OrchestratorCommand::RunDaemon => {
            // This is called internally by the daemon process
            // Set up logging to file
            use tracing_subscriber::EnvFilter;

            let log_path = cudgel::config::xdg_state_home().join("cudgel/orchestrator.log");
            let log_file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)?;

            let _ = tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
                )
                .with_writer(log_file)
                .try_init();

            // Run the polling loop
            let config_owned = Arc::try_unwrap(config).unwrap_or_else(|arc| (*arc).clone());
            orchestrator::run_polling_loop(config_owned).await?;
        }
    }

    Ok(())
}

async fn cmd_deps(check: bool, clean_models: bool, clean_all: bool, verbose: bool) -> cudgel::Result<()> {
    use cudgel::deps;

    if clean_all {
        println!(
            "{}",
            "⚠️  WARNING: Cleaning all data - ALL DATA WILL BE DELETED!"
                .bright_red()
                .bold()
        );
        println!("\nThis will:");
        println!("  • Stop the PostgreSQL database");
        println!("  • Delete all downloaded models");
        println!("  • Delete the PostgreSQL data directory");
        println!("  • Remove all indexed code and embeddings");
        println!("\nThis action cannot be undone!");

        if !confirm("\nAre you sure you want to clean all data?") {
            println!("{}", "Clean cancelled.".yellow());
            return Ok(());
        }

        println!();
        deps::clean_all().await?;
        return Ok(());
    }

    if clean_models {
        println!("{}", "Cleaning downloaded models...".bright_blue().bold());
        deps::clean_models().await?;
        return Ok(());
    }

    if check {
        // Validation-only mode
        println!("{}", "Checking dependencies...".bright_blue().bold());
        let dependencies = deps::validate_only().await?;

        let checker = deps::checker::DependencyChecker::default();
        println!("{}", checker.format_validation_table(&dependencies));

        if verbose {
            let diagnostics = checker.collect_diagnostics()?;
            println!("{}", diagnostics);
        }

        let all_satisfied = dependencies.iter().all(|d| d.is_satisfied());
        if !all_satisfied {
            println!(
                "\n{}",
                "Some dependencies are not satisfied. Run: cudgel deps"
                    .bright_yellow()
                    .bold()
            );
            std::process::exit(1);
        } else {
            println!("\n{}", "All dependencies satisfied!".bright_green().bold());
        }

        return Ok(());
    }

    // Install mode (default)
    deps::install_all().await?;

    if verbose {
        let checker = deps::checker::DependencyChecker::default();
        let diagnostics = checker.collect_diagnostics()?;
        println!("{}", diagnostics);
    }

    Ok(())
}

fn display_query_results(results: &[cudgel::query::SymbolResult]) {
    if results.is_empty() {
        println!("{}", "No results found".yellow());
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        "Name",
        "Kind",
        "Signature",
        "Repo",
        "File",
        "Language",
        "Line",
        "Similarity",
    ]);

    for result in results {
        table.add_row(vec![
            result.name.clone(),
            result.kind.clone(),
            result.signature.clone().unwrap_or_else(|| "-".to_string()),
            result.repo_name.clone(),
            result.path.clone(),
            result.language.clone().unwrap_or_else(|| "-".to_string()),
            result.start_line.to_string(),
            format!("{:.3}", result.similarity),
        ]);
    }

    println!("\n{}", "Symbols:".bright_cyan().bold());
    println!("{}", table);
}

fn display_graph(graph: &cudgel::graph::Graph) {
    if graph.nodes.is_empty() {
        println!("{}", "No graph data found".yellow());
        return;
    }

    println!(
        "\n{} {}",
        "Graph for:".bright_cyan().bold(),
        graph.root.bright_white().bold()
    );
    println!("Nodes: {}, Edges: {}", graph.nodes.len(), graph.edges.len());

    if !graph.edges.is_empty() {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec!["From", "To", "Type"]);

        for edge in &graph.edges {
            let from_node = graph.nodes.iter().find(|n| n.id == edge.from);
            let to_node = graph.nodes.iter().find(|n| n.id == edge.to);

            if let (Some(from), Some(to)) = (from_node, to_node) {
                table.add_row(vec![
                    from.name.clone(),
                    to.name.clone(),
                    edge.edge_type.clone(),
                ]);
            }
        }

        println!("\n{}", "Edges:".bright_cyan().bold());
        println!("{}", table);
    }
}
