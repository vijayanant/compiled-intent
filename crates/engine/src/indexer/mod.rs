pub mod arrow_builder;
pub mod manifest;
pub mod markdown_chunker;
pub mod storage;

use futures::stream::{iter, StreamExt};
use lancedb::connect;
use std::path::{Path, PathBuf};

use crate::engine::CompitentEngine;
use crate::error::CompitentError;
pub use arrow_builder::{ArrowBatchBuilder, IndexChunk};
pub use manifest::{DeltaFile, Manifest, MANIFEST_FILENAME};
pub use markdown_chunker::{chunk_markdown, ParentBlock};
pub use storage::LanceStorage;

struct ChunkTask {
    chunk_id: String,
    parent_id: String,
    filename: String,
    stored_content: String,
    child_text: String,
    metadata_json: String,
    new_hash: String,
}

fn discover_markdown_files<P: AsRef<Path>>(adr_paths: &[P]) -> Vec<PathBuf> {
    let mut md_files = Vec::new();
    for adr_path in adr_paths {
        for entry in walkdir::WalkDir::new(adr_path.as_ref())
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                md_files.push(path.to_path_buf());
            }
        }
    }
    md_files
}

fn prepare_chunk_tasks(delta_files: &[DeltaFile]) -> Vec<ChunkTask> {
    let mut tasks = Vec::new();
    for file in delta_files {
        let (metadata_json, body_content) = Manifest::extract_metadata(&file.content);
        let blocks = chunk_markdown(&body_content);

        for (b_idx, block) in blocks.into_iter().enumerate() {
            let parent_id = format!("{}_b{}", file.filename, b_idx);
            let stored_content = format!(
                "Source: {}\nSection: {}\n\n{}",
                file.filename, block.header, block.payload
            );

            for (c_idx, child_text) in block.children.into_iter().enumerate() {
                tasks.push(ChunkTask {
                    chunk_id: format!("{}_c{}", parent_id, c_idx),
                    parent_id: parent_id.clone(),
                    filename: file.filename.clone(),
                    stored_content: stored_content.clone(),
                    child_text,
                    metadata_json: metadata_json.clone(),
                    new_hash: file.hash.clone(),
                });
            }
        }
    }
    tasks
}

async fn generate_index_chunks(
    engine: &CompitentEngine,
    tasks: Vec<ChunkTask>,
) -> Result<(Vec<IndexChunk>, Vec<(String, String)>), CompitentError> {
    tracing::info!(
        "Generating embeddings for {} total chunks concurrently...",
        tasks.len()
    );

    let results = iter(tasks)
        .map(|task| async move {
            let vec = engine.get_embedding(&task.child_text).await;
            (task, vec)
        })
        .buffer_unordered(16)
        .collect::<Vec<_>>()
        .await;

    let mut chunks = Vec::with_capacity(results.len());
    let mut file_hashes = Vec::new();

    for (task, vector_res) in results {
        let vector = vector_res?;
        file_hashes.push((task.filename.clone(), task.new_hash));

        chunks.push(IndexChunk {
            chunk_id: task.chunk_id,
            parent_id: Some(task.parent_id),
            filename: task.filename,
            vector_text: task.child_text,
            context_payload: task.stored_content,
            metadata: task.metadata_json,
            vector,
        });
    }

    Ok((chunks, file_hashes))
}

pub async fn index_adrs<P: AsRef<Path> + std::fmt::Debug>(
    engine: &CompitentEngine,
    adr_paths: &[P],
) -> Result<(), CompitentError> {
    tracing::info!("Scanning directories '{:?}' for ADRs...", adr_paths);

    let manifest_path = PathBuf::from(&engine.config.lancedb_path).join(MANIFEST_FILENAME);
    let mut manifest = Manifest::load(&manifest_path);

    let md_files = discover_markdown_files(adr_paths);
    let orphans = manifest.find_orphans(adr_paths, &md_files);
    let deltas = manifest.compute_deltas(&md_files)?;

    if deltas.is_empty() && orphans.is_empty() {
        tracing::info!("Cache is warm. All files are up to date. No indexing required.");
        return Ok(());
    }

    let db = connect(&engine.config.lancedb_path).execute().await?;

    if !orphans.is_empty() {
        purge_orphaned_files(&db, &mut manifest, &manifest_path, &orphans).await?;
    }

    if deltas.is_empty() {
        return Ok(());
    }

    tracing::info!("Found {} new/updated files to index.", deltas.len());

    let filenames_to_clean: Vec<&str> = deltas
        .iter()
        .filter(|d| manifest.contains_file(&d.filename))
        .map(|d| d.filename.as_str())
        .collect();
    LanceStorage::invalidate_stale_files(&db, crate::DEFAULT_TABLE_NAME, &filenames_to_clean).await;

    let tasks = prepare_chunk_tasks(&deltas);
    let (index_chunks, updated_hashes) = generate_index_chunks(engine, tasks).await?;

    for (filename, hash) in updated_hashes {
        manifest.update(filename, hash);
    }

    let batch = ArrowBatchBuilder::build(index_chunks, engine.config.embedding_dimensions)?;
    let table = LanceStorage::write_batch(&db, crate::DEFAULT_TABLE_NAME, batch).await?;

    manifest.save(&manifest_path)?;
    LanceStorage::build_fts_index(&table, "context_payload").await?;

    Ok(())
}

async fn purge_orphaned_files(
    db: &lancedb::Connection,
    manifest: &mut Manifest,
    manifest_path: &Path,
    orphans: &[String],
) -> Result<(), CompitentError> {
    tracing::info!("Purging {} orphaned files from index...", orphans.len());
    let orphan_refs: Vec<&str> = orphans.iter().map(|s| s.as_str()).collect();
    LanceStorage::invalidate_stale_files(db, crate::DEFAULT_TABLE_NAME, &orphan_refs).await;
    for orphan in orphans {
        manifest.remove(orphan);
    }
    manifest.save(manifest_path)
}
