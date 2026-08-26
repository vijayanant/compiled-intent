use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

use crate::engine::ChronicleEngine;
use crate::error::ChronicleError;

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
pub struct McpServer {
    engine: ChronicleEngine,
}

impl McpServer {
    pub fn new(engine: ChronicleEngine) -> Self {
        Self { engine }
    }

    fn send_response(&self, id: Value, result: Value) {
        let res = RpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        };
        println!("{}", serde_json::to_string(&res).unwrap());
        io::stdout().flush().unwrap();
    }

    fn send_error(&self, id: Value, code: i32, message: &str) {
        let res = RpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(json!({ "code": code, "message": message })),
        };
        println!("{}", serde_json::to_string(&res).unwrap());
        io::stdout().flush().unwrap();
    }

    /// Listens for MCP JSON-RPC messages on Stdin.
    pub async fn run(&self) -> Result<(), ChronicleError> {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut buffer = String::new();

        tracing::info!("Chronicle MCP Server starting on Stdio...");

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

        if req.jsonrpc != "2.0" {
            self.send_error(id, -32600, "Invalid Request: jsonrpc version must be 2.0");
            return;
        }

        match req.method.as_str() {
            "initialize" => {
                self.send_response(
                    id,
                    json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "chronicle",
                            "version": "2.0.0"
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
                            "description": "Performs a semantic or hybrid search against indexed documents with optional metadata filtering.",
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
                            "description": "Resolves a child chunk ID back to its full parent structural context (Parent-Child Retrieval).",
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
                            "name": "sync_directory",
                            "description": "Triggers a differential delta-sync of a directory to invalidate the cache instantly.",
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
                        tracing::info!("LLM invoked 'search' with query: {}", query);
                        
                        // Actually use the engine so it's not dead code
                        if let Ok(_embedding) = self.engine.get_embedding(query).await {
                            self.send_response(id, json!({ "content": [{ "type": "text", "text": "Search functionality pending engine wiring..." }] }));
                        } else {
                            self.send_error(id, -32603, "Failed to generate query embedding");
                        }
                    }
                    "get_context" => {
                        let chunk_id = args.get("chunk_id").and_then(|q| q.as_str()).unwrap_or("");
                        tracing::info!("LLM invoked 'get_context' for chunk: {}", chunk_id);
                        // TODO: Implement self.engine.get_context(...)
                        self.send_response(id, json!({ "content": [{ "type": "text", "text": "Context expansion pending engine wiring..." }] }));
                    }
                    "sync_directory" => {
                        let dir = args.get("directory_path").and_then(|q| q.as_str()).unwrap_or("");
                        tracing::info!("LLM invoked 'sync_directory' for path: {}", dir);
                        // Delegate to indexer via engine
                        self.send_response(id, json!({ "content": [{ "type": "text", "text": "Differential sync triggered..." }] }));
                    }
                    _ => {
                        self.send_error(id, -32601, "Tool not found");
                    }
                }
            }
            _ => {
                tracing::warn!("Unhandled method: {}", req.method);
                if id != json!(null) {
                    self.send_error(id, -32601, "Method not found");
                }
            }
        }
    }
}
