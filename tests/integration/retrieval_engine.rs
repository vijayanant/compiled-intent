//! Retrieval Engine Integration Suite
//!
//! Verifies search modalities, exact code symbol lookups via Tantivy FTS,
//! and parent context expansion via `get_context`.
//!
//! Design Intent References:
//! - ADR-002: Markdown AST Parsing and Small-to-Big Retrieval Strategy
//! - ADR-003: Tri-Modal Retrieval and Reciprocal Rank Fusion (RRF)
//! - REQ-02: Storage and Retrieval

use super::common::{create_test_engine, TempTestDir};
use arrow::array::StringArray;
use compiled_intent::indexer::{ArrowBatchBuilder, IndexChunk, LanceStorage};
use futures::stream::StreamExt;
use lance_index::scalar::FullTextSearchQuery;
use lancedb::query::{ExecutableQuery, QueryBase};

async fn setup_fts_test_table(db: &lancedb::Connection) -> lancedb::Table {
    let chunk_a = IndexChunk {
        chunk_id: "sec.md_b0_c0".to_string(),
        parent_id: Some("sec.md_b0".to_string()),
        filename: "sec.md".to_string(),
        vector_text: "SessionLedger struct for audit trail".to_string(),
        context_payload:
            "Source: sec.md\nSection: Security\n\npub struct SessionLedger { id: u64 }".to_string(),
        metadata: "{}".to_string(),
        vector: vec![0.1; 384],
    };

    let chunk_b = IndexChunk {
        chunk_id: "sec.md_b1_c0".to_string(),
        parent_id: Some("sec.md_b1".to_string()),
        filename: "sec.md".to_string(),
        vector_text: "User authentication middleware tokens".to_string(),
        context_payload: "Source: sec.md\nSection: Auth\n\nBearer token authorization logic."
            .to_string(),
        metadata: "{}".to_string(),
        vector: vec![0.2; 384],
    };

    let batch = ArrowBatchBuilder::build(vec![chunk_a, chunk_b], 384).unwrap();
    let table = LanceStorage::write_batch(db, "test_symbols", batch)
        .await
        .unwrap();
    LanceStorage::build_fts_index(&table, "context_payload")
        .await
        .unwrap();
    table
}

#[tokio::test]
async fn test_exact_symbol_retrieval_via_fts() {
    // Design Intent: ADR-003 & REQ-02 (Full-Text Search prevents vector dilution on exact code symbols)
    let temp_dir = TempTestDir::new("retrieval_fts");
    let lancedb_dir = temp_dir.sub_dir("lancedb");

    let db = lancedb::connect(&lancedb_dir.display().to_string())
        .execute()
        .await
        .unwrap();

    let table = setup_fts_test_table(&db).await;

    // Query specifically for the struct name "SessionLedger"
    let mut stream = table
        .query()
        .full_text_search(FullTextSearchQuery::new("SessionLedger".to_string()))
        .limit(5)
        .execute()
        .await
        .unwrap();

    let mut matched_payloads = Vec::new();
    while let Some(batch_result) = stream.next().await {
        let b = batch_result.unwrap();
        let col = b
            .column_by_name("context_payload")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .unwrap();
        for i in 0..b.num_rows() {
            matched_payloads.push(col.value(i).to_string());
        }
    }

    assert_eq!(
        matched_payloads.len(),
        1,
        "FTS must return exact symbol match"
    );
    assert!(matched_payloads[0].contains("pub struct SessionLedger"));
    assert!(!matched_payloads[0].contains("Bearer token"));
}

#[tokio::test]
async fn test_get_context_parent_file_expansion() {
    // Design Intent: ADR-002 & REQ-03 (Context expansion resolves child needle back to full parent file)
    let temp_dir = TempTestDir::new("retrieval_ctx");
    let full_content = r#"---
title: Cryptographic Standards
status: accepted
---

# Cryptographic Primitives

All token signing operations MUST use Ed25519 or ES256.
RS256 is deprecated and prohibited in production environments.
"#;
    let file_path = temp_dir.write_file("crypto_spec.md", full_content);
    let engine = create_test_engine(&temp_dir);

    let chunk_id = format!("{}_b0_c0", file_path.display());
    let expanded_context = engine
        .get_context(&chunk_id)
        .await
        .expect("get_context must resolve parent file");

    assert!(expanded_context.contains("Cryptographic Primitives"));
    assert!(expanded_context.contains("Ed25519"));
    assert!(expanded_context.contains("RS256 is deprecated"));
}

#[tokio::test]
async fn test_zero_search_results_handling() {
    // Design Intent: ADR-003 & REQ-02 (Empty search results must return [] cleanly without panic)
    let temp_dir = TempTestDir::new("retrieval_empty");
    let lancedb_dir = temp_dir.sub_dir("lancedb");
    let db = lancedb::connect(&lancedb_dir.display().to_string())
        .execute()
        .await
        .unwrap();

    let chunk = IndexChunk {
        chunk_id: "spec.md_b0_c0".to_string(),
        parent_id: Some("spec.md_b0".to_string()),
        filename: "spec.md".to_string(),
        vector_text: "KnownSymbol".to_string(),
        context_payload: "Known content".to_string(),
        metadata: "{}".to_string(),
        vector: vec![0.1; 384],
    };
    let batch = ArrowBatchBuilder::build(vec![chunk], 384).unwrap();
    let table = LanceStorage::write_batch(&db, "empty_search_test", batch)
        .await
        .unwrap();
    LanceStorage::build_fts_index(&table, "context_payload")
        .await
        .unwrap();

    let mut stream = table
        .query()
        .full_text_search(FullTextSearchQuery::new(
            "NonExistentSymbolXYZ999".to_string(),
        ))
        .limit(5)
        .execute()
        .await
        .unwrap();

    let mut count = 0;
    while let Some(batch_result) = stream.next().await {
        let b = batch_result.unwrap();
        count += b.num_rows();
    }

    assert_eq!(
        count, 0,
        "Query for non-existent symbol must yield 0 results safely"
    );
}

async fn setup_score_contract_table(db: &lancedb::Connection) {
    let chunk_a = IndexChunk {
        chunk_id: "doc.md_b0_c0".to_string(),
        parent_id: Some("doc.md_b0".to_string()),
        filename: "doc.md".to_string(),
        vector_text: "SessionLedger high priority token".to_string(),
        context_payload: "SessionLedger audit log implementation details".to_string(),
        metadata: "{}".to_string(),
        vector: vec![0.1; 384],
    };
    let chunk_b = IndexChunk {
        chunk_id: "doc.md_b1_c0".to_string(),
        parent_id: Some("doc.md_b1".to_string()),
        filename: "doc.md".to_string(),
        vector_text: "SessionLedger secondary reference".to_string(),
        context_payload: "SessionLedger secondary reference notes".to_string(),
        metadata: "{}".to_string(),
        vector: vec![0.2; 384],
    };

    let batch = ArrowBatchBuilder::build(vec![chunk_a, chunk_b], 384).unwrap();
    let table = LanceStorage::write_batch(db, "adrs", batch).await.unwrap();
    LanceStorage::build_fts_index(&table, "context_payload")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_unified_normalized_score_contract() {
    // Design Intent: ADR-009 & REQ-01 & REQ-02
    // Verifies all modalities return "score" in [0.0, 1.0], never "distance", and sort descending
    let temp_dir = TempTestDir::new("retrieval_score_contract");
    let lancedb_dir = temp_dir.sub_dir("lancedb");
    let db = lancedb::connect(&lancedb_dir.display().to_string())
        .execute()
        .await
        .unwrap();

    setup_score_contract_table(&db).await;

    let engine = create_test_engine(&temp_dir);
    let results = engine
        .search("SessionLedger", 5, "fts")
        .await
        .expect("search must succeed");

    assert!(!results.is_empty(), "Must return at least 1 match");

    let mut previous_score = 1.0;
    for result in &results {
        // Assert distance key is forbidden
        assert!(
            result.get("distance").is_none(),
            "Legacy 'distance' key must NOT exist in search response"
        );

        // Assert score exists and is bounded in [0.0, 1.0]
        let score = result
            .get("score")
            .and_then(|v| v.as_f64())
            .expect("Result must have a numeric 'score' field") as f32;

        assert!(
            (0.0..=1.0).contains(&score),
            "Score must be normalized in [0.0, 1.0], got {}",
            score
        );

        // Assert descending sort order
        assert!(
            score <= previous_score,
            "Results must be sorted descending by score"
        );
        previous_score = score;
    }
}

#[tokio::test]
async fn test_parent_chunk_collapsing_deduplication() {
    // Design Intent: ADR-002 & REQ-02
    // Sibling child chunks sharing the same parent_id must collapse into the single
    // highest-scoring hit, preventing sibling crowding in top-k retrieval.
    let temp_dir = TempTestDir::new("retrieval_collapse");
    let lancedb_dir = temp_dir.sub_dir("lancedb");
    let db = lancedb::connect(&lancedb_dir.display().to_string())
        .execute()
        .await
        .unwrap();

    let chunk_a1 = IndexChunk {
        chunk_id: "doc_a.md_b0_c0".to_string(),
        parent_id: Some("doc_a.md_b0".to_string()),
        filename: "doc_a.md".to_string(),
        vector_text: "ArchitectureConstraint core rule".to_string(),
        context_payload: "Source: doc_a.md\nSection: Architecture\n\nArchitectureConstraint rule 1".to_string(),
        metadata: "{}".to_string(),
        vector: vec![0.1; 384],
    };
    let chunk_a2 = IndexChunk {
        chunk_id: "doc_a.md_b0_c1".to_string(),
        parent_id: Some("doc_a.md_b0".to_string()),
        filename: "doc_a.md".to_string(),
        vector_text: "ArchitectureConstraint secondary elaboration".to_string(),
        context_payload: "Source: doc_a.md\nSection: Architecture\n\nArchitectureConstraint rule 1".to_string(),
        metadata: "{}".to_string(),
        vector: vec![0.1; 384],
    };
    let chunk_b1 = IndexChunk {
        chunk_id: "doc_b.md_b0_c0".to_string(),
        parent_id: Some("doc_b.md_b0".to_string()),
        filename: "doc_b.md".to_string(),
        vector_text: "ArchitectureConstraint in another document".to_string(),
        context_payload: "Source: doc_b.md\nSection: Governance\n\nArchitectureConstraint rule 2".to_string(),
        metadata: "{}".to_string(),
        vector: vec![0.2; 384],
    };

    let batch = ArrowBatchBuilder::build(vec![chunk_a1, chunk_a2, chunk_b1], 384).unwrap();
    let table = LanceStorage::write_batch(&db, "adrs", batch).await.unwrap();
    LanceStorage::build_fts_index(&table, "context_payload")
        .await
        .unwrap();

    let engine = create_test_engine(&temp_dir);
    let results = engine
        .search("ArchitectureConstraint", 5, "fts")
        .await
        .expect("search must succeed");

    // Must return exactly 2 distinct parent blocks, NOT 3
    assert_eq!(
        results.len(),
        2,
        "Sibling child chunks for doc_a.md_b0 must collapse into 1 parent result"
    );

    let returned_chunk_ids: Vec<&str> = results
        .iter()
        .filter_map(|r| r.get("chunk_id").and_then(|v| v.as_str()))
        .collect();

    // Verify both doc_a and doc_b are represented
    assert!(returned_chunk_ids.iter().any(|id| id.starts_with("doc_a.md_b0")));
    assert!(returned_chunk_ids.iter().any(|id| id.starts_with("doc_b.md_b0")));
}

