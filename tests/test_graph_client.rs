// tests/test_graph_client.rs
//! Unit tests for knowledge graph client operations.

use cudgel::config::Config;
use cudgel::database::Database;
use cudgel::kg::{
    CodeEntity, Component, ComponentType, EntityMetadata, EntityType, KgClient, PostgresKgClient,
    Repository, Visibility,
};
use std::sync::Arc;

/// Helper to check if PostgreSQL is available for testing
async fn is_postgres_available() -> bool {
    let config = Config::local().expect("Config should be valid");
    match Database::new(&config).await {
        Ok(db) => db.health_check().await.unwrap_or(false),
        Err(_) => false,
    }
}

/// Setup a test graph client with PostgreSQL database
///
/// Returns `Some(Arc<dyn KgClient>)` if setup succeeds, `None` if PostgreSQL is unavailable.
/// Skips tests gracefully when database is not available.
pub async fn setup_test_graph_client() -> Option<Arc<dyn KgClient>> {
    if !is_postgres_available().await {
        eprintln!("PostgreSQL unavailable, skipping test");
        return None;
    }

    let config = Config::local().expect("Config should be valid");
    let db = Database::new(&config).await.ok()?;

    // Initialize main schema
    db.init_schema().await.ok()?;

    // Initialize KG schema
    db.init_kg_schema().await.ok()?;

    let client = PostgresKgClient::new(Arc::new(db));
    Some(Arc::new(client))
}

#[tokio::test]
async fn test_setup_graph_client() {
    let client = setup_test_graph_client().await;

    if let Some(client) = client {
        // Verify schema is initialized
        assert!(
            client.is_schema_initialized().await.unwrap(),
            "Schema should be initialized after setup"
        );
    } else {
        eprintln!("Skipping test: PostgreSQL not available");
    }
}

// ============================================================================
// User Story 1 Tests (T029-T034)
// ============================================================================

/// T029: Test create and get repository
#[tokio::test]
async fn test_create_and_get_repository() {
    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    // Create a repository (without ID and timestamps)
    let repo = Repository {
        id: 0, // Will be ignored by create
        path: "/test/repo".to_string(),
        name: "test-repo".to_string(),
        summary: Some("A test repository".to_string()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Create repository
    let repo_id = client
        .create_repository(repo.clone())
        .await
        .expect("Failed to create repository");

    assert!(repo_id > 0, "Repository ID should be positive");

    // Get repository by path
    let retrieved = client
        .get_repository_by_path(&repo.path)
        .await
        .expect("Failed to get repository")
        .expect("Repository should exist");

    assert_eq!(retrieved.path, repo.path);
    assert_eq!(retrieved.name, repo.name);
    assert_eq!(retrieved.summary, repo.summary);
    assert_eq!(retrieved.id, repo_id);
}

/// T030: Test create and get component
#[tokio::test]
async fn test_create_and_get_component() {
    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    // Create a repository first
    let repo = Repository {
        id: 0,
        path: "/test/repo".to_string(),
        name: "test-repo".to_string(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let repo_id = client
        .create_repository(repo)
        .await
        .expect("Failed to create repository");

    // Create a component
    let component = Component {
        id: 0,
        repository_id: repo_id,
        name: "parser".to_string(),
        path: "src/parser".to_string(),
        component_type: ComponentType::Module,
        summary: Some("Parser module".to_string()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let component_id = client
        .create_component(component.clone())
        .await
        .expect("Failed to create component");

    assert!(component_id > 0, "Component ID should be positive");

    // Get components by repository
    let components = client
        .get_components(&repo_id)
        .await
        .expect("Failed to get components");

    assert_eq!(components.len(), 1);
    assert_eq!(components[0].name, component.name);
    assert_eq!(components[0].path, component.path);
    assert_eq!(components[0].component_type, component.component_type);
    assert_eq!(components[0].id, component_id);
}

/// T031: Test create and get entity
#[tokio::test]
async fn test_create_and_get_entity() {
    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    // Create repository and component first
    let repo = Repository {
        id: 0,
        path: "/test/repo".to_string(),
        name: "test-repo".to_string(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let repo_id = client.create_repository(repo).await.unwrap();

    let component = Component {
        id: 0,
        repository_id: repo_id,
        name: "parser".to_string(),
        path: "src/parser".to_string(),
        component_type: ComponentType::Module,
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let component_id = client.create_component(component).await.unwrap();

    // Create an entity
    let entity = CodeEntity {
        id: 0,
        component_id,
        name: "parse_file".to_string(),
        entity_type: EntityType::Function,
        file_path: "src/parser.rs".to_string(),
        line_start: 10,
        line_end: 50,
        visibility: Visibility::Public,
        metadata: EntityMetadata {
            signature: Some("fn parse_file(path: &str) -> Result<AST>".to_string()),
            doc_comment: Some("Parses a file into an AST".to_string()),
            language: "rust".to_string(),
        },
        summary: Some("Main parsing function".to_string()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let entity_id = client
        .create_entity(entity.clone())
        .await
        .expect("Failed to create entity");

    assert!(entity_id > 0, "Entity ID should be positive");

    // Get entity by ID
    let retrieved = client
        .get_entity(&entity_id)
        .await
        .expect("Failed to get entity")
        .expect("Entity should exist");

    assert_eq!(retrieved.name, entity.name);
    assert_eq!(retrieved.entity_type, entity.entity_type);
    assert_eq!(retrieved.file_path, entity.file_path);
    assert_eq!(retrieved.line_start, entity.line_start);
    assert_eq!(retrieved.line_end, entity.line_end);
    assert_eq!(retrieved.visibility, entity.visibility);
    assert_eq!(retrieved.id, entity_id);
}

/// T032: Test create entities batch
#[tokio::test]
async fn test_create_entities_batch() {
    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    // Setup repository and component
    let repo = Repository {
        id: 0,
        path: "/test/repo".to_string(),
        name: "test-repo".to_string(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let repo_id = client.create_repository(repo).await.unwrap();

    let component = Component {
        id: 0,
        repository_id: repo_id,
        name: "parser".to_string(),
        path: "src/parser".to_string(),
        component_type: ComponentType::Module,
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let component_id = client.create_component(component).await.unwrap();

    // Create multiple entities
    let entities = vec![
        CodeEntity {
            id: 0,
            component_id,
            name: "parse_file".to_string(),
            entity_type: EntityType::Function,
            file_path: "src/parser.rs".to_string(),
            line_start: 10,
            line_end: 50,
            visibility: Visibility::Public,
            metadata: EntityMetadata::default(),
            summary: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
        CodeEntity {
            id: 0,
            component_id,
            name: "Parser".to_string(),
            entity_type: EntityType::Struct,
            file_path: "src/parser.rs".to_string(),
            line_start: 60,
            line_end: 100,
            visibility: Visibility::Public,
            metadata: EntityMetadata::default(),
            summary: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
        CodeEntity {
            id: 0,
            component_id,
            name: "tokenize".to_string(),
            entity_type: EntityType::Function,
            file_path: "src/parser.rs".to_string(),
            line_start: 110,
            line_end: 150,
            visibility: Visibility::Private,
            metadata: EntityMetadata::default(),
            summary: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    ];

    let entity_ids = client
        .create_entities_batch(entities.clone())
        .await
        .expect("Failed to create entities batch");

    assert_eq!(
        entity_ids.len(),
        3,
        "Should return 3 entity IDs for 3 entities"
    );

    // Verify all IDs are positive
    for id in &entity_ids {
        assert!(*id > 0, "All entity IDs should be positive");
    }

    // Verify entities can be retrieved
    for id in &entity_ids {
        let entity = client
            .get_entity(id)
            .await
            .expect("Failed to get entity")
            .expect("Entity should exist");
        assert!(
            entities.iter().any(|e| e.name == entity.name),
            "Entity should match one of the created entities"
        );
    }
}

/// T033: Test update repository summary
#[tokio::test]
async fn test_update_repository_summary() {
    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    // Create a repository
    let repo = Repository {
        id: 0,
        path: "/test/repo".to_string(),
        name: "test-repo".to_string(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let repo_id = client.create_repository(repo).await.unwrap();

    // Update summary
    let new_summary = "An updated repository summary with detailed architecture info".to_string();

    client
        .update_repository_summary(&repo_id, new_summary.clone())
        .await
        .expect("Failed to update repository summary");

    // Verify update
    let retrieved = client
        .get_repository_by_path("/test/repo")
        .await
        .expect("Failed to get repository")
        .expect("Repository should exist");

    assert_eq!(retrieved.summary, Some(new_summary));
}

/// T034: Test get components
#[tokio::test]
async fn test_get_components() {
    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    // Create a repository
    let repo = Repository {
        id: 0,
        path: "/test/repo".to_string(),
        name: "test-repo".to_string(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let repo_id = client.create_repository(repo).await.unwrap();

    // Create multiple components
    let components_data = vec![
        ("parser", "src/parser", ComponentType::Module),
        ("indexer", "src/indexer", ComponentType::Module),
        ("database", "src/database", ComponentType::Module),
    ];

    for (name, path, comp_type) in components_data {
        let component = Component {
            id: 0,
            repository_id: repo_id,
            name: name.to_string(),
            path: path.to_string(),
            component_type: comp_type,
            summary: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        client
            .create_component(component)
            .await
            .expect("Failed to create component");
    }

    // Get all components
    let components = client
        .get_components(&repo_id)
        .await
        .expect("Failed to get components");

    assert_eq!(components.len(), 3, "Should have 3 components");

    // Verify component names
    let names: Vec<String> = components.iter().map(|c| c.name.clone()).collect();
    assert!(names.contains(&"parser".to_string()));
    assert!(names.contains(&"indexer".to_string()));
    assert!(names.contains(&"database".to_string()));
}
