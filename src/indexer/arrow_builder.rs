use std::sync::Arc;
use arrow::array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use crate::error::CompitentError;

#[derive(Debug, Clone)]
pub struct IndexChunk {
    pub chunk_id: String,
    pub parent_id: Option<String>,
    pub filename: String,
    pub vector_text: String,
    pub context_payload: String,
    pub metadata: String,
    pub vector: Vec<f32>,
}

pub struct ArrowBatchBuilder;

impl ArrowBatchBuilder {
    pub fn schema(dimensions: i32) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("chunk_id", DataType::Utf8, false),
            Field::new("parent_id", DataType::Utf8, true),
            Field::new("filename", DataType::Utf8, false),
            Field::new("vector_text", DataType::Utf8, false),
            Field::new("context_payload", DataType::Utf8, false),
            Field::new("metadata", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dimensions,
                ),
                false,
            ),
        ]))
    }

    pub fn build(chunks: Vec<IndexChunk>, dimensions: i32) -> Result<RecordBatch, CompitentError> {
        let count = chunks.len();
        let mut chunk_ids = Vec::with_capacity(count);
        let mut parent_ids = Vec::with_capacity(count);
        let mut filenames = Vec::with_capacity(count);
        let mut vector_texts = Vec::with_capacity(count);
        let mut context_payloads = Vec::with_capacity(count);
        let mut metadatas = Vec::with_capacity(count);
        let mut vectors = Vec::with_capacity(count * dimensions as usize);

        for chunk in chunks {
            chunk_ids.push(chunk.chunk_id);
            parent_ids.push(chunk.parent_id);
            filenames.push(chunk.filename);
            vector_texts.push(chunk.vector_text);
            context_payloads.push(chunk.context_payload);
            metadatas.push(chunk.metadata);
            vectors.extend(chunk.vector);
        }

        let schema = Self::schema(dimensions);

        let chunk_id_array = Arc::new(StringArray::from(chunk_ids));
        let parent_id_array = Arc::new(StringArray::from(parent_ids));
        let filename_array = Arc::new(StringArray::from(filenames));
        let vector_text_array = Arc::new(StringArray::from(vector_texts));
        let context_payload_array = Arc::new(StringArray::from(context_payloads));
        let metadata_array = Arc::new(StringArray::from(metadatas));

        let vector_values = Float32Array::from(vectors);
        let item_field = Arc::new(Field::new("item", DataType::Float32, true));
        let vector_array = Arc::new(FixedSizeListArray::try_new(
            item_field,
            dimensions,
            Arc::new(vector_values),
            None,
        )?);

        let batch = RecordBatch::try_new(
            schema,
            vec![
                chunk_id_array as _,
                parent_id_array as _,
                filename_array as _,
                vector_text_array as _,
                context_payload_array as _,
                metadata_array as _,
                vector_array as _,
            ],
        )?;

        Ok(batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::Array;

    #[test]
    fn test_arrow_batch_builder() {
        let chunk = IndexChunk {
            chunk_id: "test.md_b0_c0".to_string(),
            parent_id: Some("test.md_b0".to_string()),
            filename: "test.md".to_string(),
            vector_text: "sample query".to_string(),
            context_payload: "Full context payload".to_string(),
            metadata: "{}".to_string(),
            vector: vec![0.1, 0.2, 0.3, 0.4],
        };

        let batch = ArrowBatchBuilder::build(vec![chunk], 4).expect("Should build RecordBatch");
        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 7);
    }

    #[test]
    fn test_empty_chunks_batch() {
        let batch = ArrowBatchBuilder::build(vec![], 4).expect("Should build empty RecordBatch");
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.num_columns(), 7);
    }

    #[test]
    fn test_batch_with_none_parent_id() {
        let chunk = IndexChunk {
            chunk_id: "root.md_c0".to_string(),
            parent_id: None,
            filename: "root.md".to_string(),
            vector_text: "test".to_string(),
            context_payload: "payload".to_string(),
            metadata: "{}".to_string(),
            vector: vec![0.5, 0.5],
        };

        let batch = ArrowBatchBuilder::build(vec![chunk], 2).expect("Should build batch with null parent");
        assert_eq!(batch.num_rows(), 1);
        let parent_col = batch
            .column_by_name("parent_id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .expect("parent_id column must exist");
        assert!(parent_col.is_null(0));
    }
}
