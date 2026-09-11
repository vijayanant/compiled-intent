use arrow::array::RecordBatch;
use arrow::record_batch::{RecordBatchIterator, RecordBatchReader};
use lancedb::connection::Connection;
use lancedb::index::scalar::FtsIndexBuilder;
use lancedb::index::Index;
use lancedb::table::Table;
use crate::error::CompitentError;

pub struct LanceStorage;

impl LanceStorage {
    pub async fn invalidate_stale_files(
        db: &Connection,
        table_name: &str,
        filenames: &[&str],
    ) {
        if filenames.is_empty() {
            return;
        }
        let table_exists = db
            .table_names()
            .execute()
            .await
            .unwrap_or_default()
            .contains(&table_name.to_string());

        if table_exists {
            if let Ok(table) = db.open_table(table_name).execute().await {
                Self::invalidate_stale_vectors(&table, filenames).await;
            }
        }
    }

    async fn invalidate_stale_vectors(
        table: &Table,
        filenames: &[&str],
    ) {
        for filename in filenames {
            tracing::debug!("Removing stale vectors for {}", filename);
            let escaped_filename = filename.replace('\'', "''");
            let query = format!("filename = '{}'", escaped_filename);
            if let Err(e) = table.delete(&query).await {
                tracing::warn!("Failed to remove stale vectors for {}: {}", filename, e);
            }
        }
    }

    pub async fn write_batch(
        db: &Connection,
        table_name: &str,
        batch: RecordBatch,
    ) -> Result<Table, CompitentError> {
        let schema = batch.schema();
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        let reader: Box<dyn RecordBatchReader + Send> = Box::new(batches);

        let table_exists = db
            .table_names()
            .execute()
            .await
            .unwrap_or_default()
            .contains(&table_name.to_string());

        let table = if table_exists {
            tracing::info!("Appending deltas to LanceDB...");
            let table = db.open_table(table_name).execute().await?;
            table.add(reader).execute().await?;
            table
        } else {
            tracing::info!("Creating new LanceDB table...");
            db.create_table(table_name, reader).execute().await?
        };

        Ok(table)
    }

    pub async fn build_fts_index(table: &Table, column: &str) -> Result<(), CompitentError> {
        tracing::info!("Building Full-Text Search (Tantivy) Index on '{}'...", column);
        table
            .create_index(&[column], Index::FTS(FtsIndexBuilder::default()))
            .execute()
            .await?;
        tracing::info!("Tantivy FTS Index built successfully!");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::{ArrowBatchBuilder, IndexChunk};

    #[tokio::test]
    async fn test_sql_quote_escaping_in_invalidation() {
        let temp_dir = std::env::temp_dir().join(format!("intent_storage_test_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let db = lancedb::connect(&temp_dir.display().to_string()).execute().await.unwrap();

        let quote_file = "docs/user's_spec.md";
        let chunk = IndexChunk {
            chunk_id: format!("{}_b0_c0", quote_file),
            parent_id: Some(format!("{}_b0", quote_file)),
            filename: quote_file.to_string(),
            vector_text: "content".to_string(),
            context_payload: "payload".to_string(),
            metadata: "{}".to_string(),
            vector: vec![0.1; 4],
        };

        let batch = ArrowBatchBuilder::build(vec![chunk], 4).unwrap();
        let table = LanceStorage::write_batch(&db, "test_quotes", batch).await.unwrap();
        assert_eq!(table.count_rows(None).await.unwrap(), 1);

        // Invalidate file with single quote in filename
        LanceStorage::invalidate_stale_files(&db, "test_quotes", &[quote_file]).await;

        let table_after = db.open_table("test_quotes").execute().await.unwrap();
        assert_eq!(table_after.count_rows(None).await.unwrap(), 0, "Quoted filename must be cleanly deleted");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
