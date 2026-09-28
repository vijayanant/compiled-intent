pub mod tools;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
                    error: Some(
                        json!({ "code": -32700, "message": format!("Parse error: {}", e) }),
                    ),
                };
                return Some(serde_json::to_string(&err_res).unwrap());
            }
        };

        self.handle_request(req)
            .await
            .map(|res| serde_json::to_string(&res).unwrap())
    }

    fn initialize_response(id: Value) -> RpcResponse {
        RpcResponse {
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
        }
    }

    async fn handle_tools_call(&self, id: Value, params: Option<Value>) -> RpcResponse {
        let params = params.unwrap_or_default();
        let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        let (result, error) = match tools::dispatch_tool_call(&self.engine, name, &args).await {
            Ok(val) => (Some(val), None),
            Err((code, msg)) => (None, Some(json!({ "code": code, "message": msg }))),
        };

        RpcResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result,
            error,
        }
    }

    async fn handle_request(&self, req: RpcRequest) -> Option<RpcResponse> {
        let is_notification = req.id.is_none();
        let id = req.id.clone().unwrap_or(json!(null));

        if req.jsonrpc != JSONRPC_VERSION {
            if is_notification {
                return None;
            }
            return Some(RpcResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                id,
                result: None,
                error: Some(
                    json!({ "code": -32600, "message": "Invalid Request: jsonrpc version must be 2.0" }),
                ),
            });
        }

        match req.method.as_str() {
            "initialize" => Some(Self::initialize_response(id)),
            "notifications/initialized" => {
                tracing::info!("Client initialized.");
                None
            }
            "ping" => Some(RpcResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                id,
                result: Some(json!({})),
                error: None,
            }),
            "tools/list" => Some(RpcResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                id,
                result: Some(tools::tool_definitions()),
                error: None,
            }),
            "tools/call" => Some(self.handle_tools_call(id, req.params).await),
            _ => Self::unhandled_method_response(&req.method, is_notification, id),
        }
    }

    fn unhandled_method_response(
        method: &str,
        is_notification: bool,
        id: serde_json::Value,
    ) -> Option<RpcResponse> {
        if is_notification || method.starts_with("notifications/") {
            tracing::debug!("Ignoring unhandled notification: {}", method);
            None
        } else {
            tracing::warn!("Unhandled method: {}", method);
            Some(RpcResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                id,
                result: None,
                error: Some(
                    json!({ "code": tools::JSONRPC_METHOD_NOT_FOUND, "message": "Method not supported" }),
                ),
            })
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
