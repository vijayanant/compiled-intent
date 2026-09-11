pub mod tools;

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

const JSONRPC_VERSION: &str = "2.0";
const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_VERSION: &str = "2.0.0";

pub struct McpServer {
    engine: CompitentEngine,
}

impl McpServer {
    pub fn new(engine: CompitentEngine) -> Self {
        Self { engine }
    }

    /// Processes a single raw JSON-RPC string message and returns the serialized response if applicable.
    pub async fn process_message(&self, raw_json: &str) -> Option<String> {
        let req: RpcRequest = match serde_json::from_str(raw_json) {
            Ok(r) => r,
            Err(e) => {
                let err_res = RpcResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    id: json!(null),
                    result: None,
                    error: Some(json!({ "code": -32700, "message": format!("Parse error: {}", e) })),
                };
                return Some(serde_json::to_string(&err_res).unwrap());
            }
        };

        let id = req.id.clone().unwrap_or(json!(null));

        if req.jsonrpc != JSONRPC_VERSION {
            let res = RpcResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                id,
                result: None,
                error: Some(json!({ "code": -32600, "message": "Invalid Request: jsonrpc version must be 2.0" })),
            };
            return Some(serde_json::to_string(&res).unwrap());
        }

        match req.method.as_str() {
            "initialize" => {
                let res = RpcResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    id,
                    result: Some(json!({
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "intent",
                            "version": SERVER_VERSION
                        }
                    })),
                    error: None,
                };
                Some(serde_json::to_string(&res).unwrap())
            }
            "notifications/initialized" => {
                tracing::info!("Client initialized.");
                None
            }
            "tools/list" => {
                let res = RpcResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    id,
                    result: Some(tools::tool_definitions()),
                    error: None,
                };
                Some(serde_json::to_string(&res).unwrap())
            }
            "tools/call" => {
                let params = req.params.unwrap_or_default();
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                let (result, error) = match tools::dispatch_tool_call(&self.engine, name, &args).await {
                    Ok(val) => (Some(val), None),
                    Err((code, msg)) => (None, Some(json!({ "code": code, "message": msg }))),
                };

                let res = RpcResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    id,
                    result,
                    error,
                };
                Some(serde_json::to_string(&res).unwrap())
            }
            _ => {
                tracing::warn!("Unhandled method: {}", req.method);
                let res = RpcResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    id,
                    result: None,
                    error: Some(json!({ "code": tools::JSONRPC_METHOD_NOT_FOUND, "message": "Method not supported" })),
                };
                Some(serde_json::to_string(&res).unwrap())
            }
        }
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

            if let Some(resp) = self.process_message(&buffer).await {
                println!("{}", resp);
                io::stdout().flush().unwrap();
            }

            buffer.clear();
        }

        Ok(())
    }
}
