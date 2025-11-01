//! Cudgel CLI - Code indexing tool

use clap::{Parser, Subcommand};
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};
use cudgel::{
    config::Config, database::Database, graph::GraphQuery, indexer::Indexer, query::QueryEngine,
};
use std::path::PathBuf;
use std::sync::Arc;
use walkdir::WalkDir;
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
        /// Path to the repository
        path: PathBuf,

        /// Repository name (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,

        /// Dry run - show what would be indexed without actually indexing
        #[arg(long)]
        dry_run: bool,
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

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
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
    let config = Arc::new(Config::from_env().map_err(|e| {
        eprintln!("{}", "Configuration Error:".bright_red().bold());
        eprintln!("{}", e.to_string());
        e
    })?);

    let result = match cli.command {
        Commands::Index {
            path,
            name,
            dry_run,
        } => cmd_index(config, path, name, dry_run).await,
        Commands::Query {
            query,
            repo,
            limit,
            json,
        } => cmd_query(config, query, repo, limit, json).await,
        Commands::Graph {
            symbol,
            repo,
            depth,
            graph_type,
            direction,
            json,
        } => cmd_graph(config, symbol, repo, depth, graph_type, direction, json).await,
        Commands::InitDb { reset } => cmd_init_db(config, reset).await,
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

async fn cmd_index(
    config: Arc<Config>,
    path: PathBuf,
    _name: Option<String>,
    dry_run: bool,
) -> cudgel::Result<()> {
    // Validate path exists and is accessible
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

    // Check if path is readable
    if let Err(e) = std::fs::read_dir(&path) {
        return Err(cudgel::Error::Io(std::io::Error::new(
            e.kind(),
            format!(
                "Cannot read directory: {}. Check permissions.",
                path.display()
            ),
        )));
    }

    if dry_run {
        println!(
            "{}",
            "DRY RUN - No changes will be made".bright_yellow().bold()
        );
    }

    println!("{}", "Indexing repository...".bright_blue().bold());
    println!("Path: {}", path.display());

    if dry_run {
        // Dry run mode - just scan and report
        return cmd_index_dry_run(&path).await;
    }

    let db = Arc::new(Database::new(&config).await?);

    // Check database health
    if let Err(e) = db.health_check().await {
        // Convert to user-friendly error
        return Err(e.with_context());
    }

    let mut indexer = Indexer::new(config.clone(), db)?;

    let (repo_id, stats) = indexer.index_repository(&path).await?;

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

    Ok(())
}

/// Perform a dry run of indexing - scan files and report what would be indexed
async fn cmd_index_dry_run(path: &PathBuf) -> cudgel::Result<()> {
    use cudgel::parser::CodeParser;
    use std::collections::HashMap;

    println!("\n{}", "Scanning repository...".bright_cyan().bold());

    let mut files_by_language: HashMap<String, usize> = HashMap::new();
    let mut total_files = 0;
    let mut supported_files = 0;
    let mut unsupported_files = 0;

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let file_path = entry.path();
        total_files += 1;

        if let Some(lang) = CodeParser::detect_language(file_path) {
            *files_by_language.entry(lang).or_insert(0) += 1;
            supported_files += 1;
        } else {
            unsupported_files += 1;
        }
    }

    println!("\n{}", "Dry Run Summary:".bright_green().bold());
    println!("  Total files found: {}", total_files);
    println!("  Supported files: {} ", supported_files);
    println!("  Unsupported/skipped files: {}", unsupported_files);

    if !files_by_language.is_empty() {
        println!("\n  Files by language:");
        let mut sorted: Vec<_> = files_by_language.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (lang, count) in sorted {
            println!("    {}: {}", lang, count);
        }
    }

    println!(
        "\n{}",
        format!(
            "Run without --dry-run to actually index {} files",
            supported_files
        )
        .bright_yellow()
    );

    Ok(())
}

async fn cmd_query(
    config: Arc<Config>,
    query: String,
    repo: Option<String>,
    limit: i64,
    json: bool,
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

    if json {
        match serde_json::to_string_pretty(&results) {
            Ok(json_str) => println!("{}", json_str),
            Err(e) => {
                eprintln!(
                    "{}: Failed to serialize results to JSON: {}",
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
