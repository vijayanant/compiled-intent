# 01-REQ-MCP-INTERFACE

## 1. Overview

The Intent Engine MUST expose its capabilities via the Model Context Protocol (MCP). The transport layer MUST be JSON-RPC 2.0 over standard input and output (`stdio`).

*Per ADR-001, the system must dogfood its own directives. The search tool MUST be named `recall` to enforce semantic correctness.*

## 2. Tool Registration (`tools/list`)

The server MUST respond to `tools/list` with exactly three tools.

### 2.1 `recall`

**Description:** Performs semantic or hybrid search against indexed documents.
**Input Schema:**

```json
{
  "type": "object",
  "properties": {
    "query": { "type": "string", "description": "The search term" },
    "namespace": { "type": "string", "description": "Target LanceDB table/collection" },
    "mode": { "type": "string", "enum": ["vector", "fts", "hybrid"], "default": "hybrid" },
    "metadata_filter": { "type": "string", "description": "SQL filter for JSON metadata (e.g., status = 'accepted')" },
    "limit": { "type": "integer", "default": 3 }
  },
  "required": ["query", "namespace"]
}
```

**Acceptance Criteria:**

- **Given** an invalid namespace, **When** invoked, **Then** return an MCP Error (`-32602` Invalid Params) stating "Namespace not found".
- **Given** a valid query, **When** invoked, **Then** invoke LanceDB search and return an array of `chunk_id`, `context`, and `distance`.

### 2.2 `get_context`

**Description:** Resolves a child chunk ID back to its parent context.
**Input Schema:**

```json
{
  "type": "object",
  "properties": {
    "chunk_id": { "type": "string" },
    "expand_level": { "type": "string", "enum": ["parent", "document"] }
  },
  "required": ["chunk_id"]
}
```

**Acceptance Criteria:**

- **Given** a valid `chunk_id` (e.g., `file.md_b1_c2`), **When** invoked, **Then** return the `context_payload` of the parent chunk.

### 2.3 `index_directory`

**Description:** Triggers a sync of a local directory into a LanceDB namespace.
**Input Schema:**

```json
{
  "type": "object",
  "properties": {
    "directory_path": { "type": "string" },
    "namespace": { "type": "string" }
  },
  "required": ["directory_path", "namespace"]
}
```

## 3. Resource Discovery (`resources/list`)

**Description:** The server MUST dynamically expose available namespaces as resources so AI agents can discover them without guessing.
**Acceptance Criteria:**

- **When** `resources/list` is called, **Then** the engine MUST query LanceDB for all table names (`db.table_names()`).
- The response MUST format these as resources (e.g., `uri: "intent://namespace/{table_name}"`).
