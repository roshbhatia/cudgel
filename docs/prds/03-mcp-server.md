# PRD: MCP (Model Context Protocol) Server

## Overview
Implement an MCP server to enable AI assistants (Claude Desktop, etc.) to query Cudgel for code context and understanding.

## Goals
1. Enable Claude Desktop to search indexed codebases
2. Provide tools for semantic code search
3. Support graph traversal for understanding relationships
4. Simple setup and configuration

## Non-Goals
- Building a chat interface (Claude Desktop provides this)
- Streaming large code snippets (use references instead)
- Code modification capabilities

## Success Metrics
- Claude can answer "What does function X do?" accurately 90% of the time
- Average MCP request latency: <500ms
- Setup time: <5 minutes for new users
- Zero crashes in 1000 requests

## User Stories

### As a developer using Claude Desktop, I want to...
1. Ask Claude about functions in my codebase without copying code
2. Understand how different parts of my code relate
3. Find examples of how a specific API is used
4. Get explanations of complex code flows

## Detailed Requirements

### 1. MCP Protocol Implementation

**MCP Version**: 1.0
**Transport**: stdio (standard input/output)

**Server Capabilities:**
```json
{
  "serverInfo": {
    "name": "cudgel",
    "version": "0.1.0"
  },
  "capabilities": {
    "tools": {}
  }
}
```

**Requirements:**
- Implements MCP protocol spec v1.0
- Handles initialize/initialized handshake
- Responds to tool list requests
- Executes tool calls with parameters
- Returns structured JSON responses

**Acceptance Criteria:**
- [ ] Passes MCP protocol compliance tests
- [ ] Claude Desktop successfully connects
- [ ] All tool calls return valid responses
- [ ] Error messages include context
- [ ] Handles connection drops gracefully

### 2. MCP Tools

#### Tool: search_symbols
**Description**: Search for symbols (functions, classes, etc.) by name or description

**Parameters:**
```typescript
{
  query: string,      // Search query
  limit?: number,     // Max results (default: 10)
  kind?: string,      // Filter by symbol kind
  repository?: string // Filter by repository
}
```

**Response:**
```typescript
{
  results: [
    {
      name: string,
      kind: string,      // "function", "class", etc.
      file: string,
      line: number,
      signature: string,
      docstring: string | null,
      similarity: number
    }
  ]
}
```

**Acceptance Criteria:**
- [ ] Returns relevant results for natural language queries
- [ ] Results sorted by relevance
- [ ] Handles typos reasonably well
- [ ] Response time <500ms for typical queries

#### Tool: get_symbol
**Description**: Get detailed information about a specific symbol

**Parameters:**
```typescript
{
  name: string,          // Symbol name
  repository?: string    // Optional repository filter
}
```

**Response:**
```typescript
{
  name: string,
  kind: string,
  file: string,
  line: number,
  end_line: number,
  signature: string,
  docstring: string | null,
  code: string,
  references_count: number
}
```

**Acceptance Criteria:**
- [ ] Includes full code snippet
- [ ] Shows signature and documentation
- [ ] Counts references
- [ ] Returns error if not found

#### Tool: get_call_graph
**Description**: Get call graph for a symbol (what it calls, what calls it)

**Parameters:**
```typescript
{
  symbol: string,
  depth?: number,        // Default: 2
  direction?: string,    // "outgoing" | "incoming" | "both"
  repository?: string
}
```

**Response:**
```typescript
{
  nodes: [
    {
      id: string,
      name: string,
      kind: string,
      file: string
    }
  ],
  edges: [
    {
      from: string,      // Node ID
      to: string,        // Node ID
      type: string       // "calls", "imports", etc.
    }
  ]
}
```

**Acceptance Criteria:**
- [ ] Graph traversal limited to specified depth
- [ ] Includes both incoming and outgoing edges
- [ ] Handles cycles without infinite loops
- [ ] Returns empty graph if symbol not found

#### Tool: find_references
**Description**: Find all places where a symbol is used

**Parameters:**
```typescript
{
  symbol: string,
  repository?: string,
  limit?: number
}
```

**Response:**
```typescript
{
  references: [
    {
      file: string,
      line: number,
      column: number,
      context: string    // Code snippet showing usage
    }
  ],
  total: number
}
```

**Acceptance Criteria:**
- [ ] Finds direct and indirect references
- [ ] Includes context around each usage
- [ ] Paginated results
- [ ] Accurate count of total references

#### Tool: get_file_symbols
**Description**: List all symbols defined in a file

**Parameters:**
```typescript
{
  path: string,
  repository?: string
}
```

**Response:**
```typescript
{
  file: string,
  symbols: [
    {
      name: string,
      kind: string,
      line: number,
      signature: string
    }
  ]
}
```

**Acceptance Criteria:**
- [ ] Returns all top-level symbols
- [ ] Ordered by line number
- [ ] Includes nested symbols (methods in classes)
- [ ] Returns error if file not indexed

### 3. CLI Integration

**Command:**
```bash
cudgel mcp start [--log-level debug]
```

**Behavior:**
- Starts MCP server on stdio
- Logs to stderr (since stdout is used for protocol)
- Graceful shutdown on SIGTERM/SIGINT

**Acceptance Criteria:**
- [ ] `cudgel mcp start` works with Claude Desktop
- [ ] Logs are helpful for debugging
- [ ] Can run as background service
- [ ] Automatic restart on crashes

### 4. Claude Desktop Configuration

**Config File**: `~/Library/Application Support/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "cudgel": {
      "command": "cudgel",
      "args": ["mcp", "start"],
      "env": {}
    }
  }
}
```

**Documentation:**
- Installation guide
- Configuration examples
- Troubleshooting tips
- Example conversations

**Acceptance Criteria:**
- [ ] Copy-paste config works for all users
- [ ] Clear instructions for each OS
- [ ] Screenshots showing successful connection
- [ ] Example prompts demonstrating each tool

## Implementation Plan

### Phase 1: Protocol Foundation (Week 1)
1. Research MCP protocol specification
2. Implement handshake and capability negotiation
3. Create tool registry system
4. Write protocol compliance tests

### Phase 2: Core Tools (Week 2)
1. Implement `search_symbols`
2. Implement `get_symbol`
3. Implement `get_call_graph`
4. Integration tests for each tool

### Phase 3: Additional Tools (Week 3)
1. Implement `find_references`
2. Implement `get_file_symbols`
3. Error handling for all tools
4. Performance optimization

### Phase 4: Integration (Week 4)
1. CLI command implementation
2. Claude Desktop configuration
3. Documentation and examples
4. User testing with Claude Desktop

## Dependencies
- Cudgel database must be populated (indexed codebase)
- Claude Desktop installed
- Understanding of MCP protocol

## Risks & Mitigation

**Risk**: MCP protocol changes/updates
**Mitigation**: Follow MCP spec closely, version server capability

**Risk**: Large responses timeout
**Mitigation**: Pagination, streaming for large results

**Risk**: Poor search results
**Mitigation**: Tune embedding model, add explicit symbol matching

## Open Questions
- Should we support multiple concurrent connections?
- Do we need rate limiting?
- Should tools cache results?

## Testing Plan

### Unit Tests
- Each tool returns correct data structure
- Parameter validation works
- Error cases handled properly

### Integration Tests
- Full request/response cycle
- Claude Desktop can connect
- All tools work end-to-end

### Performance Tests
- Response times <500ms
- Handles 100 concurrent requests
- No memory leaks over 1000 requests

## References
- [MCP Specification](https://modelcontextprotocol.io/docs)
- [Claude Desktop MCP Docs](https://modelcontextprotocol.io/docs/tools/claude-desktop)
- [Example MCP Servers](https://github.com/modelcontextprotocol/servers)
