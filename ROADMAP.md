# Cudgel Roadmap

## Overview

This roadmap outlines the planned improvements and features for Cudgel, prioritizing reliability, user experience, and accessibility.

## Phase 1: Reliability & Code Quality

**Goal**: Ensure Cudgel is production-ready with excellent code quality and user experience

### Tasks
- [ ] **Code Quality Audit**
  - Make all code idiomatic Rust
  - Follow Rust API guidelines
  - Improve error messages with actionable suggestions
  - Add comprehensive documentation for all public APIs

- [ ] **UX Improvements**
  - Add progress indicators for long-running operations
  - Improve CLI output formatting and colors
  - Add `--dry-run` mode for indexing
  - Better error recovery and helpful error messages

- [ ] **Reliability**
  - Add retry logic for database operations
  - Graceful handling of large repositories
  - Memory usage optimization for large codebases
  - Connection pool tuning

- [ ] **Testing**
  - Increase test coverage to >80%
  - Add property-based testing
  - Performance benchmarks
  - Stress testing with large repositories

**Success Criteria**: All tests pass, code passes `cargo clippy` with no warnings, excellent error messages, smooth UX

---

## Phase 2: Web UI for Visualization

**Goal**: Add a browser-based UI for code exploration and visualization

### Tasks
- [ ] **Backend Service**
  - REST API server (using Axum or Actix)
  - WebSocket support for real-time updates
  - Runs as a service alongside PostgreSQL
  - Port: 8080 (configurable)

- [ ] **Frontend Application**
  - Modern JavaScript framework (React/Vue/Svelte)
  - Query previewer with syntax highlighting
  - Graph visualization (force-directed layout)
  - Search interface with filters
  - Symbol details panel

- [ ] **Graph View (Obsidian-style)**
  - Interactive node-link diagram
  - Zoom and pan
  - Node sizing based on importance
  - Color coding by file type
  - Click to expand call graph

- [ ] **Integration**
  - `cudgel ui start` command to launch web server
  - Auto-open browser
  - Live updates when repository is re-indexed
  - Task automation: `task ui-dev` for development

**Tech Stack**:
- Backend: Axum + Tower
- Frontend: React + D3.js/Cytoscape.js for graphs
- Build: Vite
- Styling: Tailwind CSS

**Success Criteria**: Beautiful, responsive UI that makes code exploration intuitive and fast

---

## Phase 3: MCP (Model Context Protocol) Server

**Goal**: Enable AI assistants to query Cudgel for code context

### Tasks
- [ ] **MCP Server Implementation**
  - Implement MCP protocol
  - Tools for querying symbols
  - Tools for graph traversal
  - Tools for semantic search

- [ ] **Integration**
  - `cudgel mcp start` command
  - Configuration for connecting to Claude Desktop
  - Documentation for setup

- [ ] **MCP Tools**
  - `search_symbols(query: str, limit: int)`
  - `get_symbol(name: str)`
  - `get_call_graph(symbol: str, depth: int)`
  - `find_references(symbol: str)`
  - `get_file_symbols(path: str)`

**Success Criteria**: Claude Desktop can query Cudgel to answer questions about codebases

---

## Phase 4: CI/CD & Releases

**Goal**: Automated testing, building, and releasing

### Tasks
- [ ] **GitHub Actions Workflows**
  - `.github/workflows/ci.yml`: Run tests, clippy, fmt on PRs
  - `.github/workflows/release.yml`: Build binaries on tags
  - Multi-platform builds (Linux, macOS, Windows)

- [ ] **Release Automation**
  - Semantic versioning
  - Changelog generation from commits
  - GitHub Releases with binaries
  - Homebrew formula updates
  - Crates.io publishing

- [ ] **Quality Gates**
  - All tests must pass
  - Clippy warnings = failure
  - Code coverage reports
  - Dependency security audits

**Success Criteria**: Every commit to main is tested, every tag produces release artifacts

---

## Phase 5: Documentation Site

**Goal**: Comprehensive documentation available at cudgel.dev (or GitHub Pages)

### Tasks
- [ ] **Documentation Site**
  - Static site generator (mdBook or Docusaurus)
  - API documentation
  - User guides
  - Architecture diagrams
  - Examples and tutorials

- [ ] **Content**
  - Getting Started guide
  - CLI reference
  - API documentation (from rustdoc)
  - Integration guides (MCP, UI)
  - Troubleshooting

- [ ] **Deployment**
  - GitHub Pages deployment
  - Custom domain (optional)
  - Search functionality
  - Version switcher

**Success Criteria**: Docs are easy to find, navigate, and understand

---

## Phase 6: Nix Flake Distribution

**Goal**: Make Cudgel easily installable via Nix flakes

### Tasks
- [ ] **Flake Configuration**
  - `flake.nix` with proper inputs/outputs
  - Build Cudgel from source
  - Include PostgreSQL + pgvector as dependencies
  - Development shell with all tools

- [ ] **Package Definition**
  - NixOS module for running Cudgel as a service
  - Home Manager integration
  - Proper dependency management

- [ ] **Testing**
  - Test on NixOS
  - Test on macOS with Nix
  - Test on Linux with Nix

**Success Criteria**: `nix run github:roshbhatia/cudgel` works out of the box

---

## Future Considerations

- **LSP Server**: Integrate with editors (VS Code, Neovim)
- **GitHub Integration**: Index repositories directly from GitHub URLs
- **Real Embeddings**: Replace dummy embeddings with ONNX models
- **Incremental Indexing**: Only re-index changed files
- **Multi-repo Search**: Search across multiple indexed repositories
- **Custom Analyzers**: Plugin system for language-specific analysis

---

## Contributing

See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for how to contribute to these roadmap items.

## Status

Current focus: **Phase 1 - Reliability & Code Quality**
