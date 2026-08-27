pub mod markdown_chunker;

use crate::engine::ChronicleEngine;
use crate::error::ChronicleError;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use sha2::{Digest, Sha256};

use arrow::array::{
    FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, RecordBatchReader,
    StringArray,
};
use arrow::datatypes::{DataType, Field, Schema};
use lancedb::connect;

fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in hash {
        use std::fmt::Write;
        write!(&mut hex, "{:02x}", byte).unwrap();
    }
    hex
}

fn extract_metadata(content: &str) -> (String, String) {
    if content.starts_with("---\n") || content.starts_with("---\r\n") {
        if let Some(end_idx) = content[4..].find("---\n").map(|i| i + 4) {
            let yaml_str = &content[4..end_idx];
            if let Ok(json_val) = serde_yaml::from_str::<serde_json::Value>(yaml_str) {
                let rest = &content[end_idx + 4..];
                return (json_val.to_string(), rest.to_string());
            }
        }
    }
    ("{}".to_string(), content.to_string())
}

pub async fn index_adrs(engine: &ChronicleEngine, adr_paths: &[String]) -> Result<(), ChronicleError> {
    tracing::info!("Scanning directories '{:?}' for ADRs...", adr_paths);

    // 1. Initialize Differential Sync Manifest
    let manifest_path = PathBuf::from(&engine.config.lancedb_path).join(".manifest.json");
    let mut manifest: HashMap<String, String> = if manifest_path.exists() {
        let data = std::fs::read_to_string(&manifest_path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        HashMap::new()
    };

    let mut md_files = Vec::new();
    for adr_path in adr_paths {
        if let Ok(entries) = std::fs::read_dir(adr_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                    md_files.push(path);
                }
            }
        }
    }

    if md_files.is_empty() {
        tracing::warn!("No markdown files found.");
        return Ok(());
    }

    // 2. Compute Deltas
    let mut files_to_process = Vec::new();
    for file_path in md_files {
        let filename = file_path.file_name().unwrap().to_string_lossy().to_string();
        let raw_content = std::fs::read_to_string(&file_path)
            .map_err(|e| ChronicleError::Database(format!("Read error: {}", e)))?;
            
        let current_hash = hash_content(&raw_content);
        
        let needs_update = match manifest.get(&filename) {
            Some(stored_hash) => stored_hash != &current_hash,
            None => true, // New file
        };
        
        if needs_update {
            files_to_process.push((filename, raw_content, current_hash));
        }
    }

    if files_to_process.is_empty() {
        tracing::info!("Cache is warm. All files are up to date. No indexing required.");
        return Ok(());
    }

    tracing::info!("Found {} new/updated files to index.", files_to_process.len());

    let db = connect(&engine.config.lancedb_path)
        .execute()
        .await
        .map_err(|e| ChronicleError::Database(e.to_string()))?;

    // 3. Invalidate Stale Cache in LanceDB
    let table_exists = db.table_names().execute().await.unwrap_or_default().contains(&"adrs".to_string());
    if table_exists {
        if let Ok(table) = db.open_table("adrs").execute().await {
            for (filename, _, _) in &files_to_process {
                if manifest.contains_key(filename) {
                    tracing::debug!("Removing stale vectors for {}", filename);
                    let query = format!("filename = '{}'", filename);
                    let _ = table.delete(&query).await;
                }
            }
        }
    }

    // 4. Generate New Parent-Child Vectors
    let mut chunk_ids = Vec::new();
    let mut parent_ids = Vec::new();
    let mut filenames = Vec::new();
    let mut vector_texts = Vec::new();
    let mut context_payloads = Vec::new();
    let mut metadatas = Vec::new();
    let mut vectors = Vec::new();

    for (filename, raw_content, new_hash) in files_to_process {
        let (metadata_json, content) = extract_metadata(&raw_content);
        let blocks = markdown_chunker::chunk_markdown(&content);

        for (b_idx, block) in blocks.iter().enumerate() {
            let parent_id = format!("{}_b{}", filename, b_idx);
            let stored_content = format!("Source: {}\nSection: {}\n\n{}", filename, block.header, block.payload);

            for (c_idx, child_text) in block.children.iter().enumerate() {
                tracing::debug!("Generating embedding for {} - {} (Child {})", filename, block.header, c_idx);
                let vector = engine.get_embedding(child_text).await?;

                chunk_ids.push(format!("{}_c{}", parent_id, c_idx));
                parent_ids.push(parent_id.clone());
                filenames.push(filename.clone());
                vector_texts.push(child_text.clone());
                context_payloads.push(stored_content.clone());
                metadatas.push(metadata_json.clone());
                vectors.extend(vector);
            }
        }
        
        manifest.insert(filename, new_hash);
    }

    tracing::info!("Building Apache Arrow RecordBatch...");
    let schema = Arc::new(Schema::new(vec![
        Field::new("chunk_id", DataType::Utf8, false),
        Field::new("parent_id", DataType::Utf8, true),
        Field::new("filename", DataType::Utf8, false),
        Field::new("vector_text", DataType::Utf8, false),
        Field::new("context_payload", DataType::Utf8, false),
        Field::new("metadata", DataType::Utf8, false),
        Field::new("vector", DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 5120), false),
    ]));

    let chunk_id_array = Arc::new(StringArray::from(chunk_ids));
    let parent_id_array = Arc::new(StringArray::from_iter(parent_ids.into_iter().map(Some)));
    let filename_array = Arc::new(StringArray::from(filenames));
    let vector_text_array = Arc::new(StringArray::from(vector_texts));
    let context_payload_array = Arc::new(StringArray::from(context_payloads));
    let metadata_array = Arc::new(StringArray::from(metadatas));
    
    let vector_values = Float32Array::from(vectors);
    let item_field = Arc::new(Field::new("item", DataType::Float32, true));
    let vector_array = Arc::new(FixedSizeListArray::try_new(item_field, 5120, Arc::new(vector_values), None)
        .map_err(|e| ChronicleError::Database(format!("Arrow List error: {}", e)))?);

    let batch = RecordBatch::try_new(schema.clone(), vec![
        chunk_id_array as _, parent_id_array as _, filename_array as _, 
        vector_text_array as _, context_payload_array as _, metadata_array as _, vector_array as _
    ]).map_err(|e| ChronicleError::Database(format!("Arrow Batch error: {}", e)))?;

    let batches = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());
    let reader: Box<dyn RecordBatchReader + Send> = Box::new(batches);

    if table_exists {
        tracing::info!("Appending deltas to LanceDB...");
        let table = db.open_table("adrs").execute().await.map_err(|e| ChronicleError::Database(e.to_string()))?;
        table.add(reader).execute().await.map_err(|e| ChronicleError::Database(e.to_string()))?;
    } else {
        tracing::info!("Creating new LanceDB table...");
        db.create_table("adrs", reader).execute().await.map_err(|e| ChronicleError::Database(e.to_string()))?;
    }

    std::fs::create_dir_all(&engine.config.chronicle_dir).unwrap_or_default();
    let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
    std::fs::write(&manifest_path, manifest_json).unwrap();

    tracing::info!("Delta sync successful!");
    Ok(())
}
