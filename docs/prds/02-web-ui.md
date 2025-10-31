# PRD: Web UI for Code Visualization

## Overview
Build a browser-based UI for exploring indexed codebases with an Obsidian-style graph view and query interface.

## Goals
1. Provide visual code exploration and navigation
2. Offer interactive graph visualization of code relationships
3. Enable powerful search with preview
4. Run as a local service alongside PostgreSQL

## Non-Goals
- Cloud/hosted version (local-only for now)
- Code editing capabilities
- Authentication/multi-user support

## Success Metrics
- Time to find a specific function: <5 seconds
- Graph view renders 1000 nodes smoothly (>30 FPS)
- Search results appear in <100ms
- 90% of users prefer UI over CLI for exploration

## User Stories

### As a developer, I want to...
1. Visualize my codebase as an interactive graph
2. Search for symbols and see previews before diving in
3. Click on a node to see its relationships
4. Explore call graphs interactively
5. Filter by file type, symbol kind, or package

## Detailed Requirements

### 1. Backend Service

**Tech Stack**: Axum (Rust web framework)

**Endpoints:**
```
GET  /api/symbols?q=<query>&limit=<n>          - Search symbols
GET  /api/symbols/:id                          - Get symbol details
GET  /api/symbols/:id/references               - Get references
GET  /api/graph/:symbol?depth=<n>              - Get graph data
GET  /api/repositories                         - List indexed repos
POST /api/repositories/:id/reindex             - Trigger reindex
WS   /api/events                               - WebSocket for live updates
```

**Requirements:**
- RESTful API with JSON responses
- CORS enabled for local development
- WebSocket for real-time indexing updates
- Pagination for large result sets
- Request logging and metrics

**Acceptance Criteria:**
- [ ] All endpoints documented with OpenAPI/Swagger
- [ ] Response times <100ms for typical queries
- [ ] WebSocket notifies UI when indexing completes
- [ ] API versioned (e.g., `/api/v1/...`)
- [ ] Error responses include helpful messages

### 2. Frontend Application

**Tech Stack**:
- React 18 + TypeScript
- Vite for build tooling
- Tailwind CSS for styling
- D3.js or Cytoscape.js for graph visualization
- Monaco Editor for code preview
- React Query for data fetching

**Pages:**
1. **Dashboard**: Recent repos, quick search, stats
2. **Search**: Full-text search with live preview
3. **Graph**: Interactive force-directed graph
4. **Symbol Details**: Code, references, documentation
5. **Settings**: Index management, preferences

**Acceptance Criteria:**
- [ ] Responsive design (works on laptop screens 1920x1080 and 1366x768)
- [ ] Dark mode (default) and light mode
- [ ] Keyboard shortcuts for common actions
- [ ] Search as you type with debouncing
- [ ] Accessible (WCAG 2.1 AA compliant)

### 3. Graph Visualization (Obsidian-style)

**Layout**: Force-directed (d3-force or Cytoscape.js)

**Features:**
- **Nodes**:
  - Size based on LOC or number of references
  - Color coded by file type (Python=blue, Rust=orange, etc.)
  - Icon based on symbol kind (function, class, struct)
  - Label shows symbol name

- **Edges**:
  - Directed arrows for calls/references
  - Line thickness based on number of calls
  - Color indicates edge type (calls=green, imports=blue)

- **Interactions**:
  - Click node → show details panel
  - Double-click node → expand outgoing edges
  - Hover → show tooltip with signature
  - Right-click → context menu (go to file, copy name)
  - Drag to reposition nodes
  - Zoom and pan
  - Highlight path between two nodes

- **Filters**:
  - Show only: functions/classes/imports
  - Filter by file path pattern
  - Depth slider (1-5 levels)
  - Hide external dependencies

**Acceptance Criteria:**
- [ ] Renders 1000 nodes at >30 FPS
- [ ] Graph updates smoothly when filters change
- [ ] Can save and load graph layouts
- [ ] Screenshot/export graph as PNG/SVG
- [ ] Minimap for navigation in large graphs

### 4. Search Interface

**Features:**
- Live search with debouncing (300ms)
- Syntax highlighting in preview
- Filter by kind, file, repository
- Sort by relevance, name, location
- Show matching line in context

**Search Results:**
```
┌────────────────────────────────────────────┐
│ authenticate_user              function    │
│ src/auth.rs:45                             │
│ ────────────────────────────────────────   │
│  43  pub fn authenticate_user(            │
│  44    username: &str,                    │
│▶ 45    password: &str                     │
│  46  ) -> Result<User> {                  │
│ ────────────────────────────────────────   │
└────────────────────────────────────────────┘
```

**Acceptance Criteria:**
- [ ] Search results appear in <100ms
- [ ] Highlighted matches in preview
- [ ] Navigate results with arrow keys
- [ ] "Open in editor" button (opens file at line)
- [ ] Search history (last 10 queries)

### 5. Integration

**CLI Command:**
```bash
cudgel ui start [--port 8080] [--open]
```

**Task Automation:**
```yaml
ui-dev:
  desc: Start UI dev server with auto-reload
  cmds:
    - cd ui && npm run dev

ui-build:
  desc: Build production UI assets
  cmds:
    - cd ui && npm run build

ui-start:
  desc: Start production UI server
  cmds:
    - cudgel ui start --port 8080 --open
```

**Acceptance Criteria:**
- [ ] `cudgel ui start` builds UI and starts server
- [ ] Auto-opens browser to http://localhost:8080
- [ ] `--port` flag changes port
- [ ] UI shows connection status
- [ ] Graceful shutdown on Ctrl+C

## Implementation Plan

### Phase 1: Backend API (Week 1-2)
1. Set up Axum project structure
2. Implement REST endpoints
3. Add WebSocket support
4. Write OpenAPI documentation
5. Integration tests for API

### Phase 2: Frontend Setup (Week 3)
1. Create React + Vite project
2. Set up routing and layout
3. Implement API client
4. Add Tailwind CSS
5. Dark mode support

### Phase 3: Search Interface (Week 4)
1. Build search page
2. Add live search with debouncing
3. Implement result preview
4. Add filters and sorting
5. Keyboard navigation

### Phase 4: Graph Visualization (Week 5-6)
1. Choose library (D3 vs Cytoscape)
2. Implement force-directed layout
3. Add node/edge styling
4. Interaction handlers
5. Filters and controls

### Phase 5: Polish & Testing (Week 7)
1. User testing sessions
2. Fix bugs and UX issues
3. Performance optimization
4. Accessibility audit
5. Documentation

## Dependencies
- Backend API must be stable before frontend work
- Graph library evaluation needed before Phase 4

## Risks & Mitigation

**Risk**: Graph performance with large codebases
**Mitigation**: Virtualization, level-of-detail rendering, lazy loading

**Risk**: Complex D3.js/Cytoscape learning curve
**Mitigation**: Start with simpler layouts, iterate based on feedback

**Risk**: Keeping UI in sync with indexing
**Mitigation**: WebSocket for real-time updates, loading states

## Open Questions
- Should we support embedding the UI in VS Code?
- Do we need a mobile-friendly version?
- Should graph state be persisted across sessions?

## References
- [Obsidian Graph View](https://obsidian.md/)
- [Cytoscape.js](https://js.cytoscape.org/)
- [D3 Force Directed Graph](https://d3-graph-gallery.com/network.html)
