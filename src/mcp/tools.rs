use crate::engine::CompitentEngine;
use serde_json::{json, Value};

const DEFAULT_SEARCH_LIMIT: usize = 3;
pub const JSONRPC_METHOD_NOT_FOUND: i32 = -32601;
pub const JSONRPC_INTERNAL_ERROR: i32 = -32603;

pub fn tool_definitions() -> Value {
    json!({
        "tools": [
            {
                "name": "search",
                "description": "Performs a semantic, full-text, or hybrid search against indexed documents. Returns a JSON array of objects with keys: 'chunk_id' (string), 'context' (string), and 'score' (float in range [0.0, 1.0]). Higher scores indicate greater relevance (1.0 is maximum relevance). Results are sorted in descending score order.",
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
    })
}

async fn handle_search(engine: &CompitentEngine, args: &Value) -> Result<Value, (i32, String)> {
    let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
    let limit = args
        .get("limit")
        .and_then(|l| l.as_u64())
        .map(|l| l as usize)
        .unwrap_or(DEFAULT_SEARCH_LIMIT);
    let mode = args
        .get("mode")
        .and_then(|m| m.as_str())
        .unwrap_or("vector");

    tracing::info!(
        "LLM invoked 'search' in {} mode with query: {}",
        mode,
        query
    );

    engine
        .search(query, limit, mode)
        .await
        .map(|results| {
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&results).unwrap_or_default()
                }]
            })
        })
        .map_err(|e| (JSONRPC_INTERNAL_ERROR, format!("Search failed: {}", e)))
}

async fn handle_get_context(
    engine: &CompitentEngine,
    args: &Value,
) -> Result<Value, (i32, String)> {
    let chunk_id = args.get("chunk_id").and_then(|q| q.as_str()).unwrap_or("");
    tracing::info!("LLM invoked 'get_context' for chunk: {}", chunk_id);

    engine
        .get_context(chunk_id)
        .await
        .map(|text| {
            json!({
                "content": [{
                    "type": "text",
                    "text": text
                }]
            })
        })
        .map_err(|e| {
            (
                JSONRPC_INTERNAL_ERROR,
                format!("Context expansion failed: {}", e),
            )
        })
}

async fn handle_index_directory(
    engine: &CompitentEngine,
    args: &Value,
) -> Result<Value, (i32, String)> {
    let dir = args
        .get("directory_path")
        .and_then(|q| q.as_str())
        .unwrap_or("");
    tracing::info!("LLM invoked 'index_directory' for path: {}", dir);

    crate::indexer::index_adrs(engine, &[dir])
        .await
        .map(|_| {
            json!({
                "content": [{
                    "type": "text",
                    "text": "Indexing successful!"
                }]
            })
        })
        .map_err(|e| (JSONRPC_INTERNAL_ERROR, format!("Indexing failed: {}", e)))
}

pub async fn dispatch_tool_call(
    engine: &CompitentEngine,
    name: &str,
    args: &Value,
) -> Result<Value, (i32, String)> {
    match name {
        "search" => handle_search(engine, args).await,
        "get_context" => handle_get_context(engine, args).await,
        "index_directory" => handle_index_directory(engine, args).await,
        _ => Err((JSONRPC_METHOD_NOT_FOUND, "Tool not found".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EngineConfig;

    #[tokio::test]
    async fn test_dispatch_tool_call_validation_and_errors() {
        let temp_dir =
            std::env::temp_dir().join(format!("intent_tool_test_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = CompitentEngine::new(EngineConfig {
            llama_endpoint: "http://localhost:8080".to_string(),
            embedding_model: "test".to_string(),
            embedding_dimensions: 384,
            intent_dir: temp_dir.display().to_string(),
            lancedb_path: temp_dir.join("db").display().to_string(),
        });

        // 1. Unknown tool returns JSONRPC_METHOD_NOT_FOUND (-32601)
        let err = dispatch_tool_call(&engine, "non_existent_tool", &json!({}))
            .await
            .expect_err("Unknown tool must fail");
        assert_eq!(err.0, JSONRPC_METHOD_NOT_FOUND);
        assert_eq!(err.1, "Tool not found");

        // 2. get_context on missing file returns JSONRPC_INTERNAL_ERROR (-32603)
        let err2 = dispatch_tool_call(
            &engine,
            "get_context",
            &json!({ "chunk_id": "non_existent_file.md_b0_c0" }),
        )
        .await
        .expect_err("Missing file must return error");
        assert_eq!(err2.0, JSONRPC_INTERNAL_ERROR);
        assert!(err2.1.contains("Context expansion failed"));

        // 3. get_context on existing file returns standard MCP text payload
        let valid_file = temp_dir.join("doc.md");
        std::fs::write(&valid_file, "# Rule\n\nContent payload").unwrap();
        let ok_res = dispatch_tool_call(
            &engine,
            "get_context",
            &json!({ "chunk_id": format!("{}_b0_c0", valid_file.display()) }),
        )
        .await
        .expect("Existing file must succeed");

        assert_eq!(ok_res["content"][0]["type"], "text");
        assert!(ok_res["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Content payload"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
