//! Indexer Pipeline Integration Suite
//!
//! Verifies the end-to-end ingestion and sync lifecycle:
//! Markdown AST chunking -> Arrow `RecordBatch` -> LanceDB storage -> FTS Index -> Delta Sync -> Orphan Purge.
//!
//! Design Intent References:
//! - ADR-002: Markdown AST Parsing and Small-to-Big Retrieval Strategy
//! - ADR-004: Differential State Manifest and Scoped Orphan Purging
//! - REQ-03: AST Chunking
//! - REQ-04: Sync Lifecycle

use super::common::TempTestDir;
use compiled_intent::indexer::{
    chunk_markdown, ArrowBatchBuilder, IndexChunk, LanceStorage, Manifest,
};

#[tokio::test]
async fn test_ast_chunking_with_sliding_window_integrity() {
    // Design Intent: ADR-002 (AST boundaries & small-to-big parent payload)
    let markdown = r#"
# Heading One

Paragraph inside heading one with several architectural rules.
Another sentence providing additional detail.

```rust
// Code block comment with # symbol should not split section
fn calculate_hash() -> u64 { 42 }
```

## Subheading Two

Short sub-section content.
"#;

    let blocks = chunk_markdown(markdown);
    assert_eq!(
        blocks.len(),
        2,
        "Code blocks with # must not create false AST sections"
    );

    // Parent Block 1
    assert_eq!(blocks[0].header, "Heading One");
    assert!(blocks[0].payload.contains("calculate_hash"));
    assert!(!blocks[0].children.is_empty());

    // All children of Parent 1 must be contained within Parent 1's payload
    for child in &blocks[0].children {
        assert!(blocks[0].payload.contains(child));
        assert!(
            !child.contains("Subheading Two"),
            "Child chunk must not bleed into next section"
        );
    }

    // Parent Block 2
    assert_eq!(blocks[1].header, "Subheading Two");
    assert!(blocks[1].payload.contains("Short sub-section content."));
}

#[tokio::test]
async fn test_arrow_batch_schema_conformance() {
    // Design Intent: ADR-001 (Embedded Arrow schema with scalar & vector columns)
    let chunks = vec![
        IndexChunk {
            chunk_id: "doc.md_b0_c0".to_string(),
            parent_id: Some("doc.md_b0".to_string()),
            filename: "doc.md".to_string(),
            vector_text: "Child text".to_string(),
            context_payload: "Full parent context".to_string(),
            metadata: r#"{"type":"spec"}"#.to_string(),
            vector: vec![0.5; 384],
        },
        IndexChunk {
            chunk_id: "doc.md_b0_c1".to_string(),
            parent_id: None, // Root fallback chunk
            filename: "doc.md".to_string(),
            vector_text: "Second child".to_string(),
            context_payload: "Full parent context".to_string(),
            metadata: r#"{"type":"spec"}"#.to_string(),
            vector: vec![0.1; 384],
        },
    ];

    let batch = ArrowBatchBuilder::build(chunks, 384).expect("Should build RecordBatch");
    assert_eq!(batch.num_rows(), 2);
    assert_eq!(batch.num_columns(), 7);

    let schema = batch.schema();
    let col_names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
    assert_eq!(
        col_names,
        vec![
            "chunk_id",
            "parent_id",
            "filename",
            "vector_text",
            "context_payload",
            "metadata",
            "vector"
        ]
    );
}

#[tokio::test]
async fn test_differential_sync_lifecycle_and_orphan_purge() {
    // Design Intent: ADR-004 (Differential Sync and Cache Invalidation)
    let temp_dir = TempTestDir::new("indexer_sync");
    let adrs_dir = temp_dir.sub_dir("specs");

    let file_1 = temp_dir.write_file("specs/001_core.md", "# Core Engine\n\nPure Rust core.");
    let file_2 = temp_dir.write_file("specs/002_mcp.md", "# MCP Server\n\nStdio wire format.");

    let adr_paths = vec![adrs_dir];
    let discovered = vec![file_1.clone(), file_2.clone()];

    let mut manifest = Manifest::new();

    // 1. First sync pass: both files are deltas
    let deltas_1 = manifest.compute_deltas(&discovered).unwrap();
    assert_eq!(
        deltas_1.len(),
        2,
        "Initial pass must report all files as deltas"
    );
    for d in &deltas_1 {
        manifest.update(d.filename.clone(), d.hash.clone());
    }

    // 2. Second sync pass: cache is warm, 0 deltas
    let deltas_2 = manifest.compute_deltas(&discovered).unwrap();
    assert_eq!(deltas_2.len(), 0, "Idempotent pass must report 0 deltas");

    // 3. Mutate one file: exactly 1 delta detected
    std::fs::write(
        &file_1,
        "# Core Engine\n\nPure Rust core with async Tokio runtime.",
    )
    .unwrap();
    let deltas_3 = manifest.compute_deltas(&discovered).unwrap();
    assert_eq!(
        deltas_3.len(),
        1,
        "Mutated file must be the only detected delta"
    );
    assert_eq!(deltas_3[0].filename, file_1.display().to_string());
    manifest.update(deltas_3[0].filename.clone(), deltas_3[0].hash.clone());

    // 4. Delete file_2: detected as orphan
    std::fs::remove_file(&file_2).unwrap();
    let remaining_discovered = vec![file_1.clone()];
    let orphans = manifest.find_orphans(&adr_paths, &remaining_discovered);
    assert_eq!(orphans.len(), 1, "Deleted file must be detected as orphan");
    assert_eq!(orphans[0], file_2.display().to_string());

    // 5. Purge orphan vectors from LanceDB storage
    assert_orphan_purge_in_storage(&temp_dir, &file_2).await;

    // Remove from manifest
    manifest.remove(&orphans[0]);
    assert!(!manifest.contains_file(&orphans[0]));
    assert!(manifest.contains_file(&file_1.display().to_string()));
}

async fn assert_orphan_purge_in_storage(temp_dir: &TempTestDir, orphan_file: &std::path::Path) {
    let lancedb_dir = temp_dir.sub_dir("lancedb");
    let db = lancedb::connect(&lancedb_dir.display().to_string())
        .execute()
        .await
        .unwrap();

    let orphan_chunk = IndexChunk {
        chunk_id: "specs/002_mcp.md_b0_c0".to_string(),
        parent_id: Some("specs/002_mcp.md_b0".to_string()),
        filename: orphan_file.display().to_string(),
        vector_text: "Stdio wire format".to_string(),
        context_payload: "Payload".to_string(),
        metadata: "{}".to_string(),
        vector: vec![0.0; 384],
    };

    let batch = ArrowBatchBuilder::build(vec![orphan_chunk], 384).unwrap();
    LanceStorage::write_batch(&db, "test_adrs", batch)
        .await
        .unwrap();

    let table = db.open_table("test_adrs").execute().await.unwrap();
    assert_eq!(table.count_rows(None).await.unwrap(), 1);

    let orphan_path_str = orphan_file.display().to_string();
    LanceStorage::invalidate_stale_files(&db, "test_adrs", &[&orphan_path_str]).await;

    let table_after = db.open_table("test_adrs").execute().await.unwrap();
    assert_eq!(
        table_after.count_rows(None).await.unwrap(),
        0,
        "Orphaned vectors must be purged"
    );
}

#[tokio::test]
async fn test_multi_directory_scoping_for_orphan_detection() {
    // Design Intent: ADR-004 (Orphan detection must be scoped to scanned directories)
    let temp_dir = TempTestDir::new("multi_dir_sync");
    let dir_a = temp_dir.sub_dir("agents_adrs");
    let _dir_b = temp_dir.sub_dir("docs_adrs");

    let file_a1 = temp_dir.write_file("agents_adrs/001.md", "# Agent Rule");
    let file_b1 = temp_dir.write_file("docs_adrs/002.md", "# Project Rule");

    let mut manifest = Manifest::new();
    manifest.update(file_a1.display().to_string(), "hash_a".to_string());
    manifest.update(file_b1.display().to_string(), "hash_b".to_string());

    // Delete file_a1 from agents_adrs, and scan ONLY dir_a
    std::fs::remove_file(&file_a1).unwrap();
    let adr_paths = vec![dir_a];
    let discovered = vec![]; // no files left in dir_a

    let orphans = manifest.find_orphans(&adr_paths, &discovered);
    // dir_a's deleted file must be detected as orphan
    assert_eq!(orphans.len(), 1);
    assert_eq!(orphans[0], file_a1.display().to_string());

    // dir_b's file must NOT be detected as an orphan because dir_b was not in adr_paths
    assert!(manifest.contains_file(&file_b1.display().to_string()));
}
