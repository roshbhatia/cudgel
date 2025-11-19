# Data Model: Automatic Dependency Management

**Feature**: 004-auto-deps-management  
**Phase**: 1 (Design)  
**Date**: 2025-11-19

## Overview

This document defines the key entities for automatic dependency management in cudgel. All entities are technology-agnostic descriptions of data structures and their relationships.

---

## Entity: Dependency

**Purpose**: Represents a required component that cudgel needs to function properly.

### Attributes

| Attribute | Type | Description | Validation Rules |
|-----------|------|-------------|------------------|
| name | String | Human-readable dependency name | Non-empty, unique (e.g., "ONNX Embedding Model", "PostgreSQL Database", "Database Schema") |
| component_type | Enum | Category of dependency | One of: MODEL, DATABASE, SCHEMA, EXTERNAL_TOOL |
| status | Enum | Current state of dependency | One of: MISSING, SATISFIED, CORRUPTED, UNKNOWN |
| required | Boolean | Whether dependency is mandatory | True for core features, false for optional features |
| validator | Function | Logic to check if dependency is satisfied | Must complete in <2 seconds |
| installer | Function | Logic to install/configure dependency | May be long-running (minutes) |
| error_message | String | User-facing message when validation fails | Must include remediation steps |

### Relationships

- **One Dependency has zero or more Prerequisites** (other Dependencies that must be satisfied first)
  - Example: "Database Schema" requires "PostgreSQL Database" to be running
  - Example: "ONNX Model" requires "Sufficient Disk Space" (implicit prerequisite)

### States & Transitions

```
UNKNOWN ──(validate)──> MISSING ──(install)──> SATISFIED
                            │                      │
                            │                      │
                            └──(install fails)──> CORRUPTED
                                                   │
                                                   └──(cleanup + retry)──> MISSING
```

### Business Rules

1. Dependencies MUST be validated in prerequisite order
2. Installation MUST NOT proceed if prerequisites are MISSING
3. CORRUPTED dependencies MUST be cleanable (return to MISSING state)
4. Validation MUST be idempotent (safe to check repeatedly)
5. Installation MUST be idempotent (safe to run when already SATISFIED)

---

## Entity: ModelArtifact

**Purpose**: Represents a downloadable machine learning model file from HuggingFace Hub.

### Attributes

| Attribute | Type | Description | Validation Rules |
|-----------|------|-------------|------------------|
| model_id | String | HuggingFace model identifier | Format: "org/model-name" (e.g., "sentence-transformers/all-MiniLM-L6-v2") |
| filename | String | Specific file within model repository | Valid filename (e.g., "model.onnx", "tokenizer.json") |
| source_url | URL | Full download URL | Must be HTTPS, point to huggingface.co |
| target_path | Path | Local filesystem destination | Must be within XDG_DATA_HOME/cudgel/ |
| expected_size_bytes | Integer | Approximate file size | 0-200,000,000 (0-200MB) for validation |
| checksum | String | Integrity verification hash | Optional SHA-256 hex string |
| download_progress_bytes | Integer | Current download progress | 0 to expected_size_bytes |
| download_state | Enum | Current download state | One of: PENDING, IN_PROGRESS, COMPLETED, FAILED |

### Relationships

- **One ModelArtifact belongs to one Dependency**
  - Example: "model.onnx" artifact belongs to "ONNX Embedding Model" dependency
  - Example: "tokenizer.json" artifact belongs to "ONNX Embedding Model" dependency
- **One Dependency may require multiple ModelArtifacts**
  - Example: ONNX model requires 3 files (model.onnx, tokenizer.json, config.json)

### States & Transitions

```
PENDING ──(start download)──> IN_PROGRESS ──(complete)──> COMPLETED
                                    │                          │
                                    │                          │
                                    └──(network error)──> FAILED
                                                           │
                                                           └──(retry)──> IN_PROGRESS
```

### Business Rules

1. Download progress MUST be tracked for files >10MB
2. Partial downloads MUST be resumable (HTTP Range requests)
3. Downloaded files MUST be verified before marking COMPLETED
4. FAILED downloads MUST be cleanable (temp files removed)
5. Target path MUST respect XDG_DATA_HOME environment variable
6. Downloads MUST show progress indicators for operations >5 seconds

---

## Entity: DatabaseInstance

**Purpose**: Represents a PostgreSQL database server instance managed by cudgel.

### Attributes

| Attribute | Type | Description | Validation Rules |
|-----------|------|-------------|------------------|
| host | String | Database server hostname | Valid hostname or IP (default: "localhost") |
| port | Integer | Database server port | 1-65535 (default: 45678) |
| database_name | String | Name of specific database | Valid PostgreSQL identifier (default: "cudgel") |
| data_directory | Path | PostgreSQL data directory location | Must be within XDG_DATA_HOME/cudgel/ |
| pid | Integer | Process ID of running server | Optional, 0 if not running |
| status | Enum | Current server state | One of: STOPPED, STARTING, RUNNING, ERROR |
| log_file | Path | Path to server logs | Within XDG_STATE_HOME/cudgel/ |
| version | String | PostgreSQL version | Semver format (e.g., "15.3") |

### Relationships

- **One DatabaseInstance hosts one or more Schemas**
  - Example: "cudgel" database contains "repositories", "symbols", "embeddings" tables
- **One DatabaseInstance is a prerequisite for one Schema Dependency**
  - Schema cannot be initialized unless database is RUNNING

### States & Transitions

```
STOPPED ──(start)──> STARTING ──(verify ready)──> RUNNING
            │             │                          │
            │             │                          │
            │             └──(timeout)──> ERROR     │
            │                                        │
            └──(startup error)──> ERROR              │
                                    │                │
                                    └──(stop)───────>└──(stop)──> STOPPED
```

### Business Rules

1. Database MUST be checked for running state before attempting start
2. Start operation MUST be idempotent (no-op if already RUNNING)
3. Database MUST use non-standard port to avoid conflicts (45678)
4. Data directory MUST be initialized on first start (initdb)
5. pgvector extension MUST be created on first start
6. Stop operation MUST use graceful shutdown (no data loss)
7. Status check MUST complete in <100ms (use pg_isready, not full connection)

---

## Entity: SchemaVersion

**Purpose**: Represents the state of database schema initialization and migrations.

### Attributes

| Attribute | Type | Description | Validation Rules |
|-----------|------|-------------|------------------|
| version_number | Integer | Current schema version | Non-negative, monotonically increasing |
| applied_at | Timestamp | When this version was applied | ISO 8601 format |
| tables_created | List[String] | Names of tables created | Valid PostgreSQL identifiers |
| indexes_created | List[String] | Names of indexes created | Valid PostgreSQL identifiers |
| extensions_enabled | List[String] | PostgreSQL extensions enabled | Valid extension names (e.g., "vector") |
| is_initialized | Boolean | Whether schema setup is complete | True if all required objects exist |

### Relationships

- **One SchemaVersion belongs to one DatabaseInstance**
  - Schema version tracked within specific database
- **One SchemaVersion is required by all cudgel Operations**
  - Index, Query, Orchestrator operations require initialized schema

### States & Transitions

```
NOT_INITIALIZED ──(run migrations)──> INITIALIZING ──(verify)──> INITIALIZED
                                            │                        │
                                            │                        │
                                            └──(error)──> ERROR      │
                                                            │         │
                                                            └─────────┘
                                                           (rollback + retry)
```

### Business Rules

1. Schema initialization MUST be idempotent (CREATE IF NOT EXISTS)
2. Required tables: repositories, files, symbols, embeddings, scheduled_tasks, call_graph_edges
3. Required extensions: vector (pgvector)
4. Required indexes: HNSW index on embeddings.embedding column
5. Schema version MUST be tracked in a metadata table
6. Initialization MUST complete in <10 seconds

---

## Composite Entity: DependencyGraph

**Purpose**: Represents the dependency relationships and validation order.

### Structure

```
DependencyGraph
  ├── External Tools (level 0)
  │   ├── PostgreSQL Installation
  │   ├── Python/uv (optional, for manual model conversion)
  │   └── Sufficient Disk Space (implicit)
  │
  ├── Database Layer (level 1)
  │   └── DatabaseInstance
  │       ├── Prerequisites: PostgreSQL Installation
  │       └── Artifacts: data directory, PID file, log files
  │
  ├── Schema Layer (level 2)
  │   └── SchemaVersion
  │       ├── Prerequisites: DatabaseInstance (RUNNING)
  │       └── Artifacts: tables, indexes, extensions
  │
  └── Model Layer (level 3)
      └── ONNX Embedding Model
          ├── Prerequisites: Sufficient Disk Space
          └── Artifacts: model.onnx, tokenizer.json, config.json
```

### Validation Order

1. **Level 0**: Check external tools and disk space (fast, informational)
2. **Level 1**: Validate/start database (may require initialization)
3. **Level 2**: Validate/initialize schema (requires database running)
4. **Level 3**: Validate/download models (independent, can run parallel)

### Business Rules

1. Dependencies MUST be validated in level order
2. Higher levels MUST NOT proceed if lower levels have MISSING dependencies
3. Dependencies within same level MAY be validated/installed in parallel
4. Full dependency check MUST complete in <2 seconds when all satisfied
5. Full dependency installation MAY take up to 5 minutes (model download)

---

## Data Persistence

**Note**: This feature does not introduce new persistent data structures. It validates and manages existing cudgel infrastructure.

### Existing Tables Used
- `scheduled_tasks` - Check if PostgreSQL can connect (validates database)
- All existing tables - Validates schema is initialized

### Transient Data
- Download progress - In-memory only, not persisted
- Dependency status - Calculated on-demand, not cached

### File System Locations (XDG-Compliant)

```
$XDG_DATA_HOME/cudgel/               (default: ~/.local/share/cudgel/)
  ├── models/
  │   └── all-MiniLM-L6-v2/
  │       ├── model.onnx              (ModelArtifact)
  │       ├── tokenizer.json          (ModelArtifact)
  │       └── config.json             (ModelArtifact)
  └── postgres/                       (DatabaseInstance data_directory)
      ├── base/
      ├── global/
      └── pg_wal/

$XDG_STATE_HOME/cudgel/              (default: ~/.local/state/cudgel/)
  ├── postgres.log                    (DatabaseInstance log_file)
  └── orchestrator.log                (existing)
```

---

## Error Scenarios by Entity

### Dependency
- **Invalid prerequisite order**: Attempting to install before prerequisites satisfied → Error message with prerequisite list
- **Validation timeout**: Validator takes >2 seconds → Error with performance issue indication
- **Unknown status**: Cannot determine if satisfied → Error with diagnostic steps

### ModelArtifact
- **Network failure**: Cannot reach HuggingFace Hub → Error with connectivity troubleshooting
- **Disk full**: Insufficient space for download → Error with required space and available space
- **Corrupted download**: File size mismatch → Error with retry instruction
- **Checksum mismatch**: File hash doesn't match expected → Error with re-download instruction

### DatabaseInstance
- **Port conflict**: Port 45678 already in use → Error with process identification command
- **Permission denied**: Cannot write to data directory → Error with permission fix commands
- **PostgreSQL not installed**: Cannot find pg_ctl → Error with platform-specific install instructions
- **Startup timeout**: Database doesn't respond within 30 seconds → Error with log file location
- **Extension missing**: pgvector not available → Error with extension installation instructions

### SchemaVersion
- **Migration failure**: Cannot create tables → Error with SQL error details and rollback action
- **Extension failure**: Cannot enable pgvector → Error with PostgreSQL version requirement (15+)
- **Connection error**: Cannot connect to database → Error indicating database must be running

---

## Summary

This data model defines three core entities (Dependency, ModelArtifact, DatabaseInstance) and one composite entity (DependencyGraph) that work together to provide automatic dependency management for cudgel. All entities follow principles of idempotent operations, clear state transitions, and actionable error messages.

**Key Design Principles**:
1. Technology-agnostic entity definitions
2. Clear state machines for all stateful entities
3. Explicit prerequisite relationships
4. XDG-compliant file system layout
5. Fast validation (<2s), acceptable installation time (<5min)
6. Idempotent operations throughout
