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
                self.send_response(id, tools::tool_definitions());
            }
            "tools/call" => {
                let params = req.params.unwrap_or_default();
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                match tools::dispatch_tool_call(&self.engine, name, &args).await {
                    Ok(res) => self.send_response(id, res),
                    Err((code, msg)) => self.send_error(id, code, &msg),
                }
            }
            _ => {
                tracing::warn!("Unhandled method: {}", req.method);
                self.send_error(id, tools::JSONRPC_METHOD_NOT_FOUND, "Method not supported");
            }
        }
    }
}
