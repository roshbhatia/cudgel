// tests/test_graph_client.rs
//! Unit tests for knowledge graph client operations.

use cudgel::config::Config;
use cudgel::database::Database;
use cudgel::kg::{
    CodeEntity, Component, ComponentType, DependencyType, EntityMetadata, EntityType, KgClient,
    PostgresKgClient, Repository, Visibility,
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

// ============================================================================
// Additional Entity Operation Tests (T055)
// ============================================================================

/// T055a: Test find_entities_by_name
#[tokio::test]
async fn test_find_entities_by_name() {
    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    // Setup repository, component, and entities
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

    // Create entities with same name in different files
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
            metadata: EntityMetadata {
                signature: Some("fn parse_file(path: &str)".to_string()),
                doc_comment: None,
                language: "rust".to_string(),
            },
            summary: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
        CodeEntity {
            id: 0,
            component_id,
            name: "parse_file".to_string(),
            entity_type: EntityType::Function,
            file_path: "src/parser_v2.rs".to_string(),
            line_start: 15,
            line_end: 60,
            visibility: Visibility::Public,
            metadata: EntityMetadata {
                signature: Some("fn parse_file_v2(path: &str)".to_string()),
                doc_comment: None,
                language: "rust".to_string(),
            },
            summary: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    ];

    for entity in entities {
        client.create_entity(entity).await.unwrap();
    }

    // Find entities by name
    let found_entities = client
        .find_entities_by_name(&repo_id, "parse_file")
        .await
        .expect("Failed to find entities by name");

    assert_eq!(
        found_entities.len(),
        2,
        "Should find 2 entities with same name"
    );

    for entity in &found_entities {
        assert_eq!(entity.name, "parse_file");
        assert_eq!(entity.component_id, component_id);
    }
}

/// T055b: Test search_entities_by_name
#[tokio::test]
async fn test_search_entities_by_name() {
    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    // Setup repository, component, and entities
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

    // Create entities with similar names
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
            name: "parse_data".to_string(),
            entity_type: EntityType::Function,
            file_path: "src/parser.rs".to_string(),
            line_start: 60,
            line_end: 100,
            visibility: Visibility::Public,
            metadata: EntityMetadata::default(),
            summary: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    ];

    for entity in entities {
        client.create_entity(entity).await.unwrap();
    }

    // Search for entities with "parse" pattern
    let matches = client
        .search_entities_by_name(&repo_id, "parse", 0.5)
        .await
        .expect("Failed to search entities");

    assert!(
        matches.len() >= 2,
        "Should find at least 2 matching entities"
    );

    // Should find both "parse_file" and "parse_data"
    let names: Vec<String> = matches.iter().map(|m| m.entity.name.clone()).collect();
    assert!(names.contains(&"parse_file".to_string()));
    assert!(names.contains(&"parse_data".to_string()));

    // All matches should have confidence >= threshold
    for m in &matches {
        assert!(m.confidence >= 0.5, "Confidence should meet threshold");
    }
}

/// T055c: Test get_entities_by_file
#[tokio::test]
async fn test_get_entities_by_file() {
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

    // Create entities in different files
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
            name: "tokenize".to_string(),
            entity_type: EntityType::Function,
            file_path: "src/tokenizer.rs".to_string(),
            line_start: 20,
            line_end: 80,
            visibility: Visibility::Public,
            metadata: EntityMetadata::default(),
            summary: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    ];

    for entity in entities {
        client.create_entity(entity).await.unwrap();
    }

    // Get entities by file
    let parser_entities = client
        .get_entities_by_file(&repo_id, "src/parser.rs")
        .await
        .expect("Failed to get entities by file");

    assert_eq!(
        parser_entities.len(),
        1,
        "Should find 1 entity in parser.rs"
    );
    assert_eq!(parser_entities[0].name, "parse_file");
    assert_eq!(parser_entities[0].file_path, "src/parser.rs");

    let tokenizer_entities = client
        .get_entities_by_file(&repo_id, "src/tokenizer.rs")
        .await
        .expect("Failed to get entities by file");

    assert_eq!(
        tokenizer_entities.len(),
        1,
        "Should find 1 entity in tokenizer.rs"
    );
    assert_eq!(tokenizer_entities[0].name, "tokenize");
    assert_eq!(tokenizer_entities[0].file_path, "src/tokenizer.rs");
}

/// T055d: Test get_all_entity_names
#[tokio::test]
async fn test_get_all_entity_names() {
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

    // Create entities with different names
    let entity_names = vec!["parse_file", "tokenize", "analyze_ast"];
    for name in &entity_names {
        let entity = CodeEntity {
            id: 0,
            component_id,
            name: name.to_string(),
            entity_type: EntityType::Function,
            file_path: "src/parser.rs".to_string(),
            line_start: 10,
            line_end: 50,
            visibility: Visibility::Public,
            metadata: EntityMetadata::default(),
            summary: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        client.create_entity(entity).await.unwrap();
    }

    // Get all entity names
    let names = client
        .get_all_entity_names(&repo_id)
        .await
        .expect("Failed to get all entity names");

    assert_eq!(names.len(), 3, "Should find 3 entity names");

    for name in &entity_names {
        assert!(
            names.contains(&name.to_string()),
            "Should contain entity name: {}",
            name
        );
    }
}

/// T055e: Test update_entity_summary
#[tokio::test]
async fn test_update_entity_summary() {
    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    // Setup repository, component, and entity
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

    let entity = CodeEntity {
        id: 0,
        component_id,
        name: "parse_file".to_string(),
        entity_type: EntityType::Function,
        file_path: "src/parser.rs".to_string(),
        line_start: 10,
        line_end: 50,
        visibility: Visibility::Public,
        metadata: EntityMetadata::default(),
        summary: Some("Original summary".to_string()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let entity_id = client.create_entity(entity).await.unwrap();

    // Update entity summary
    let new_summary = "Updated summary with LLM-generated content".to_string();
    client
        .update_entity_summary(&entity_id, new_summary.clone())
        .await
        .expect("Failed to update entity summary");

    // Verify the update
    let updated_entity = client
        .get_entity(&entity_id)
        .await
        .expect("Failed to get entity")
        .expect("Entity should exist");

    assert_eq!(updated_entity.summary, Some(new_summary));
}

/// T055f: Test delete_entity_cascade
#[tokio::test]
async fn test_delete_entity_cascade() {
    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    // Setup repository, component, and entity
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

    let entity = CodeEntity {
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
    };

    let entity_id = client.create_entity(entity).await.unwrap();

    // Verify entity exists
    let before_delete = client
        .get_entity(&entity_id)
        .await
        .expect("Failed to get entity");

    assert!(before_delete.is_some(), "Entity should exist before delete");

    // Delete entity
    client
        .delete_entity_cascade(&entity_id)
        .await
        .expect("Failed to delete entity");

    // Verify entity is deleted
    let after_delete = client
        .get_entity(&entity_id)
        .await
        .expect("Failed to get entity");

    assert!(
        after_delete.is_none(),
        "Entity should not exist after delete"
    );
}

// ============================================================================
// User Story 2 Tests: Entity Relationship Discovery (T073-T082)
// ============================================================================

/// Helper to create test repository, component, and entities for relationship tests
async fn setup_relationship_test_data(
    client: &Arc<dyn KgClient>,
) -> (i32, i32, i32, i32) {

    // Create repository
    let repo = Repository {
        id: 0,
        path: "/test/relationships".to_string(),
        name: "relationships-test".to_string(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let repo_id = client.create_repository(repo).await.unwrap();

    // Create component
    let component = Component {
        id: 0,
        repository_id: repo_id,
        name: "core".to_string(),
        path: "src/core".to_string(),
        component_type: ComponentType::Module,
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let component_id = client.create_component(component).await.unwrap();

    // Create first entity (Parser)
    let entity1 = CodeEntity {
        id: 0,
        component_id,
        name: "Parser".to_string(),
        entity_type: EntityType::Struct,
        file_path: "src/core/parser.rs".to_string(),
        line_start: 10,
        line_end: 50,
        visibility: Visibility::Public,
        metadata: EntityMetadata::default(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let entity1_id = client.create_entity(entity1).await.unwrap();

    // Create second entity (Lexer)
    let entity2 = CodeEntity {
        id: 0,
        component_id,
        name: "Lexer".to_string(),
        entity_type: EntityType::Struct,
        file_path: "src/core/lexer.rs".to_string(),
        line_start: 5,
        line_end: 30,
        visibility: Visibility::Public,
        metadata: EntityMetadata::default(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let entity2_id = client.create_entity(entity2).await.unwrap();

    (repo_id, component_id, entity1_id, entity2_id)
}

/// T073: Test create_dependency relationship
#[tokio::test]
async fn test_create_dependency_relationship() {

    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    let (_, _, parser_id, lexer_id) = setup_relationship_test_data(&client).await;

    // Create dependency: Parser depends on Lexer
    let rel_id = client
        .create_dependency(&parser_id, &lexer_id, DependencyType::Import)
        .await
        .expect("Failed to create dependency");

    assert!(rel_id > 0, "Relationship ID should be positive");

    // Verify relationship exists by querying outgoing relationships
    let relationships = client
        .get_outgoing_relationships(&parser_id)
        .await
        .expect("Failed to get outgoing relationships");

    assert!(
        !relationships.dependencies.is_empty(),
        "Parser should have dependencies"
    );
    assert_eq!(
        relationships.dependencies[0].entity.id, lexer_id,
        "Parser should depend on Lexer"
    );
}

/// T074: Test create_uses relationship
#[tokio::test]
async fn test_create_uses_relationship() {
    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    let (_, _, parser_id, lexer_id) = setup_relationship_test_data(&client).await;

    // Create uses relationship: Parser uses Lexer
    let rel_id = client
        .create_uses(&parser_id, &lexer_id, "tokenization context".to_string())
        .await
        .expect("Failed to create uses relationship");

    assert!(rel_id > 0, "Relationship ID should be positive");

    // Verify relationship exists
    let relationships = client
        .get_outgoing_relationships(&parser_id)
        .await
        .expect("Failed to get outgoing relationships");

    assert!(
        !relationships.uses.is_empty(),
        "Parser should use other entities"
    );
}

/// T075: Test create_calls relationship
#[tokio::test]
async fn test_create_calls_relationship() {
    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    let (_, component_id, _, _) = setup_relationship_test_data(&client).await;

    // Create two function entities
    let caller = CodeEntity {
        id: 0,
        component_id,
        name: "parse".to_string(),
        entity_type: EntityType::Function,
        file_path: "src/core/parser.rs".to_string(),
        line_start: 100,
        line_end: 150,
        visibility: Visibility::Public,
        metadata: EntityMetadata::default(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let caller_id = client.create_entity(caller).await.unwrap();

    let callee = CodeEntity {
        id: 0,
        component_id,
        name: "tokenize".to_string(),
        entity_type: EntityType::Function,
        file_path: "src/core/lexer.rs".to_string(),
        line_start: 50,
        line_end: 80,
        visibility: Visibility::Public,
        metadata: EntityMetadata::default(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let callee_id = client.create_entity(callee).await.unwrap();

    // Create calls relationship
    let rel_id = client
        .create_calls(&caller_id, &callee_id, 5)
        .await
        .expect("Failed to create calls relationship");

    assert!(rel_id > 0, "Relationship ID should be positive");

    // Verify relationship exists
    let relationships = client
        .get_outgoing_relationships(&caller_id)
        .await
        .expect("Failed to get outgoing relationships");

    assert!(
        !relationships.calls.is_empty(),
        "Caller function should have calls"
    );
}

/// T076: Test create_implements relationship
#[tokio::test]
async fn test_create_implements_relationship() {
    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    let (_, component_id, _, _) = setup_relationship_test_data(&client).await;

    // Create trait entity
    let trait_entity = CodeEntity {
        id: 0,
        component_id,
        name: "Parseable".to_string(),
        entity_type: EntityType::Trait,
        file_path: "src/core/traits.rs".to_string(),
        line_start: 10,
        line_end: 20,
        visibility: Visibility::Public,
        metadata: EntityMetadata::default(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let trait_id = client.create_entity(trait_entity).await.unwrap();

    // Create struct entity
    let struct_entity = CodeEntity {
        id: 0,
        component_id,
        name: "JsonParser".to_string(),
        entity_type: EntityType::Struct,
        file_path: "src/core/json_parser.rs".to_string(),
        line_start: 30,
        line_end: 100,
        visibility: Visibility::Public,
        metadata: EntityMetadata::default(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let struct_id = client.create_entity(struct_entity).await.unwrap();

    // Create implements relationship
    let rel_id = client
        .create_implements(&struct_id, &trait_id)
        .await
        .expect("Failed to create implements relationship");

    assert!(rel_id > 0, "Relationship ID should be positive");

    // Verify relationship exists
    let relationships = client
        .get_outgoing_relationships(&struct_id)
        .await
        .expect("Failed to get outgoing relationships");

    assert!(
        !relationships.implements.is_empty(),
        "Struct should implement trait"
    );
}

/// T077: Test get_outgoing_relationships
#[tokio::test]
async fn test_get_outgoing_relationships() {

    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    let (_, _, parser_id, lexer_id) = setup_relationship_test_data(&client).await;

    // Create dependency
    client
        .create_dependency(&parser_id, &lexer_id, DependencyType::Import)
        .await
        .unwrap();

    // Create uses
    client
        .create_uses(&parser_id, &lexer_id, "test context".to_string())
        .await
        .unwrap();

    // Get outgoing relationships
    let relationships = client
        .get_outgoing_relationships(&parser_id)
        .await
        .expect("Failed to get outgoing relationships");

    assert!(
        !relationships.dependencies.is_empty(),
        "Should have dependency relationships"
    );
    assert!(
        !relationships.uses.is_empty(),
        "Should have uses relationships"
    );
}

/// T078: Test get_incoming_relationships
#[tokio::test]
async fn test_get_incoming_relationships() {

    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    let (_, _, parser_id, lexer_id) = setup_relationship_test_data(&client).await;

    // Create dependency: Parser depends on Lexer
    client
        .create_dependency(&parser_id, &lexer_id, DependencyType::Import)
        .await
        .unwrap();

    // Get incoming relationships for Lexer (what depends on it)
    let relationships = client
        .get_incoming_relationships(&lexer_id)
        .await
        .expect("Failed to get incoming relationships");

    assert!(
        !relationships.dependents.is_empty(),
        "Lexer should have dependents"
    );
    assert_eq!(
        relationships.dependents[0].entity.id, parser_id,
        "Parser should be a dependent of Lexer"
    );
}

/// T079: Test get_all_relationships
#[tokio::test]
async fn test_get_all_relationships() {

    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    let (_, component_id, parser_id, lexer_id) = setup_relationship_test_data(&client).await;

    // Create third entity
    let analyzer = CodeEntity {
        id: 0,
        component_id,
        name: "Analyzer".to_string(),
        entity_type: EntityType::Struct,
        file_path: "src/core/analyzer.rs".to_string(),
        line_start: 10,
        line_end: 100,
        visibility: Visibility::Public,
        metadata: EntityMetadata::default(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let analyzer_id = client.create_entity(analyzer).await.unwrap();

    // Parser depends on Lexer (outgoing)
    client
        .create_dependency(&parser_id, &lexer_id, DependencyType::Import)
        .await
        .unwrap();

    // Analyzer depends on Parser (incoming)
    client
        .create_dependency(&analyzer_id, &parser_id, DependencyType::Import)
        .await
        .unwrap();

    // Get all relationships for Parser
    let relationships = client
        .get_all_relationships(&parser_id)
        .await
        .expect("Failed to get all relationships");

    assert!(
        !relationships.dependencies.is_empty(),
        "Should have outgoing dependencies"
    );
    assert!(
        !relationships.dependents.is_empty(),
        "Should have incoming dependents"
    );
}

/// T080: Test traverse_dependencies with multi-hop
#[tokio::test]
async fn test_traverse_dependencies_multi_hop() {

    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    let (_, component_id, _, _) = setup_relationship_test_data(&client).await;

    // Create chain: A -> B -> C -> D
    let entity_a = CodeEntity {
        id: 0,
        component_id,
        name: "EntityA".to_string(),
        entity_type: EntityType::Struct,
        file_path: "src/core/a.rs".to_string(),
        line_start: 1,
        line_end: 10,
        visibility: Visibility::Public,
        metadata: EntityMetadata::default(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let a_id = client.create_entity(entity_a).await.unwrap();

    let entity_b = CodeEntity {
        id: 0,
        component_id,
        name: "EntityB".to_string(),
        entity_type: EntityType::Struct,
        file_path: "src/core/b.rs".to_string(),
        line_start: 1,
        line_end: 10,
        visibility: Visibility::Public,
        metadata: EntityMetadata::default(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let b_id = client.create_entity(entity_b).await.unwrap();

    let entity_c = CodeEntity {
        id: 0,
        component_id,
        name: "EntityC".to_string(),
        entity_type: EntityType::Struct,
        file_path: "src/core/c.rs".to_string(),
        line_start: 1,
        line_end: 10,
        visibility: Visibility::Public,
        metadata: EntityMetadata::default(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let c_id = client.create_entity(entity_c).await.unwrap();

    let entity_d = CodeEntity {
        id: 0,
        component_id,
        name: "EntityD".to_string(),
        entity_type: EntityType::Struct,
        file_path: "src/core/d.rs".to_string(),
        line_start: 1,
        line_end: 10,
        visibility: Visibility::Public,
        metadata: EntityMetadata::default(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let d_id = client.create_entity(entity_d).await.unwrap();

    // Create dependency chain
    client
        .create_dependency(&a_id, &b_id, DependencyType::Import)
        .await
        .unwrap();
    client
        .create_dependency(&b_id, &c_id, DependencyType::Import)
        .await
        .unwrap();
    client
        .create_dependency(&c_id, &d_id, DependencyType::Import)
        .await
        .unwrap();

    // Traverse from A with max_depth = 3
    let dependencies = client
        .traverse_dependencies(&a_id, 3)
        .await
        .expect("Failed to traverse dependencies");

    // Should get B, C, D
    assert!(
        dependencies.len() >= 3,
        "Should traverse at least 3 hops: found {}",
        dependencies.len()
    );

    let names: Vec<String> = dependencies.iter().map(|e| e.name.clone()).collect();
    assert!(names.contains(&"EntityB".to_string()), "Should find EntityB");
    assert!(names.contains(&"EntityC".to_string()), "Should find EntityC");
    assert!(names.contains(&"EntityD".to_string()), "Should find EntityD");
}

// ============================================================================
// User Story 2 Integration Tests (T081-T082)
// ============================================================================

/// T081: Test query parser relationship intent extraction
#[tokio::test]
async fn test_query_parser_relationship_intent() {
    use cudgel::kg::{QueryIntent, QueryParser};

    let parser = QueryParser::new();

    // Test various query patterns
    let test_cases = vec![
        ("what does Parser depend on", QueryIntent::Dependencies { entity_name: "Parser".to_string() }),
        ("dependencies of Config", QueryIntent::Dependencies { entity_name: "Config".to_string() }),
        ("what depends on Database", QueryIntent::Dependents { entity_name: "Database".to_string() }),
        ("what does Indexer use", QueryIntent::Uses { entity_name: "Indexer".to_string() }),
        ("what uses Parser", QueryIntent::UsedBy { entity_name: "Parser".to_string() }),
        ("what does main call", QueryIntent::Calls { entity_name: "main".to_string() }),
        ("what calls parse_file", QueryIntent::CalledBy { entity_name: "parse_file".to_string() }),
        ("what does HttpClient implement", QueryIntent::Implements { entity_name: "HttpClient".to_string() }),
        ("what implements Serializable", QueryIntent::ImplementedBy { entity_name: "Serializable".to_string() }),
        ("what does Parser interact with", QueryIntent::AllRelationships { entity_name: "Parser".to_string() }),
        ("relationships of Config", QueryIntent::AllRelationships { entity_name: "Config".to_string() }),
    ];

    for (query, expected_intent) in test_cases {
        let intent = parser.parse(query).unwrap_or_else(|_| panic!("Failed to parse query: {}", query));
        assert_eq!(
            intent, expected_intent,
            "Query '{}' should produce {:?}",
            query, expected_intent
        );
    }

    // Test invalid queries
    let invalid_queries = vec![
        "",
        "random nonsense",
        "hello world",
    ];

    for query in invalid_queries {
        let result = parser.parse(query);
        assert!(
            result.is_err(),
            "Query '{}' should fail to parse",
            query
        );
    }
}

/// T082: Test fuzzy entity matching with EntityMatcher
#[tokio::test]
async fn test_fuzzy_entity_matching() {
    use cudgel::kg::EntityMatcher;

    let client = match setup_test_graph_client().await {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: PostgreSQL not available");
            return;
        }
    };

    // Create test repository
    let repo = Repository {
        id: 0,
        path: "/test/fuzzy".to_string(),
        name: "test-fuzzy".to_string(),
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let repo_id = client.create_repository(repo).await.unwrap();

    // Create test component
    let component = Component {
        id: 0,
        repository_id: repo_id,
        name: "test-component".to_string(),
        path: "src".to_string(),
        component_type: ComponentType::Module,
        summary: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let component_id = client.create_component(component).await.unwrap();

    // Create test entities with similar names
    let entities = vec![
        "HttpParser",
        "HttpParserConfig",
        "JsonParser",
        "XmlParser",
        "DatabaseConnection",
    ];

    for name in entities {
        let entity = CodeEntity {
            id: 0,
            component_id,
            name: name.to_string(),
            entity_type: EntityType::Class,
            file_path: format!("src/{}.rs", name.to_lowercase()),
            line_start: 1,
            line_end: 100,
            visibility: Visibility::Public,
            metadata: EntityMetadata::default(),
            summary: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        client.create_entity(entity).await.unwrap();
    }

    // Test fuzzy matching
    let matcher = EntityMatcher::new();
    
    // Exact match
    let matches = matcher.find_entities_by_name(&*client, repo_id, "HttpParser").await.unwrap();
    assert!(!matches.is_empty(), "Should find exact match for HttpParser");
    assert_eq!(matches[0].entity.name, "HttpParser");
    assert_eq!(matches[0].confidence, 1.0, "Exact match should have confidence 1.0");

    // Case insensitive
    let matches = matcher.find_entities_by_name(&*client, repo_id, "httpparser").await.unwrap();
    assert!(!matches.is_empty(), "Should find case-insensitive match");
    assert_eq!(matches[0].entity.name, "HttpParser");

    // No match for completely different name
    let matches = matcher.find_entities_by_name(&*client, repo_id, "CompletelyDifferentThing").await.unwrap();
    assert!(matches.is_empty() || matches[0].confidence < 0.85, "Should not match unrelated names with high confidence");

    // Custom threshold
    let matcher_loose = EntityMatcher::with_threshold(0.5).unwrap();
    let matches = matcher_loose.find_entities_by_name(&*client, repo_id, "Parse").await.unwrap();
    // With lower threshold, we should find matches containing "Parser"
    assert!(!matches.is_empty(), "Lower threshold should find partial matches");
}
