use cudgel::{
    config::Config, database::Database, embeddings::EmbeddingGenerator, graph::GraphQuery,
    indexer::Indexer, parser::CodeParser, query::QueryEngine,
};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper to check if PostgreSQL is available for testing
async fn is_postgres_available() -> bool {
    let config = Config::local();
    match Database::new(&config).await {
        Ok(db) => db.health_check().await.unwrap_or(false),
        Err(_) => false,
    }
}

/// Create a test database connection or skip the test
async fn setup_test_db() -> Option<Arc<Database>> {
    if !is_postgres_available().await {
        eprintln!("Skipping test: PostgreSQL not available");
        return None;
    }

    let config = Config::local();
    let db = Database::new(&config).await.ok()?;

    // Initialize schema
    db.init_schema().await.ok()?;

    Some(Arc::new(db))
}

/// Create a temporary test repository with sample files
fn create_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Initialize as a git repository
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(base_path)
        .output()
        .expect("Failed to initialize git repository");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(base_path)
        .output()
        .expect("Failed to set git user email");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(base_path)
        .output()
        .expect("Failed to set git user name");

    // Create a simple Python file
    let python_code = r#"
def hello_world(name):
    """Say hello to someone."""
    return f"Hello, {name}!"

class Calculator:
    """A simple calculator class."""

    def add(self, a, b):
        """Add two numbers."""
        return a + b

    def subtract(self, a, b):
        """Subtract b from a."""
        return a - b
"#;
    fs::write(base_path.join("test.py"), python_code).unwrap();

    // Create a simple Rust file
    let rust_code = r#"
pub fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    pub fn distance(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}
"#;
    fs::write(base_path.join("test.rs"), rust_code).unwrap();

    // Create a JavaScript file
    let js_code = r#"
function greet(name) {
    return `Hello, ${name}!`;
}

class User {
    constructor(name, email) {
        this.name = name;
        this.email = email;
    }

    getDisplayName() {
        return this.name;
    }
}

const processData = (data) => {
    return data.map(x => x * 2);
};
"#;
    fs::write(base_path.join("test.js"), js_code).unwrap();

    // Add files to git
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(base_path)
        .output()
        .expect("Failed to git add files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(base_path)
        .output()
        .expect("Failed to git commit");

    temp_dir
}

#[tokio::test]
async fn test_database_health_check() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return, // Skip test if DB not available
    };

    assert!(db.health_check().await.is_ok());
}

#[tokio::test]
async fn test_pgvector_extension() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    let has_pgvector = db.check_pgvector().await.unwrap();
    assert!(has_pgvector, "pgvector extension should be installed");
}

#[test]
fn test_parser_detect_language() {
    assert_eq!(
        CodeParser::detect_language(Path::new("test.py")),
        Some("python".to_string())
    );
    assert_eq!(
        CodeParser::detect_language(Path::new("test.rs")),
        Some("rust".to_string())
    );
    assert_eq!(
        CodeParser::detect_language(Path::new("test.js")),
        Some("javascript".to_string())
    );
    assert_eq!(
        CodeParser::detect_language(Path::new("test.ts")),
        Some("typescript".to_string())
    );
    assert_eq!(
        CodeParser::detect_language(Path::new("test.go")),
        Some("go".to_string())
    );
    assert_eq!(CodeParser::detect_language(Path::new("test.txt")), None);
}

#[test]
fn test_parser_parse_python() {
    let mut parser = CodeParser::new();
    let code = r#"
def hello():
    return "world"
"#;

    let result = parser.parse_file(Path::new("test.py"), code);
    assert!(result.is_ok());

    let (ast, _hash) = result.unwrap();
    assert_eq!(ast.node_type, "module");
}

#[test]
fn test_parser_extract_symbols_python() {
    let mut parser = CodeParser::new();
    let code = r#"
def hello():
    """A greeting function."""
    return "world"

class Foo:
    def bar(self):
        pass
"#;

    let (ast, _) = parser.parse_file(Path::new("test.py"), code).unwrap();
    let symbols = parser.extract_symbols(&ast, "python");

    assert!(symbols.len() >= 2);
    assert!(symbols.iter().any(|s| s.name.contains("hello")));
    assert!(symbols.iter().any(|s| s.name.contains("Foo")));
}

#[test]
fn test_parser_extract_symbols_rust() {
    let mut parser = CodeParser::new();
    let code = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}

struct Point {
    x: f64,
    y: f64,
}
"#;

    let (ast, _) = parser.parse_file(Path::new("test.rs"), code).unwrap();
    let symbols = parser.extract_symbols(&ast, "rust");

    assert!(symbols.len() >= 2);
    assert!(symbols.iter().any(|s| s.name.contains("add")));
    assert!(symbols.iter().any(|s| s.name.contains("Point")));
}

#[tokio::test]
async fn test_config_validation() {
    // Test valid config
    let config = Config::local();
    assert!(config.validate().is_ok());

    // Config should have default values
    assert_eq!(
        config.database.host,
        std::env::var("PGHOST").unwrap_or_else(|_| "localhost".to_string())
    );
}

#[test]
fn test_config_validation_invalid_port() {
    use cudgel::config::{Config, DatabaseConfig, EmbeddingConfig, IndexingConfig};
    use std::path::PathBuf;

    // Test with port 0 (invalid)
    let config = Config {
        database: DatabaseConfig {
            host: "localhost".to_string(),
            port: 0,
            database: "cudgel".to_string(),
            user: "test".to_string(),
            password: "test".to_string(),
        },
        embedding: EmbeddingConfig {
            model_path: PathBuf::from("./models"),
            dimension: 384,
        },
        indexing: IndexingConfig {
            batch_size: 100,
            max_file_size: 1024 * 1024,
        },
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_config_validation_empty_host() {
    use cudgel::config::{Config, DatabaseConfig, EmbeddingConfig, IndexingConfig};
    use std::path::PathBuf;

    let config = Config {
        database: DatabaseConfig {
            host: "".to_string(),
            port: 5432,
            database: "cudgel".to_string(),
            user: "test".to_string(),
            password: "test".to_string(),
        },
        embedding: EmbeddingConfig {
            model_path: PathBuf::from("./models"),
            dimension: 384,
        },
        indexing: IndexingConfig {
            batch_size: 100,
            max_file_size: 1024 * 1024,
        },
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_config_validation_invalid_dimension() {
    use cudgel::config::{Config, DatabaseConfig, EmbeddingConfig, IndexingConfig};
    use std::path::PathBuf;

    let config = Config {
        database: DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "cudgel".to_string(),
            user: "test".to_string(),
            password: "test".to_string(),
        },
        embedding: EmbeddingConfig {
            model_path: PathBuf::from("./models"),
            dimension: 0, // Invalid - must be positive
        },
        indexing: IndexingConfig {
            batch_size: 100,
            max_file_size: 1024 * 1024,
        },
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_config_validation_invalid_batch_size() {
    use cudgel::config::{Config, DatabaseConfig, EmbeddingConfig, IndexingConfig};
    use std::path::PathBuf;

    let config = Config {
        database: DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "cudgel".to_string(),
            user: "test".to_string(),
            password: "test".to_string(),
        },
        embedding: EmbeddingConfig {
            model_path: PathBuf::from("./models"),
            dimension: 384,
        },
        indexing: IndexingConfig {
            batch_size: 0, // Invalid - must be positive
            max_file_size: 1024 * 1024,
        },
    };

    assert!(config.validate().is_err());

    // Test batch size too large
    let config = Config {
        database: DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "cudgel".to_string(),
            user: "test".to_string(),
            password: "test".to_string(),
        },
        embedding: EmbeddingConfig {
            model_path: PathBuf::from("./models"),
            dimension: 384,
        },
        indexing: IndexingConfig {
            batch_size: 20000, // Too large
            max_file_size: 1024 * 1024,
        },
    };

    assert!(config.validate().is_err());
}

#[test]
fn test_parser_large_file_handling() {
    let mut parser = CodeParser::new();

    // Test parser can handle reasonably large files
    let large_code = "def function_".to_string() + &"a".repeat(1000) + "():\n    pass\n";
    let result = parser.parse_file(Path::new("test.py"), &large_code);
    assert!(result.is_ok());
}

#[test]
fn test_parser_syntax_error_handling() {
    let mut parser = CodeParser::new();

    // Python code with syntax errors should still parse (tree-sitter is error-tolerant)
    let code = "def incomplete_function(\n";
    let result = parser.parse_file(Path::new("test.py"), code);
    // Tree-sitter can parse incomplete/invalid syntax, so this should succeed
    assert!(result.is_ok());
}

#[test]
fn test_embedding_generation() {
    let config = Arc::new(Config::local());
    let embedder = EmbeddingGenerator::new(config).expect("Failed to create embedder");

    let embedding = embedder.encode("test text");
    if let Err(e) = &embedding {
        eprintln!("Embedding error: {:?}", e);
    }
    assert!(
        embedding.is_ok(),
        "Failed to generate embedding: {:?}",
        embedding.err()
    );

    let embedding = embedding.unwrap();
    assert_eq!(embedding.len(), 384); // Default dimension

    // Check normalization (L2 norm should be approximately 1)
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 0.01);
}

#[tokio::test]
async fn test_repository_indexing() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    let config = Arc::new(Config::local());
    let temp_repo = create_test_repo();

    let mut indexer = Indexer::new(config.clone(), db.clone()).unwrap();
    let result = indexer.index_repository(temp_repo.path()).await;

    assert!(result.is_ok());
    let (repo_id, stats) = result.unwrap();
    assert!(repo_id > 0);

    // Check statistics
    assert!(stats.total_files >= 3); // At least our 3 test files
    assert!(stats.indexed_files > 0);
    assert!(stats.total_symbols > 0);
    assert!(!stats.files_by_language.is_empty());
}

#[tokio::test]
async fn test_symbol_query() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    let config = Arc::new(Config::local());
    let temp_repo = create_test_repo();

    // Index the repository
    let mut indexer = Indexer::new(config.clone(), db.clone()).unwrap();
    let (_, _) = indexer.index_repository(temp_repo.path()).await.unwrap();

    // Query for symbols
    let query_engine = QueryEngine::new(config.clone(), db.clone()).unwrap();

    // Test with multiple queries to ensure semantic search works
    // The test repo has functions like "hello_world", "Calculator.add", "fibonacci", etc.
    let queries = vec![
        "calculator add subtract",
        "fibonacci recursive",
        "greeting message",
        "math calculation",
    ];

    let mut found_results = false;
    for query in queries {
        let results = query_engine.search_symbols(query, 10, None).await;
        assert!(
            results.is_ok(),
            "Query '{}' failed: {:?}",
            query,
            results.err()
        );

        if !results.as_ref().unwrap().is_empty() {
            found_results = true;
            eprintln!("Query '{}' found {} results", query, results.unwrap().len());
            break;
        }
    }

    // With real semantic embeddings, at least one query should return results
    assert!(found_results,
        "Expected to find symbols with real semantic embeddings, but all queries returned 0 results");
}

#[tokio::test]
async fn test_graph_query() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    let config = Arc::new(Config::local());
    let temp_repo = create_test_repo();

    // Index the repository
    let mut indexer = Indexer::new(config.clone(), db.clone()).unwrap();
    indexer.index_repository(temp_repo.path()).await.unwrap();

    // Query graph
    let graph_query = GraphQuery::new(db.clone());
    let result = graph_query.get_references("hello_world", None, 1).await;

    assert!(result.is_ok());
    // Graph might be empty if no references exist, which is fine for this test
}

#[tokio::test]
async fn test_database_operations() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    // Add a repository
    let repo_id = db.add_repository("/test/path", "test_repo").await;
    assert!(repo_id.is_ok());
    let repo_id = repo_id.unwrap();
    assert!(repo_id > 0);

    // Add a file
    let file_id = db
        .add_file(repo_id, "test.py", Some("python"), "# test", "hash123")
        .await;
    assert!(file_id.is_ok());
    let file_id = file_id.unwrap();
    assert!(file_id > 0);

    // Add a symbol
    let embedding = vec![0.1; 384];
    let symbol_id = db
        .add_symbol(
            file_id,
            "test_function",
            "function",
            Some("def test_function():"),
            Some("A test function"),
            1,
            10,
            &embedding,
        )
        .await;
    assert!(symbol_id.is_ok());
    assert!(symbol_id.unwrap() > 0);
}

// ============================================================================
// IndexFilter Tests
// ============================================================================

#[test]
fn test_index_filter_validation_valid_languages() {
    use cudgel::indexer::IndexFilter;

    // Test all supported languages
    let filter = IndexFilter::new().with_languages(vec![
        "python".to_string(),
        "rust".to_string(),
        "javascript".to_string(),
        "typescript".to_string(),
        "go".to_string(),
        "c".to_string(),
        "cpp".to_string(),
        "java".to_string(),
    ]);

    assert!(filter.validate().is_ok());
}

#[test]
fn test_index_filter_validation_invalid_language() {
    use cudgel::indexer::IndexFilter;

    let filter = IndexFilter::new().with_languages(vec!["ruby".to_string()]);

    let result = filter.validate();
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(error.contains("Unsupported language"));
    assert!(error.contains("ruby"));
}

#[test]
fn test_index_filter_validation_invalid_include_pattern() {
    use cudgel::indexer::IndexFilter;

    // Invalid glob pattern (unmatched bracket)
    let filter = IndexFilter::new().with_include_patterns(vec!["src/**/*[".to_string()]);

    let result = filter.validate();
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(error.contains("Invalid include pattern"));
}

#[test]
fn test_index_filter_validation_invalid_exclude_pattern() {
    use cudgel::indexer::IndexFilter;

    // Invalid glob pattern (unmatched bracket)
    let filter = IndexFilter::new().with_exclude_patterns(vec!["target/**/*[".to_string()]);

    let result = filter.validate();
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(error.contains("Invalid exclude pattern"));
}

#[test]
fn test_index_filter_empty_filter() {
    use cudgel::indexer::IndexFilter;

    let filter = IndexFilter::new();
    assert!(filter.is_empty());
    assert!(filter.validate().is_ok());
}

#[test]
fn test_index_filter_language_filtering() {
    use cudgel::indexer::IndexFilter;
    use std::path::Path;

    let filter = IndexFilter::new().with_languages(vec!["rust".to_string(), "python".to_string()]);

    // Should match Rust and Python files
    assert!(filter.should_index_file(Path::new("src/main.rs")));
    assert!(filter.should_index_file(Path::new("test.py")));

    // Should not match other languages
    assert!(!filter.should_index_file(Path::new("app.js")));
    assert!(!filter.should_index_file(Path::new("Main.java")));
    assert!(!filter.should_index_file(Path::new("main.go")));
}

#[test]
fn test_index_filter_include_patterns() {
    use cudgel::indexer::IndexFilter;
    use std::path::Path;

    let filter = IndexFilter::new().with_include_patterns(vec!["**/src/**/*.rs".to_string()]);

    // Should match files in src directory
    assert!(filter.should_index_file(Path::new("project/src/main.rs")));
    assert!(filter.should_index_file(Path::new("src/lib.rs")));

    // Should not match files outside src directory
    assert!(!filter.should_index_file(Path::new("tests/test.rs")));
    assert!(!filter.should_index_file(Path::new("main.rs")));
}

#[test]
fn test_index_filter_exclude_patterns() {
    use cudgel::indexer::IndexFilter;
    use std::path::Path;

    let filter = IndexFilter::new().with_exclude_patterns(vec![
        "**/target/**".to_string(),
        "**/node_modules/**".to_string(),
    ]);

    // Should not match excluded directories
    assert!(!filter.should_index_file(Path::new("project/target/debug/main.rs")));
    assert!(!filter.should_index_file(Path::new("app/node_modules/package/index.js")));

    // Should match files not in excluded directories
    assert!(filter.should_index_file(Path::new("src/main.rs")));
    assert!(filter.should_index_file(Path::new("app/src/index.js")));
}

#[test]
fn test_index_filter_exclude_takes_precedence() {
    use cudgel::indexer::IndexFilter;
    use std::path::Path;

    let filter = IndexFilter::new()
        .with_include_patterns(vec!["**/*.rs".to_string()])
        .with_exclude_patterns(vec!["**/target/**".to_string()]);

    // Should not match even though it's a .rs file (exclude takes precedence)
    assert!(!filter.should_index_file(Path::new("project/target/debug/main.rs")));

    // Should match .rs files not in target
    assert!(filter.should_index_file(Path::new("src/main.rs")));
}

#[test]
fn test_index_filter_combined_filters() {
    use cudgel::indexer::IndexFilter;
    use std::path::Path;

    let filter = IndexFilter::new()
        .with_include_patterns(vec!["**/src/**".to_string()])
        .with_exclude_patterns(vec!["**/*test*.rs".to_string()])
        .with_languages(vec!["rust".to_string()]);

    // Should match: in src, is rust, not a test file
    assert!(filter.should_index_file(Path::new("project/src/main.rs")));
    assert!(filter.should_index_file(Path::new("src/lib.rs")));

    // Should not match: not in src
    assert!(!filter.should_index_file(Path::new("main.rs")));

    // Should not match: is a test file (excluded)
    assert!(!filter.should_index_file(Path::new("src/main_test.rs")));
    assert!(!filter.should_index_file(Path::new("src/test_utils.rs")));

    // Should not match: wrong language
    assert!(!filter.should_index_file(Path::new("src/main.py")));
}

#[test]
fn test_index_filter_wildcards() {
    use cudgel::indexer::IndexFilter;
    use std::path::Path;

    let filter = IndexFilter::new().with_include_patterns(vec![
        "**/*.rs".to_string(),     // All .rs files
        "src/**/*.py".to_string(), // Python files in src
    ]);

    // Should match .rs files at any level
    assert!(filter.should_index_file(Path::new("main.rs")));
    assert!(filter.should_index_file(Path::new("lib.rs")));
    assert!(filter.should_index_file(Path::new("src/main.rs")));

    // Should match nested .py files in src
    assert!(filter.should_index_file(Path::new("src/utils.py")));
    assert!(filter.should_index_file(Path::new("src/nested/helper.py")));

    // Should not match .py files outside src
    assert!(!filter.should_index_file(Path::new("test.py")));
    assert!(!filter.should_index_file(Path::new("scripts/build.py")));
}

#[test]
fn test_index_filter_accessor_methods() {
    use cudgel::indexer::IndexFilter;

    let include = vec!["*.rs".to_string()];
    let exclude = vec!["**/target/**".to_string()];
    let languages = vec!["rust".to_string()];

    let filter = IndexFilter::new()
        .with_include_patterns(include.clone())
        .with_exclude_patterns(exclude.clone())
        .with_languages(languages.clone());

    assert_eq!(filter.include_patterns(), Some(&include));
    assert_eq!(filter.exclude_patterns(), Some(&exclude));
    assert_eq!(filter.languages(), Some(&languages));
    assert!(!filter.is_empty());
}

#[test]
fn test_index_filter_empty_patterns() {
    use cudgel::indexer::IndexFilter;

    // Empty vectors should be treated as None
    let filter = IndexFilter::new()
        .with_include_patterns(vec![])
        .with_exclude_patterns(vec![])
        .with_languages(vec![]);

    assert!(filter.is_empty());
    assert_eq!(filter.include_patterns(), None);
    assert_eq!(filter.exclude_patterns(), None);
    assert_eq!(filter.languages(), None);
}

#[test]
fn test_index_filter_case_sensitivity() {
    use cudgel::indexer::IndexFilter;
    use std::path::Path;

    let filter = IndexFilter::new().with_include_patterns(vec!["**/*.RS".to_string()]);

    // Glob patterns are typically case-sensitive on Unix, case-insensitive on Windows
    // This test documents the behavior without asserting it
    let matches_lowercase = filter.should_index_file(Path::new("src/main.rs"));
    let matches_uppercase = filter.should_index_file(Path::new("src/MAIN.RS"));

    // At least one should match on any platform
    assert!(matches_lowercase || matches_uppercase);
}

// ============================================================================
// Orchestrator and Scheduling Tests (User Story 2)
// ============================================================================

#[tokio::test]
async fn test_create_scheduled_task() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    // Create a test repository first
    let repo_id = db
        .add_repository("/test/repo", "test_scheduled_repo")
        .await
        .expect("Failed to add repository");

    // Create a scheduled task
    let task_id = db
        .create_scheduled_task(repo_id, 24)
        .await
        .expect("Failed to create scheduled task");

    assert!(task_id > 0);

    // Verify the task was created
    let tasks = db
        .get_scheduled_tasks()
        .await
        .expect("Failed to get scheduled tasks");

    assert!(!tasks.is_empty());
    let task = tasks.iter().find(|t| t.id == task_id).unwrap();
    assert_eq!(task.repo_id, repo_id);
    assert_eq!(task.interval_hours, 24);
    assert_eq!(task.status, "active");
}

#[tokio::test]
async fn test_scheduled_task_upsert() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    // Create a test repository
    let repo_id = db
        .add_repository("/test/repo2", "test_upsert_repo")
        .await
        .expect("Failed to add repository");

    // Create initial scheduled task (hourly)
    let task_id1 = db
        .create_scheduled_task(repo_id, 1)
        .await
        .expect("Failed to create first task");

    // Update to daily - should upsert and return same task_id
    let task_id2 = db
        .create_scheduled_task(repo_id, 24)
        .await
        .expect("Failed to update task");

    assert_eq!(task_id1, task_id2, "Task ID should remain the same on upsert");

    // Verify the interval was updated
    let tasks = db.get_scheduled_tasks().await.unwrap();
    let task = tasks.iter().find(|t| t.id == task_id1).unwrap();
    assert_eq!(task.interval_hours, 24);
}

#[tokio::test]
async fn test_delete_scheduled_task() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    // Create a test repository and task
    let repo_id = db
        .add_repository("/test/repo3", "test_delete_repo")
        .await
        .expect("Failed to add repository");

    db.create_scheduled_task(repo_id, 24)
        .await
        .expect("Failed to create task");

    // Delete the task
    let deleted_count = db
        .delete_scheduled_task(repo_id)
        .await
        .expect("Failed to delete task");

    assert_eq!(deleted_count, 1);

    // Verify task is gone
    let tasks = db.get_scheduled_tasks().await.unwrap();
    assert!(!tasks.iter().any(|t| t.repo_id == repo_id));
}

#[tokio::test]
async fn test_get_due_tasks() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    // Create a test repository and task
    let repo_id = db
        .add_repository("/test/repo4", "test_due_repo")
        .await
        .expect("Failed to add repository");

    let task_id = db
        .create_scheduled_task(repo_id, 1)
        .await
        .expect("Failed to create task");

    // Update the task to be due in the past
    let past_time = chrono::Utc::now() - chrono::Duration::hours(2);
    let next_run_past = chrono::Utc::now() - chrono::Duration::hours(1);
    db.update_task_execution(task_id, past_time, next_run_past)
        .await
        .expect("Failed to update task to be due");

    // Now check for due tasks - our task should be due
    let due_tasks = db.get_due_tasks().await.expect("Failed to get due tasks");

    // Should find our task since next_run_at is in the past
    assert!(due_tasks.iter().any(|t| t.id == task_id));
}

#[tokio::test]
async fn test_update_task_execution() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    // Create a test repository and task
    let repo_id = db
        .add_repository("/test/repo5", "test_update_repo")
        .await
        .expect("Failed to add repository");

    let task_id = db
        .create_scheduled_task(repo_id, 1)
        .await
        .expect("Failed to create task");

    // Update task execution times
    let now = chrono::Utc::now();
    let next_run = now + chrono::Duration::hours(1);

    db.update_task_execution(task_id, now, next_run)
        .await
        .expect("Failed to update task execution");

    // Verify the update
    let tasks = db.get_scheduled_tasks().await.unwrap();
    let task = tasks.iter().find(|t| t.id == task_id).unwrap();

    assert!(task.last_run_at.is_some());
    let last_run = task.last_run_at.unwrap();
    assert!((last_run - now).num_seconds().abs() < 2); // Within 2 seconds
}

#[tokio::test]
async fn test_get_repository() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    // Create a test repository
    let repo_id = db
        .add_repository("/test/get_repo", "test_get_repo")
        .await
        .expect("Failed to add repository");

    // Retrieve the repository
    let repo = db
        .get_repository(repo_id)
        .await
        .expect("Failed to get repository");

    assert!(repo.is_some());
    let repo = repo.unwrap();
    assert_eq!(repo.id, repo_id);
    assert_eq!(repo.path, "/test/get_repo");
    assert_eq!(repo.name, "test_get_repo");
}

#[tokio::test]
async fn test_get_repository_not_found() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    // Try to get a non-existent repository
    let repo = db
        .get_repository(999999)
        .await
        .expect("Failed to execute query");

    assert!(repo.is_none());
}

#[cfg(unix)]
#[test]
fn test_orchestrator_is_not_running() {
    use cudgel::orchestrator;

    // Stop any running orchestrator first
    let _ = orchestrator::stop_daemon();

    // Check that no orchestrator is running
    let status = orchestrator::is_running();
    assert!(status.is_ok());

    // Either None (not running) or Some(pid) that's stale
    // We just verify the function works without error
}

#[cfg(unix)]
#[test]
fn test_orchestrator_pid_file_location() {
    use cudgel::config::xdg_state_home;

    // Verify the PID file location follows XDG spec
    let expected_pid_dir = xdg_state_home().join("cudgel");
    let expected_pid_file = expected_pid_dir.join("orchestrator.pid");

    // Just verify the path construction works
    assert!(expected_pid_file.to_string_lossy().contains("cudgel"));
    assert!(expected_pid_file
        .to_string_lossy()
        .ends_with("orchestrator.pid"));
}

#[tokio::test]
async fn test_scheduled_task_validation() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    // Create a test repository
    let repo_id = db
        .add_repository("/test/validation", "test_validation")
        .await
        .expect("Failed to add repository");

    // Test invalid interval (0 hours) - should fail due to CHECK constraint
    let result = db.create_scheduled_task(repo_id, 0).await;
    assert!(result.is_err());

    // Test invalid interval (too large - > 8760 hours/1 year) - should fail
    let result = db.create_scheduled_task(repo_id, 10000).await;
    assert!(result.is_err());

    // Test valid intervals
    let result = db.create_scheduled_task(repo_id, 1).await;
    assert!(result.is_ok());

    let result = db.create_scheduled_task(repo_id, 8760).await; // 1 year
    assert!(result.is_ok());
}
