# 01-REQ-MCP-INTERFACE: Model Context Protocol (MCP) Contract

> [!NOTE] **Architectural Rationale**
> For the technical evaluation of Stdio transport versus local HTTP/SSE daemons, see [ADR-005: Headless Stdio MCP Protocol and I/O Isolation](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/005-headless-stdio-mcp-protocol.md).

## 1. Transport & Wire Protocol

The engine MUST expose a standard Model Context Protocol (MCP) server communicating over Standard Input/Output (`stdio`) using newline-delimited JSON-RPC 2.0 framing.

### 1.1 Stdio Isolation Invariant

- **Standard Output (`stdout`)**: Strictly reserved for valid, serialized JSON-RPC 2.0 response objects. Under no circumstances may application logic, third-party libraries, or diagnostic statements print to `stdout`.
- **Standard Error (`stderr`)**: The exclusive channel for telemetry, diagnostics, and debugging. All `tracing` subscriber logs MUST write to `stderr`.

## 2. Core RPC Methods

### 2.1 `initialize`

- **Request**: `{ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { ... } }`
- **Response**:

  ```json
  {
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
      "protocolVersion": "2024-11-05",
      "capabilities": { "tools": {} },
      "serverInfo": { "name": "intent", "version": "0.1.0" }
    }
  }
  ```

### 2.2 `notifications/initialized`

- Client acknowledgement notification. The server MUST process this notification and emit **no response**.

### 2.3 `tools/list`

- Emits the catalog of supported MCP tools.
- Output MUST conform to the MCP Tools Specification containing schemas for `search`, `get_context`, and `index_directory`.

### 2.4 `tools/call`

- Dispatches tool invocations by name:
  - If tool succeeds: returns `{ "content": [{ "type": "text", "text": "..." }] }`.
  - If tool fails: returns JSON-RPC error code and descriptive message.

## 3. Tool Specifications

### 3.1 `search`

Searches project documentation using vector, full-text, or hybrid retrieval.

- **Arguments**:
  - `query` (string, required): The search stimulus or technical query.
  - `limit` (integer, optional, default: `5`): Maximum parent chunks to return.
  - `mode` (string, optional, enum: `["hybrid", "fts", "vector"]`, default: `"hybrid"`): Retrieval modality.
  - `namespace` (string, optional): Target directory namespace filter (e.g. `"specs"`, `"adrs"`).
- **Return Payload**:
  Returns a serialized JSON array of chunk match objects:
  ```json
  [
    {
      "chunk_id": "docs/adrs/001.md_b0_c0",
      "context": "Source: docs/adrs/001.md\nSection: Decision\n...",
      "distance": 0.0
    }
  ]
  ```

### 3.2 `get_context`

Expands and retrieves the complete, raw parent file content associated with a chunk ID.

- **Arguments**:
  - `chunk_id` (string, required): The chunk identifier (e.g. `"docs/specs/01-mcp.md_b0_c0"`).

### 3.3 `index_directory`

Forces an on-demand differential indexing pass over a target directory.

- **Arguments**:
  - `directory_path` (string, required): Filesystem path to scan and index.

## 4. Standard Error Codes

| Code | Constant | Meaning |
| :---: | :--- | :--- |
| `-32700` | `PARSE_ERROR` | Malformed JSON received on `stdin`. |
| `-32600` | `INVALID_REQUEST` | Missing or invalid `jsonrpc: "2.0"` field. |
| `-32601` | `METHOD_NOT_FOUND` | Unrecognized method name or unknown tool name. |
| `-32603` | `INTERNAL_ERROR` | Storage query error, embedding API failure, or filesystem I/O error. |
