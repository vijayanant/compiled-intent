use arrow::array::RecordBatch;
use arrow::record_batch::{RecordBatchIterator, RecordBatchReader};
use lancedb::connection::Connection;
use lancedb::index::scalar::FtsIndexBuilder;
use lancedb::index::Index;
use lancedb::table::Table;
use crate::error::CompitentError;

pub struct LanceStorage;

impl LanceStorage {
    pub async fn invalidate_stale_vectors(
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
