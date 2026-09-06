use serde_json::{json, Value};
use crate::engine::CompitentEngine;

const DEFAULT_SEARCH_LIMIT: usize = 3;
pub const JSONRPC_METHOD_NOT_FOUND: i32 = -32601;
pub const JSONRPC_INTERNAL_ERROR: i32 = -32603;

pub fn tool_definitions() -> Value {
    json!({
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
    })
}

pub async fn dispatch_tool_call(
    engine: &CompitentEngine,
    name: &str,
    args: &Value,
) -> Result<Value, (i32, String)> {
    match name {
        "search" => {
            let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
            let limit = args
                .get("limit")
                .and_then(|l| l.as_u64())
                .map(|l| l as usize)
                .unwrap_or(DEFAULT_SEARCH_LIMIT);
            let mode = args.get("mode").and_then(|m| m.as_str()).unwrap_or("vector");

            tracing::info!("LLM invoked 'search' in {} mode with query: {}", mode, query);

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
        "get_context" => {
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
                .map_err(|e| (JSONRPC_INTERNAL_ERROR, format!("Context expansion failed: {}", e)))
        }
        "index_directory" => {
            let dir = args.get("directory_path").and_then(|q| q.as_str()).unwrap_or("");
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
        _ => Err((JSONRPC_METHOD_NOT_FOUND, "Tool not found".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definitions_schema() {
        let tools = tool_definitions();
        let list = tools["tools"].as_array().expect("Tools must be an array");
        assert_eq!(list.len(), 3);
        let names: Vec<&str> = list.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"search"));
        assert!(names.contains(&"get_context"));
        assert!(names.contains(&"index_directory"));
    }
}
