# Feature Specification: Automatic Dependency Management

**Feature Branch**: `004-auto-deps-management`  
**Created**: 2025-11-19  
**Status**: Draft  
**Input**: User description: "cudgel should automatically pull and store models for us. Manual setup instructions are an antipattern. Add a 'deps' command to pull dependencies, validate database is up, run migrations, etc. Find similar instruction patterns in the repo. Respect XDG specification for data."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - First-Time Setup (Priority: P1)

A developer clones the cudgel repository and wants to start using it immediately without reading extensive setup documentation or manually downloading models.

**Why this priority**: First-run experience is critical for adoption. Eliminating manual setup steps reduces friction and prevents common setup errors. This is the foundation that all other functionality depends on.

**Independent Test**: Can be fully tested by running `cudgel deps` in a fresh environment (no models, no database) and verifying that all dependencies are downloaded, database is running, and schema is initialized. Delivers immediate value by making cudgel usable.

**Acceptance Scenarios**:

1. **Given** fresh cudgel installation with no models downloaded, **When** user runs `cudgel deps`, **Then** ONNX embedding models are automatically downloaded to XDG-compliant data directory and user receives success confirmation
2. **Given** PostgreSQL is not running, **When** user runs `cudgel deps`, **Then** PostgreSQL is started automatically on the configured port with success confirmation
3. **Given** database schema is not initialized, **When** user runs `cudgel deps`, **Then** database schema (tables, indexes, extensions) is created automatically
4. **Given** all dependencies are already satisfied, **When** user runs `cudgel deps`, **Then** command completes quickly with "all dependencies satisfied" message
5. **Given** user runs any cudgel command requiring dependencies (index, query, orchestrator), **When** dependencies are missing, **Then** user sees actionable error message: "Dependencies not ready. Run: cudgel deps"

---

### User Story 2 - Dependency Validation (Priority: P2)

A developer returns to cudgel after a system restart or wants to verify their environment is properly configured before running operations.

**Why this priority**: Proactive validation prevents runtime failures and provides clear diagnostics. Essential for troubleshooting but can be implemented after basic setup works.

**Independent Test**: Can be tested by running `cudgel deps --check` in various states (missing models, database stopped, partial setup) and verifying accurate status reporting. Delivers value by helping users diagnose issues.

**Acceptance Scenarios**:

1. **Given** user runs `cudgel deps --check`, **When** all dependencies are satisfied, **Then** command shows green checkmarks for each component (models, database, schema) and exits with success
2. **Given** user runs `cudgel deps --check`, **When** some dependencies are missing, **Then** command shows which components are missing/broken with specific remediation steps
3. **Given** user runs `cudgel deps --check --verbose`, **When** command executes, **Then** detailed version information and paths are displayed for each component

---

### User Story 3 - Clean Dependency Management (Priority: P3)

A developer wants to remove all downloaded dependencies to free up disk space or reset their environment to a clean state.

**Why this priority**: Useful for cleanup and troubleshooting edge cases, but not required for core functionality. Can be implemented last.

**Independent Test**: Can be tested by running `cudgel deps --clean` after setup and verifying all XDG data directories are removed while preserving user data. Delivers value for disk management.

**Acceptance Scenarios**:

1. **Given** user runs `cudgel deps --clean`, **When** command executes, **Then** downloaded models are removed from XDG data directory with confirmation
2. **Given** user runs `cudgel deps --clean --all`, **When** command executes with confirmation, **Then** models, database data, and all XDG directories are removed
3. **Given** database is running during cleanup, **When** user runs `cudgel deps --clean --all`, **Then** database is stopped gracefully before removal

---

### Edge Cases

- What happens when model download fails mid-process (network interruption)?
  - Partial downloads should be cleaned up and user should be able to retry
- How does system handle PostgreSQL already running on the target port?
  - Should detect existing instance and verify it's accessible, or error with clear message
- What happens when disk space is insufficient for model download?
  - Should fail early with disk space check before attempting download
- How does system handle permission errors writing to XDG directories?
  - Should provide clear error message with file path and suggested resolution
- What happens when user has custom XDG environment variables set?
  - Must respect XDG_DATA_HOME, XDG_STATE_HOME, etc. environment variables
- How does system handle missing PostgreSQL installation?
  - Should detect missing PostgreSQL and provide installation instructions for user's platform
- What happens when Python/uv is not available for model download?
  - Should check prerequisites and provide clear installation instructions

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a `cudgel deps` command that checks and installs all required dependencies
- **FR-002**: System MUST automatically download ONNX embedding model (sentence-transformers/all-MiniLM-L6-v2) if not present
- **FR-003**: System MUST store all downloaded models in XDG-compliant data directories (respecting XDG_DATA_HOME environment variable)
- **FR-004**: System MUST automatically start PostgreSQL database if not running
- **FR-005**: System MUST automatically initialize database schema if not present
- **FR-006**: System MUST provide `cudgel deps --check` flag to validate dependencies without modifying system
- **FR-007**: System MUST provide `cudgel deps --clean` flag to remove downloaded models and temporary files
- **FR-008**: System MUST provide `cudgel deps --clean --all` flag to remove all data including database
- **FR-009**: System MUST eliminate all error messages that contain manual setup instructions (replace with "run: cudgel deps")
- **FR-010**: System MUST verify checksums/integrity of downloaded models
- **FR-011**: System MUST provide progress indicators during long-running operations (model download, database initialization)
- **FR-012**: System MUST detect and report missing prerequisites (PostgreSQL, Python/uv) with platform-specific installation instructions
- **FR-013**: All cudgel commands (index, query, orchestrator) MUST perform lightweight dependency validation on startup
- **FR-014**: System MUST handle partial/corrupted downloads by cleaning up and allowing retry
- **FR-015**: System MUST respect all XDG Base Directory specification environment variables (XDG_DATA_HOME, XDG_STATE_HOME, XDG_CACHE_HOME, XDG_CONFIG_HOME)

### Key Entities

- **Dependency**: Represents a required component (model files, database, schema) with validation logic and installation instructions
- **ModelArtifact**: Represents downloadable ONNX model with source URL, target path, checksum, and download progress
- **DatabaseInstance**: Represents PostgreSQL instance with connection parameters, status checking, and lifecycle management

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: New users can run `cudgel deps` and have fully functional environment in under 5 minutes (assuming reasonable internet connection)
- **SC-002**: Zero manual setup steps required - `cudgel deps` handles all dependency installation and configuration
- **SC-003**: Dependency validation (`cudgel deps --check`) completes in under 2 seconds
- **SC-004**: All error messages in the codebase that reference manual setup instructions are replaced with "run: cudgel deps"
- **SC-005**: System correctly respects XDG environment variables - all data written to user-specified directories when XDG_* variables are set
- **SC-006**: Failed dependency installation provides actionable error message with specific remediation steps in 100% of cases
- **SC-007**: Users can recover from failed setup by re-running `cudgel deps` without manual intervention
- **SC-008**: Disk space requirements are checked before attempting downloads, preventing partial failures

## Assumptions

1. Users have internet connectivity for initial model download (approximately 100MB)
2. Users have Python 3.8+ and uv available for model conversion (or will be guided to install)
3. Users have PostgreSQL 15+ available (or will be guided to install)
4. Users have sufficient disk space (~500MB) for models and database
5. Model download from HuggingFace Hub is reliable and supports resumable downloads
6. XDG Base Directory specification (version 0.8) is the standard for directory layout
7. Default model (sentence-transformers/all-MiniLM-L6-v2) remains available and suitable for cudgel's use case
