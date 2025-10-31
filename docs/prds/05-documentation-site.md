# PRD: Documentation Site

## Overview
Create a comprehensive documentation site deployed via GitHub Pages with user guides, API docs, tutorials, and examples.

## Goals
1. Centralize all documentation in one searchable site
2. Auto-generate API docs from Rust code
3. Provide clear getting started guide
4. Host on GitHub Pages (or custom domain)

## Non-Goals
- Video tutorials (text/images only for now)
- Interactive playground (separate PRD)
- Multi-language support

## Success Metrics
- Time to find specific doc: <30 seconds
- 90% of common questions answered in docs
- API documentation coverage: 100% of public APIs
- Zero broken links

## Detailed Requirements

### 1. Site Generator

**Tool**: mdBook or Docusaurus

**Comparison:**
| Feature | mdBook | Docusaurus |
|---------|--------|------------|
| Language | Rust | JavaScript |
| Speed | Very fast | Fast |
| Customization | Limited | Extensive |
| Search | Built-in | Algolia/local |
| Versioning | Manual | Built-in |
| API Docs | External | Integrated |

**Recommendation**: mdBook for simplicity and Rust integration

**Acceptance Criteria:**
- [ ] Site builds in <10 seconds
- [ ] Works offline
- [ ] Mobile responsive
- [ ] Fast page loads (<1s)

### 2. Site Structure

```
docs/
├── src/
│   ├── SUMMARY.md
│   ├── getting-started/
│   │   ├── installation.md
│   │   ├── quick-start.md
│   │   └── first-index.md
│   ├── user-guide/
│   │   ├── indexing.md
│   │   ├── searching.md
│   │   ├── graph-queries.md
│   │   └── configuration.md
│   ├── architecture/
│   │   ├── overview.md
│   │   ├── parser.md
│   │   ├── database.md
│   │   └── embeddings.md
│   ├── api/
│   │   └── rust-api.md (generated from rustdoc)
│   ├── integrations/
│   │   ├── mcp-server.md
│   │   ├── web-ui.md
│   │   └── lsp.md
│   ├── contributing/
│   │   ├── development.md
│   │   ├── testing.md
│   │   └── releases.md
│   └── troubleshooting.md
└── book.toml
```

**Acceptance Criteria:**
- [ ] Clear hierarchy
- [ ] Every page has next/previous links
- [ ] Sidebar navigation works
- [ ] Search includes all pages

### 3. Content Requirements

#### Getting Started
**Pages:**
1. Installation (all platforms)
2. Quick Start (index your first repo)
3. Basic Usage Examples

**Requirements:**
- Step-by-step instructions
- Copy-paste commands
- Expected output shown
- Troubleshooting tips

**Acceptance Criteria:**
- [ ] New user can index repo in <5 minutes
- [ ] All commands tested on fresh install
- [ ] Screenshots where helpful
- [ ] Links to next steps

#### User Guide
**Pages:**
1. Indexing - Advanced options, scheduling
2. Searching - Query syntax, filters
3. Graph Queries - Exploring relationships
4. Configuration - Settings and customization

**Requirements:**
- Comprehensive coverage of all CLI commands
- Examples for common use cases
- Reference tables for flags/options
- Best practices

**Acceptance Criteria:**
- [ ] Every CLI flag documented
- [ ] Examples for each command
- [ ] Covers edge cases
- [ ] Updated when commands change

#### Architecture
**Pages:**
1. System Overview - High-level architecture
2. Parser - Tree-sitter integration
3. Database - Schema and queries
4. Embeddings - Vector search

**Requirements:**
- Diagrams showing components
- Code examples
- Performance characteristics
- Extension points

**Acceptance Criteria:**
- [ ] Diagrams are clear and accurate
- [ ] Explains design decisions
- [ ] Links to relevant code
- [ ] Useful for contributors

#### API Documentation
**Generation**: `cargo doc` → mdBook integration

**Requirements:**
- All public APIs documented
- Examples for each module
- Links to source code
- Search functionality

**Acceptance Criteria:**
- [ ] Generated automatically on deploy
- [ ] No undocumented public items
- [ ] Examples compile
- [ ] Cross-references work

### 4. Search Functionality

**Implementation**: mdBook built-in search

**Requirements:**
- Full-text search across all pages
- Search-as-you-type
- Keyboard shortcuts (/ to focus)
- Highlight matches

**Acceptance Criteria:**
- [ ] Finds relevant pages quickly
- [ ] Shows snippets with matches
- [ ] Keyboard navigable
- [ ] Works offline

### 5. Deployment

**Platform**: GitHub Pages

**Process:**
1. GitHub Action triggers on push to `main`
2. Build site with mdBook
3. Generate rustdoc
4. Deploy to `gh-pages` branch
5. Publish to `https://roshbhatia.github.io/cudgel/`

**Custom Domain (Optional):**
- `docs.cudgel.dev` → GitHub Pages
- SSL automatically provided by GitHub

**Acceptance Criteria:**
- [ ] Deploys automatically on push
- [ ] Build failures don't deploy
- [ ] HTTPS enabled
- [ ] Custom domain works (if configured)
- [ ] Redirects from old URLs

### 6. Version Switcher

**Requirement**: Support viewing docs for different versions

**Implementation:**
- Tag-based deployment
- Version selector in sidebar
- Latest version as default

**URL Structure:**
```
/            → Latest
/v0.2.0/     → Specific version
/main/       → Development version
```

**Acceptance Criteria:**
- [ ] Can view docs for any tagged version
- [ ] Version selector works
- [ ] URLs are stable
- [ ] Old versions remain accessible

### 7. Visual Design

**Theme**: Dark mode default, light mode available

**Branding:**
- Logo in header
- Consistent color scheme
- Professional typography
- Code highlighting (Rust-aware)

**Accessibility:**
- WCAG 2.1 AA compliant
- Keyboard navigation
- Screen reader friendly
- High contrast ratios

**Acceptance Criteria:**
- [ ] Dark mode looks good
- [ ] Light mode looks good
- [ ] Mobile responsive
- [ ] Passes accessibility audit
- [ ] Fast font loading

## Implementation Plan

### Phase 1: Setup (Week 1)
1. Choose mdBook vs Docusaurus
2. Create initial structure
3. Set up book.toml configuration
4. Create theme customization

### Phase 2: Core Content (Week 2-3)
1. Write Getting Started section
2. Write User Guide
3. Migrate existing docs from `docs/`
4. Add examples and screenshots

### Phase 3: Architecture Docs (Week 4)
1. Create architecture diagrams
2. Write component documentation
3. Add code examples
4. Link to source code

### Phase 4: API Integration (Week 5)
1. Configure rustdoc generation
2. Integrate rustdoc with mdBook
3. Add examples to docstrings
4. Test all examples compile

### Phase 5: Deployment (Week 6)
1. Set up GitHub Action
2. Configure GitHub Pages
3. Test deployment pipeline
4. Set up custom domain (if applicable)

### Phase 6: Polish (Week 7)
1. Proofread all content
2. Add missing screenshots
3. Fix broken links
4. User testing

## Dependencies
- All code documented (docstrings)
- Diagrams created (draw.io or mermaid)
- Screenshots captured
- GitHub Pages enabled

## Risks & Mitigation

**Risk**: Documentation becomes outdated
**Mitigation**: CI checks for broken links, reviewers verify docs in PRs

**Risk**: Rustdoc breaks mdBook integration
**Mitigation**: Use well-tested integration crates, have fallback plan

**Risk**: Search doesn't work well
**Mitigation**: Test search with real queries, add synonyms

## Open Questions
- Should we add a blog section for release announcements?
- Do we need a changelog page separate from GitHub Releases?
- Should examples be runnable in the browser?

## Testing Plan

### Content Review
- Technical accuracy check
- Grammar/spelling review
- Link validation
- Example verification

### User Testing
- New users follow getting started guide
- Developers find API they need
- Search finds relevant pages

### Automated Tests
- CI validates all internal links
- CI runs `mdbook test` to verify examples
- Lighthouse audit for performance/accessibility

## References
- [mdBook User Guide](https://rust-lang.github.io/mdBook/)
- [Docusaurus](https://docusaurus.io/)
- [Rust API Guidelines - Documentation](https://rust-lang.github.io/api-guidelines/documentation.html)
