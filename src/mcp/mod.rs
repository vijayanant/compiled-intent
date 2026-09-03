use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

use crate::engine::CompitentEngine;
use crate::error::CompitentError;

// --- MCP Protocol Types ---

#[derive(Deserialize, Debug)]
struct RpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize)]
struct RpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

/// The MCP Server router.
/// Handles the JSON-RPC protocol over Stdio.
const JSONRPC_METHOD_NOT_FOUND: i32 = -32601;
const JSONRPC_INTERNAL_ERROR: i32 = -32603;

const JSONRPC_VERSION: &str = "2.0";
const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_VERSION: &str = "2.0.0";
const DEFAULT_SEARCH_LIMIT: usize = 3;

pub struct McpServer {
    engine: CompitentEngine,
}

impl McpServer {
    pub fn new(engine: CompitentEngine) -> Self {
        Self { engine }
    }

    fn send_response(&self, id: Value, result: Value) {
        let res = RpcResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        };
        println!("{}", serde_json::to_string(&res).unwrap());
        io::stdout().flush().unwrap();
    }

    fn send_error(&self, id: Value, code: i32, message: &str) {
        let res = RpcResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(json!({ "code": code, "message": message })),
        };
        println!("{}", serde_json::to_string(&res).unwrap());
        io::stdout().flush().unwrap();
    }

    /// Listens for MCP JSON-RPC messages on Stdin.
    pub async fn run(&self) -> Result<(), CompitentError> {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut buffer = String::new();

        tracing::info!("Compitent MCP Server starting on Stdio...");

        while handle.read_line(&mut buffer)? > 0 {
            if buffer.trim().is_empty() {
                buffer.clear();
                continue;
            }

            match serde_json::from_str::<RpcRequest>(&buffer) {
                Ok(req) => {
                    self.handle_request(req).await;
                }
                Err(e) => {
                    tracing::error!("Invalid JSON payload received: {}", e);
                }
            }

            buffer.clear();
        }

        Ok(())
    }

    async fn handle_request(&self, req: RpcRequest) {
        let id = req.id.clone().unwrap_or(json!(null));

        if req.jsonrpc != JSONRPC_VERSION {
            self.send_error(id, -32600, "Invalid Request: jsonrpc version must be 2.0");
            return;
        }

        match req.method.as_str() {
            "initialize" => {
                self.send_response(
                    id,
                    json!({
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "intent",
                            "version": SERVER_VERSION
                        }
                    }),
                );
            }
            "notifications/initialized" => {
                tracing::info!("Client initialized.");
            }
            "tools/list" => {
                // Expose our Universal Semantic Filesystem capabilities
                self.send_response(id, json!({
                    "tools": [
                        {
                            "name": "search",
                            "description": "Performs a semantic or hybrid search against indexed documents. Returns a JSON array of objects with keys: 'chunk_id' (string), 'context' (string), and 'distance' (float). Note: because this uses LanceDB Hybrid Search, the distance metric is derived from Reciprocal Rank Fusion, meaning higher distance scores actually indicate WORSE relevance (a distance of 0.0 is a perfect match).",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": { "type": "string" },
                                    "namespace": { "type": "string", "description": "Target LanceDB collection" },
                                    "mode": { "type": "string", "enum": ["vector", "fts", "hybrid"] },
                                    "metadata_filter": { "type": "string", "description": "SQL filter for JSON metadata" },
                                    "limit": { "type": "integer" }
                                },
                                "required": ["query"]
                            }
                        },
                        {
                            "name": "get_context",
                            "description": "Resolves a child chunk ID back to its full parent structural context. Returns a single raw string containing the full markdown file content. Use this when a search result looks promising but you need to read the surrounding text before making a decision.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "chunk_id": { "type": "string" },
                                    "expand_level": { "type": "string", "enum": ["parent", "siblings", "document"] }
                                },
                                "required": ["chunk_id"]
                            }
                        },
                        {
                            "name": "index_directory",
                            "description": "Scans a directory for markdown files and updates the semantic LanceDB index.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "directory_path": { "type": "string" },
                                    "namespace": { "type": "string" },
                                    "chunking_strategy": { "type": "string", "enum": ["ast", "sliding_window"] }
                                },
                                "required": ["directory_path", "namespace"]
                            }
                        }
                    ]
                }));
            }
            "tools/call" => {
                let params = req.params.unwrap_or_default();
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                match name {
                    "search" => {
                        let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
                        let limit = args.get("limit").and_then(|l| l.as_u64()).map(|l| l as usize).unwrap_or(DEFAULT_SEARCH_LIMIT);
                        let mode = args.get("mode").and_then(|m| m.as_str()).unwrap_or("vector");
                        tracing::info!("LLM invoked 'search' in {} mode with query: {}", mode, query);
                        
                        match self.engine.search(query, limit, mode).await {
                            Ok(results) => self.send_response(id, json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&results).unwrap() }] })),
                            Err(e) => self.send_error(id, JSONRPC_INTERNAL_ERROR, &format!("Search failed: {}", e))
                        }
                    }
                    "get_context" => {
                        let chunk_id = args.get("chunk_id").and_then(|q| q.as_str()).unwrap_or("");
                        tracing::info!("LLM invoked 'get_context' for chunk: {}", chunk_id);
                        
                        match self.engine.get_context(chunk_id).await {
                            Ok(text) => self.send_response(id, json!({ "content": [{ "type": "text", "text": text }] })),
                            Err(e) => self.send_error(id, JSONRPC_INTERNAL_ERROR, &format!("Context expansion failed: {}", e))
                        }
                    }
                    "index_directory" => {
                        let dir = args.get("directory_path").and_then(|q| q.as_str()).unwrap_or("");
                        tracing::info!("LLM invoked 'index_directory' for path: {}", dir);
                        
                        match crate::indexer::index_adrs(&self.engine, &[dir.to_string()]).await {
                            Ok(_) => self.send_response(id, json!({ "content": [{ "type": "text", "text": "Indexing successful!" }] })),
                            Err(e) => self.send_error(id, JSONRPC_INTERNAL_ERROR, &format!("Indexing failed: {}", e))
                        }
                    }
                    _ => {
                        self.send_error(id, JSONRPC_METHOD_NOT_FOUND, "Tool not found");
                    }
                }
            }
            _ => {
                tracing::warn!("Unhandled method: {}", req.method);
                self.send_error(id, JSONRPC_METHOD_NOT_FOUND, "Method not supported");
            }
        }
    }
}
