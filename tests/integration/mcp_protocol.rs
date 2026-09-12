//! MCP Protocol Integration Suite
//!
//! Verifies JSON-RPC 2.0 protocol compliance, tool dispatch, error formatting,
//! and stdio isolation.
//!
//! Design Intent References:
//! - ADR-005: Headless Stdio MCP Protocol and I/O Isolation
//! - ADR-006: Unified Binary Architecture and CLI Topology
//! - REQ-01: MCP Interface

use super::common::{create_test_engine, TempTestDir};
use compiled_intent::mcp::McpServer;
use serde_json::{json, Value};

#[tokio::test]
async fn test_mcp_initialize_handshake() {
    // Design Intent: ADR-005 & ADR-006
    let temp_dir = TempTestDir::new("mcp_init");
    let engine = create_test_engine(&temp_dir);
    let server = McpServer::new(engine);

    let req = json!({
        "jsonrpc": "2.0",
        "id": "init-1",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    })
    .to_string();

    let resp_str = server
        .process_message(&req)
        .await
        .expect("Server must reply to initialize handshake");
    let resp: Value =
        serde_json::from_str(&resp_str).expect("Initialize response must be strictly valid JSON");

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], "init-1");
    assert_eq!(resp["result"]["serverInfo"]["name"], "intent");
    assert!(resp["result"]["capabilities"]["tools"].is_object());
}

#[tokio::test]
async fn test_mcp_tools_list_schema() {
    // Design Intent: ADR-005 & REQ-01 (Tools list discovery)
    let temp_dir = TempTestDir::new("mcp_tools");
    let engine = create_test_engine(&temp_dir);
    let server = McpServer::new(engine);

    let req = json!({
        "jsonrpc": "2.0",
        "id": 100,
        "method": "tools/list"
    })
    .to_string();

    let resp_str = server.process_message(&req).await.unwrap();
    let resp: Value = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp["id"], 100);
    let tools = resp["result"]["tools"]
        .as_array()
        .expect("Tools must be a list");
    assert_eq!(tools.len(), 3);

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(tool_names.contains(&"search"));
    assert!(tool_names.contains(&"get_context"));
    assert!(tool_names.contains(&"index_directory"));
}

#[tokio::test]
async fn test_mcp_get_context_tool_call() {
    // Design Intent: ADR-002 & REQ-01 (Parent Context Expansion via MCP)
    let temp_dir = TempTestDir::new("mcp_ctx");
    let spec_path = temp_dir.write_file(
        "auth_spec.md",
        "# Security Boundary\n\nAll JWTs must use Ed25519 signatures.",
    );

    let engine = create_test_engine(&temp_dir);
    let server = McpServer::new(engine);

    let req = json!({
        "jsonrpc": "2.0",
        "id": 201,
        "method": "tools/call",
        "params": {
            "name": "get_context",
            "arguments": {
                "chunk_id": format!("{}_b0_c0", spec_path.display()),
                "expand_level": "parent"
            }
        }
    })
    .to_string();

    let resp_str = server.process_message(&req).await.unwrap();
    let resp: Value = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp["id"], 201);
    let content = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(content.contains("All JWTs must use Ed25519 signatures."));
}

#[tokio::test]
async fn test_mcp_error_conformance() {
    // Design Intent: ADR-005 & REQ-01 (Stdio separation & standard JSON-RPC 2.0 error codes)
    let temp_dir = TempTestDir::new("mcp_err");
    let engine = create_test_engine(&temp_dir);
    let server = McpServer::new(engine);

    // 1. Parse Error (-32700)
    let bad_json = "NOT_VALID_JSON";
    let resp1: Value =
        serde_json::from_str(&server.process_message(bad_json).await.unwrap()).unwrap();
    assert_eq!(resp1["error"]["code"], -32700);

    // 2. Invalid Request (-32600)
    let bad_req = json!({
        "jsonrpc": "1.0", // invalid version
        "id": 1,
        "method": "tools/list"
    })
    .to_string();
    let resp2: Value =
        serde_json::from_str(&server.process_message(&bad_req).await.unwrap()).unwrap();
    assert_eq!(resp2["error"]["code"], -32600);

    // 3. Method Not Found (-32601)
    let missing_method = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "non_existent_rpc_call"
    })
    .to_string();
    let resp3: Value =
        serde_json::from_str(&server.process_message(&missing_method).await.unwrap()).unwrap();
    assert_eq!(resp3["error"]["code"], -32601);
}

#[tokio::test]
async fn test_mcp_notification_emits_no_response() {
    // Design Intent: ADR-005 & REQ-01 (Notifications MUST NOT produce a response on stdout)
    let temp_dir = TempTestDir::new("mcp_notif");
    let engine = create_test_engine(&temp_dir);
    let server = McpServer::new(engine);

    let notif_req = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    })
    .to_string();

    let resp = server.process_message(&notif_req).await;
    assert!(
        resp.is_none(),
        "JSON-RPC notifications must never receive a response"
    );
}
