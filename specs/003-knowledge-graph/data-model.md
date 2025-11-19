# Data Model: Knowledge Graph

**Feature**: 003-knowledge-graph  
**Date**: 2025-11-19

## Overview

This document defines the graph data model for storing code entities and their relationships in SurrealDB. The model supports repository-level architecture understanding, entity relationship discovery, and natural language querying.

---

## Graph Schema

### Node Types

#### 1. Repository Node

Represents the entire codebase being indexed.

**Schema**:
```rust
pub struct Repository {
    pub id: RecordId,              // SurrealDB ID: repository:repo_id
    pub name: String,              // Repository name
    pub path: String,              // Absolute file system path
    pub languages: Vec<String>,    // Detected languages (e.g., ["Rust", "Python"])
    pub summary: Option<String>,   // LLM-generated architecture summary
    pub indexed_at: DateTime<Utc>, // Last full index timestamp
    pub file_count: usize,         // Total files indexed
    pub entity_count: usize,       // Total entities extracted
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Validation Rules**:
- `name`: Required, 1-255 characters
- `path`: Required, must be valid absolute path
- `languages`: At least one language
- `file_count`: >= 0
- `entity_count`: >= 0

**Indexes**:
- Primary: `id`
- Unique: `path`

---

#### 2. Component Node

Represents high-level architectural units (modules, services, layers).

**Schema**:
```rust
pub struct Component {
    pub id: RecordId,                // SurrealDB ID: component:component_id
    pub name: String,                // Component name (e.g., "parser", "api-gateway")
    pub component_type: ComponentType, // Type of component
    pub repository_id: RecordId,     // Foreign key to repository
    pub summary: Option<String>,     // LLM-generated purpose summary
    pub file_paths: Vec<String>,     // Files belonging to this component
    pub entity_count: usize,         // Number of entities in component
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum ComponentType {
    Module,      // Language-specific module/package
    Service,     // Microservice or major service component
    Layer,       // Architectural layer (e.g., data, business, presentation)
    Library,     // External or internal library
}
```

**Validation Rules**:
- `name`: Required, 1-255 characters
- `component_type`: Required, must be valid enum value
- `repository_id`: Required, must reference existing repository
- `file_paths`: Non-empty list
- `entity_count`: >= 0

**Indexes**:
- Primary: `id`
- Index: `repository_id`
- Index: `name` (for fuzzy search)

---

#### 3. CodeEntity Node

Represents specific code elements (classes, functions, interfaces).

**Schema**:
```rust
pub struct CodeEntity {
    pub id: RecordId,               // SurrealDB ID: code_entity:entity_id
    pub name: String,               // Entity name (e.g., "Parser", "parse_function")
    pub entity_type: EntityType,    // Type of code entity
    pub file_path: String,          // File containing this entity
    pub line_start: usize,          // Starting line number
    pub line_end: usize,            // Ending line number
    pub component_id: Option<RecordId>, // Optional component membership
    pub repository_id: RecordId,    // Foreign key to repository
    pub summary: Option<String>,    // LLM-generated description
    pub visibility: Visibility,     // Public, private, internal, etc.
    pub metadata: EntityMetadata,   // Additional type-specific metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum EntityType {
    Class,
    Interface,
    Trait,
    Struct,
    Enum,
    Function,
    Method,
    Variable,
    Constant,
    Module,
}

pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

pub struct EntityMetadata {
    pub language: String,           // Source language
    pub signature: Option<String>,  // Function/method signature
    pub is_async: bool,             // For async functions
    pub is_test: bool,              // Test function/class
    pub dependencies_count: usize,  // Number of direct dependencies
}
```

**Validation Rules**:
- `name`: Required, 1-255 characters
- `entity_type`: Required, must be valid enum value
- `file_path`: Required, must be relative to repository root
- `line_start`: >= 1, must be <= `line_end`
- `line_end`: >= `line_start`
- `repository_id`: Required, must reference existing repository
- `visibility`: Required, must be valid enum value

**Indexes**:
- Primary: `id`
- Index: `repository_id`
- Index: `component_id`
- Index: `name` (for fuzzy search)
- Index: `file_path`
- Composite: `(repository_id, name)` (for disambiguation)

---

### Edge Types (Relationships)

#### 1. DEPENDS_ON

Represents dependency relationship between entities.

**Schema**:
```rust
pub struct DependsOn {
    pub id: RecordId,
    pub from: RecordId,             // Source entity
    pub to: RecordId,               // Target entity
    pub dependency_type: DependencyType,
    pub is_direct: bool,            // Direct vs transitive dependency
    pub created_at: DateTime<Utc>,
}

pub enum DependencyType {
    Import,      // Import/use statement
    Inheritance, // Class/trait inheritance
    Composition, // Has-a relationship
    Call,        // Function/method call
}
```

**Constraints**:
- `from`: Must reference existing code_entity or component
- `to`: Must reference existing code_entity or component
- No self-loops: `from` != `to`

---

#### 2. USES

Represents usage relationship (less strong than dependency).

**Schema**:
```rust
pub struct Uses {
    pub id: RecordId,
    pub from: RecordId,             // Source entity
    pub to: RecordId,               // Target entity
    pub context: String,            // Where/how it's used
    pub frequency: usize,           // Number of usage occurrences
    pub created_at: DateTime<Utc>,
}
```

**Constraints**:
- `from`: Must reference existing code_entity
- `to`: Must reference existing code_entity
- `frequency`: >= 1

---

#### 3. CONTAINS

Represents containment relationship (component contains entities).

**Schema**:
```rust
pub struct Contains {
    pub id: RecordId,
    pub from: RecordId,             // Container (component or repository)
    pub to: RecordId,               // Contained (entity or component)
    pub created_at: DateTime<Utc>,
}
```

**Constraints**:
- `from`: Must be component or repository
- `to`: Must be code_entity or component (child of parent)
- No cycles: Containment must form a DAG

---

#### 4. IMPLEMENTS

Represents interface/trait implementation.

**Schema**:
```rust
pub struct Implements {
    pub id: RecordId,
    pub from: RecordId,             // Implementing entity
    pub to: RecordId,               // Interface/trait
    pub created_at: DateTime<Utc>,
}
```

**Constraints**:
- `from`: Must be code_entity with type Class/Struct
- `to`: Must be code_entity with type Interface/Trait

---

#### 5. CALLS

Represents function/method invocation.

**Schema**:
```rust
pub struct Calls {
    pub id: RecordId,
    pub from: RecordId,             // Caller
    pub to: RecordId,               // Callee
    pub call_count: usize,          // Number of call sites
    pub is_recursive: bool,         // Self-recursive call
    pub created_at: DateTime<Utc>,
}
```

**Constraints**:
- `from`: Must be code_entity with type Function/Method
- `to`: Must be code_entity with type Function/Method
- `call_count`: >= 1

---

## Query Patterns

### 1. Get Repository Architecture Summary

```surrealql
SELECT 
    name, 
    summary, 
    languages, 
    file_count,
    entity_count
FROM repository
WHERE path = $repo_path
```

---

### 2. List All Components

```surrealql
SELECT 
    name, 
    component_type, 
    summary, 
    entity_count
FROM component
WHERE repository_id = $repo_id
ORDER BY entity_count DESC
```

---

### 3. Find Entity by Name (Exact)

```surrealql
SELECT * FROM code_entity
WHERE repository_id = $repo_id 
  AND name = $entity_name
```

---

### 4. Find Entity Relationships

```surrealql
-- Outgoing relationships (what entity depends on)
SELECT 
    ->depends_on->code_entity.* AS dependencies,
    ->uses->code_entity.* AS uses,
    ->calls->code_entity.* AS calls
FROM code_entity
WHERE id = $entity_id

-- Incoming relationships (what depends on entity)
SELECT 
    <-depends_on<-code_entity.* AS dependents,
    <-uses<-code_entity.* AS used_by,
    <-calls<-code_entity.* AS callers
FROM code_entity
WHERE id = $entity_id
```

---

### 5. Traverse Multi-Hop Dependencies

```surrealql
-- Get all transitive dependencies (up to 3 hops)
SELECT * FROM (
    SELECT * FROM code_entity 
    WHERE id = $entity_id
    FETCH ->depends_on->code_entity->depends_on->code_entity->depends_on->code_entity
)
```

---

### 6. Find Entities by Pattern

```surrealql
-- Cross-cutting concern analysis (e.g., error handling)
SELECT * FROM code_entity
WHERE name CONTAINS 'error' 
   OR name CONTAINS 'exception'
   OR summary CONTAINS 'error handling'
ORDER BY entity_type, name
```

---

## State Transitions

### Entity Lifecycle

```
[New File Detected]
       ↓
[Parse & Extract] → Create CodeEntity nodes
       ↓
[Analyze Dependencies] → Create relationship edges
       ↓
[Generate Summary] → Update CodeEntity.summary (async)
       ↓
[Active] ← State during normal operations
       ↓
[File Modified Detected]
       ↓
[Delete Old Entity] → Cascade delete relationships
       ↓
[Re-Parse & Extract] → (back to Create CodeEntity)
       ↓
[File Deleted Detected]
       ↓
[Delete Entity] → Cascade delete relationships
       ↓
[Removed]
```

---

## Data Consistency Rules

### 1. Referential Integrity

- All `repository_id` references must point to existing repository nodes
- All `component_id` references must point to existing component nodes
- All relationship edges must reference existing nodes

**Implementation**: SurrealDB record links with validation

---

### 2. Cascade Deletion

When a node is deleted, all connected edges must be deleted:

- Delete repository → delete all components and entities
- Delete component → delete all contained entities
- Delete entity → delete all relationships (both incoming and outgoing)

**Implementation**: SurrealDB `DELETE` with `CASCADE`

---

### 3. Summary Consistency

- Summaries may be `None` initially (async generation)
- Summary generation failures do not block entity creation
- Stale summaries are marked with `updated_at` timestamp

**Implementation**: Background task queue for summary generation

---

## Storage Estimates

### Node Storage

| Node Type | Avg Size | Count (50k files) | Total |
|-----------|----------|-------------------|-------|
| Repository | 1 KB | 1 | 1 KB |
| Component | 2 KB | ~500 | 1 MB |
| CodeEntity | 1 KB | ~100,000 | 100 MB |

### Edge Storage

| Edge Type | Avg Size | Count (50k files) | Total |
|-----------|----------|-------------------|-------|
| DEPENDS_ON | 200 bytes | ~200,000 | 40 MB |
| USES | 200 bytes | ~150,000 | 30 MB |
| CONTAINS | 100 bytes | ~100,500 | 10 MB |
| IMPLEMENTS | 100 bytes | ~20,000 | 2 MB |
| CALLS | 200 bytes | ~30,000 | 6 MB |

**Total Estimated Storage**: ~190 MB for 50k file repository

**With Indexes**: ~250-300 MB total

---

## Performance Considerations

### 1. Indexing Strategy

- Name indexes for fuzzy matching queries
- File path indexes for incremental updates
- Composite indexes for common query patterns

### 2. Query Optimization

- Limit traversal depth to 3 hops (prevent excessive fan-out)
- Use `FETCH` for eager loading of relationships
- Add timeouts to prevent runaway queries (3 second limit)

### 3. Write Optimization

- Batch entity creation (100 entities per transaction)
- Parallel relationship edge creation
- Async summary generation (don't block indexing)

---

## Migration Strategy

### Initial Schema Creation

```rust
pub async fn initialize_schema(db: &Surreal<Client>) -> Result<()> {
    // Define node tables
    db.query("DEFINE TABLE repository SCHEMAFULL").await?;
    db.query("DEFINE TABLE component SCHEMAFULL").await?;
    db.query("DEFINE TABLE code_entity SCHEMAFULL").await?;
    
    // Define relationship tables
    db.query("DEFINE TABLE depends_on TYPE RELATION").await?;
    db.query("DEFINE TABLE uses TYPE RELATION").await?;
    db.query("DEFINE TABLE contains TYPE RELATION").await?;
    db.query("DEFINE TABLE implements TYPE RELATION").await?;
    db.query("DEFINE TABLE calls TYPE RELATION").await?;
    
    // Create indexes
    db.query("DEFINE INDEX idx_entity_name ON code_entity FIELDS name").await?;
    db.query("DEFINE INDEX idx_entity_repo ON code_entity FIELDS repository_id").await?;
    db.query("DEFINE INDEX idx_entity_file ON code_entity FIELDS file_path").await?;
    
    Ok(())
}
```

### Schema Versioning

- Store schema version in repository metadata
- Check version on startup
- Provide migration functions for schema updates

---

## Validation Examples

### Valid Entity Creation

```rust
let entity = CodeEntity {
    id: RecordId::from(("code_entity", "parser_struct")),
    name: "Parser".to_string(),
    entity_type: EntityType::Struct,
    file_path: "src/parser.rs".to_string(),
    line_start: 10,
    line_end: 150,
    component_id: Some(RecordId::from(("component", "parser_module"))),
    repository_id: RecordId::from(("repository", "cudgel")),
    summary: None, // Will be filled by async task
    visibility: Visibility::Public,
    metadata: EntityMetadata {
        language: "Rust".to_string(),
        signature: None,
        is_async: false,
        is_test: false,
        dependencies_count: 0,
    },
    created_at: Utc::now(),
    updated_at: Utc::now(),
};
```

### Invalid Entity (Validation Error)

```rust
let invalid = CodeEntity {
    name: "".to_string(),           // ❌ Empty name
    line_start: 100,
    line_end: 50,                   // ❌ line_end < line_start
    repository_id: RecordId::from(("repository", "nonexistent")), // ❌ Invalid reference
    // ... other fields
};
// Should return Error::ValidationError
```
