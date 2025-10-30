//! Integration tests for Cudgel

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
    let config = Config::from_env().unwrap_or_default();
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

    let config = Config::from_env().unwrap_or_default();
    let db = Database::new(&config).await.ok()?;

    // Initialize schema
    db.init_schema().await.ok()?;

    Some(Arc::new(db))
}

/// Create a temporary test repository with sample files
fn create_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

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
    let config = Config::from_env();
    assert!(config.is_ok());

    // Config should have default values
    let config = config.unwrap();
    assert_eq!(
        config.database.host,
        std::env::var("CUDGEL_DB_HOST").unwrap_or_else(|_| "localhost".to_string())
    );
}

#[test]
fn test_embedding_generation() {
    let config = Arc::new(Config::from_env().unwrap_or_default());
    let embedder = EmbeddingGenerator::new(config).unwrap();

    let embedding = embedder.encode("test text");
    assert!(embedding.is_ok());

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

    let config = Arc::new(Config::from_env().unwrap_or_default());
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

    let config = Arc::new(Config::from_env().unwrap_or_default());
    let temp_repo = create_test_repo();

    // Index the repository
    let mut indexer = Indexer::new(config.clone(), db.clone()).unwrap();
    let (_, _) = indexer.index_repository(temp_repo.path()).await.unwrap();

    // Query for symbols
    let query_engine = QueryEngine::new(config.clone(), db.clone()).unwrap();
    let results = query_engine.search_symbols("hello", 10, None).await;

    assert!(results.is_ok());
    let results = results.unwrap();

    // We should find some symbols (with dummy embeddings, results might not be perfect)
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_graph_query() {
    let db = match setup_test_db().await {
        Some(db) => db,
        None => return,
    };

    let config = Arc::new(Config::from_env().unwrap_or_default());
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
