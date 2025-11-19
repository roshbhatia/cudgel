# Feature Specification: Fallback Tokenization Strategy

**Feature Branch**: `001-fallback-tokenization`  
**Created**: 2025-11-19  
**Status**: Draft  
**Input**: User description: "add optional support for a different (on device, maybe?) tokenizing strategy if the models arent found. for ex; im on a work computer where i cant pull from hugging face. this should be configurable via an environment variable, with the default being those ONYX models"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Restricted Environment Usage (Priority: P1)

Users working in restricted corporate environments (no internet access, firewall restrictions, or security policies preventing external downloads) need to use the code indexing tool without downloading ONNX models from HuggingFace or other external sources.

**Why this priority**: This is the primary blocker preventing users from using the tool in common enterprise environments. Without this feature, the tool is completely unusable in these scenarios, affecting a significant portion of potential users.

**Independent Test**: Can be fully tested by setting an environment variable, running the tool without ONNX models present, and verifying that indexing completes successfully using the fallback tokenization strategy. This delivers immediate value by making the tool functional in restricted environments.

**Acceptance Scenarios**:

1. **Given** no ONNX models are installed and fallback tokenization is enabled, **When** user runs indexing on a codebase, **Then** the system uses the fallback tokenizer and completes indexing successfully
2. **Given** ONNX models are not accessible and fallback is enabled, **When** user performs a semantic search query, **Then** the system returns relevant results using embeddings generated from the fallback tokenizer
3. **Given** user is in a restricted environment, **When** they set the tokenization strategy via environment variable before first run, **Then** the system initializes with the fallback strategy without attempting to access ONNX models

---

### User Story 2 - Graceful Degradation (Priority: P2)

Users who have ONNX models available want the system to automatically use them for optimal semantic search quality, while users without models want automatic fallback to a simpler strategy without manual intervention.

**Why this priority**: This enhances user experience by providing intelligent defaults and reducing configuration burden. Users get the best available option automatically.

**Independent Test**: Can be tested by running the tool with models present (verifies ONNX usage) and then removing models (verifies automatic fallback). Delivers value by reducing setup friction.

**Acceptance Scenarios**:

1. **Given** ONNX models exist in the expected location, **When** no explicit tokenization strategy is configured, **Then** the system uses ONNX models by default
2. **Given** ONNX models do not exist and no fallback strategy is configured, **When** user attempts to initialize the embedding generator, **Then** the system provides a clear error message explaining both the ONNX model download option and the fallback strategy configuration
3. **Given** user explicitly configures fallback strategy via environment variable, **When** ONNX models are available, **Then** the system respects the user's choice and uses the fallback strategy

---

### User Story 3 - Configuration Transparency (Priority: P3)

Users want to understand which tokenization strategy is being used and be able to verify their configuration is working as expected.

**Why this priority**: This provides observability and debugging support but is not essential for core functionality. Users can still use the tool without explicit strategy confirmation.

**Independent Test**: Can be tested by checking log output or diagnostic commands to confirm active strategy. Delivers value by improving troubleshooting and confidence in system behavior.

**Acceptance Scenarios**:

1. **Given** the system is initialized, **When** user reviews startup logs or diagnostic output, **Then** the active tokenization strategy (ONNX or fallback) is clearly indicated
2. **Given** user wants to verify their environment variable configuration, **When** they run the tool with verbose logging, **Then** the system logs which strategy was selected and why
3. **Given** user switches between tokenization strategies, **When** they re-index their codebase, **Then** existing embeddings are regenerated using the new strategy

---

### Edge Cases

- What happens when the environment variable contains an invalid or unsupported tokenization strategy name?
- How does the system handle partial model installations (e.g., tokenizer.json exists but model.onnx is missing)?
- What happens when switching tokenization strategies mid-operation (e.g., user sets environment variable after some files are already indexed)?
- How does search quality compare between ONNX and fallback strategies, and should users be warned about potential quality differences?
- What happens if the fallback tokenizer itself fails to initialize?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST support an environment variable (e.g., `CUDGEL_TOKENIZER_STRATEGY`) to configure the tokenization strategy with valid values including "onnx" (default) and at least one fallback option (e.g., "simple", "basic", or "builtin")
- **FR-002**: System MUST attempt to use ONNX models by default when no tokenization strategy is explicitly configured
- **FR-003**: System MUST fall back to the configured alternative tokenization strategy when ONNX models are unavailable and a fallback strategy is configured
- **FR-004**: System MUST provide clear error messages when ONNX models are unavailable and no fallback strategy is configured, explaining both model installation and fallback configuration options
- **FR-005**: System MUST generate embeddings using the active tokenization strategy that are compatible with the existing vector search infrastructure
- **FR-006**: System MUST produce embeddings with consistent dimensions (384 by default) regardless of which tokenization strategy is used
- **FR-007**: System MUST log the active tokenization strategy during initialization for diagnostic purposes
- **FR-008**: System MUST allow users to switch tokenization strategies without requiring code changes (environment variable only)
- **FR-009**: Fallback tokenization strategy MUST NOT require downloading external models or accessing network resources
- **FR-010**: System MUST validate the tokenization strategy specified in the environment variable and provide helpful error messages for invalid values

### Key Entities

- **Tokenization Strategy**: Represents the method used to convert text into tokens for embedding generation. Key attributes include strategy name (e.g., "onnx", "simple"), initialization requirements (model files vs. built-in), and embedding quality characteristics.
- **Embedding Generator Configuration**: Extended configuration that includes tokenization strategy selection, fallback behavior rules, and strategy-specific parameters. Related to existing `EmbeddingConfig` in the Config structure.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can successfully index a codebase in environments without internet access or ONNX model files by setting a single environment variable
- **SC-002**: System initialization completes within 5 seconds using fallback tokenization (compared to 10-15 seconds for ONNX model loading)
- **SC-003**: Users receive actionable error messages with clear remediation steps when encountering tokenization configuration issues (error message includes both model installation instructions and fallback configuration syntax)
- **SC-004**: 100% of indexing operations complete successfully when fallback tokenization is configured, regardless of ONNX model availability
- **SC-005**: Fallback tokenization produces semantically meaningful embeddings that enable relevant search results for common code queries (manually verified for basic search scenarios like finding functions by name or finding similar code patterns)

## Assumptions *(optional)*

- The fallback tokenization strategy will use a simpler algorithm (e.g., character n-grams, word splitting, or basic linguistic tokenization) that doesn't require pre-trained models
- Embedding quality with fallback tokenization may be lower than ONNX models, but this trade-off is acceptable for restricted environments where ONNX is not available
- Users are comfortable with environment variable configuration as the primary configuration mechanism
- The same PostgreSQL vector search infrastructure can handle embeddings from different tokenization strategies as long as dimensions remain consistent
- Re-indexing may be required when switching between tokenization strategies to ensure consistency

## Out of Scope *(optional)*

- Multiple simultaneous tokenization strategies (system uses one strategy at a time)
- Automatic quality comparison or benchmarking between tokenization strategies
- Migration tools for converting embeddings between strategies
- Support for custom user-provided tokenization models
- Dynamic strategy switching based on performance or quality metrics
- Hybrid approaches that combine multiple tokenization strategies
