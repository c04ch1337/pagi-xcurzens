//! Tool Memory: LanceDB Integration for Tool Output Indexing
//!
//! This module provides semantic memory for tool outputs, enabling the Sovereign
//! Operator to learn from its actions and perform "Reflexion" - the ability to
//! reflect on past tool executions and improve future decisions.
//!
//! ## Architecture
//!
//! Tool outputs are automatically indexed into LanceDB with embeddings, allowing:
//! 1. Semantic search over past tool executions
//! 2. Pattern recognition in tool usage
//! 3. Learning from successful and failed operations
//! 4. Context-aware tool selection
//!
//! ## Storage
//!
//! Tool outputs are stored in a dedicated LanceDB table: `tool_outputs`
//! Each record includes:
//! - Tool name and parameters
//! - Execution result (success/failure)
//! - Output content
//! - Timestamp
//! - Embedding for semantic search

use std::path::Path;
use serde::{Deserialize, Serialize};
use tracing::warn;

use pagi_core::KbRecord;

// ---------------------------------------------------------------------------
// Tool Execution Record
// ---------------------------------------------------------------------------

/// Record of a tool execution for semantic memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRecord {
    /// Unique identifier for this execution
    pub id: uuid::Uuid,
    /// Name of the tool that was executed
    pub tool_name: String,
    /// Parameters passed to the tool
    pub parameters: serde_json::Value,
    /// Whether the execution was successful
    pub success: bool,
    /// Output from the tool (stdout, result, etc.)
    pub output: String,
    /// Error message if execution failed
    pub error: Option<String>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Timestamp of execution
    pub timestamp: i64,
    /// Embedding for semantic search (optional, populated by LanceDB)
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
}

impl ToolExecutionRecord {
    /// Create a new tool execution record
    pub fn new(
        tool_name: String,
        parameters: serde_json::Value,
        success: bool,
        output: String,
        error: Option<String>,
        duration_ms: u64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            tool_name,
            parameters,
            success,
            output,
            error,
            duration_ms,
            timestamp: chrono::Utc::now().timestamp_millis(),
            embedding: None,
        }
    }

    /// Convert to KbRecord for storage in LanceDB
    pub fn to_kb_record(&self) -> KbRecord {
        let content = format!(
            "Tool: {}\nSuccess: {}\nOutput: {}\nError: {:?}\nDuration: {}ms",
            self.tool_name, self.success, self.output, self.error, self.duration_ms
        );

        KbRecord {
            id: self.id,
            content,
            metadata: serde_json::json!({
                "tool_name": self.tool_name,
                "success": self.success,
                "duration_ms": self.duration_ms,
                "timestamp": self.timestamp,
                "parameters": self.parameters,
            }),
            embedding: self.embedding.clone(),
            timestamp: self.timestamp,
        }
    }
}

// ---------------------------------------------------------------------------
// Tool Memory Manager
// ---------------------------------------------------------------------------

/// Manager for tool execution memory using LanceDB
#[cfg(feature = "lancedb")]
pub struct ToolMemoryManager {
    /// Path to the LanceDB database
    db_path: std::path::PathBuf,
    /// Table name for tool outputs
    table_name: String,
}

#[cfg(feature = "lancedb")]
impl ToolMemoryManager {
    /// Create a new ToolMemoryManager
    pub fn new<P: AsRef<Path>>(db_path: P) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
            table_name: "tool_outputs".to_string(),
        }
    }

    /// Initialize the LanceDB table for tool outputs
    pub async fn initialize(&self) -> Result<(), String> {
        let path_str = self.db_path.to_string_lossy().to_string();
        let db = lancedb::connect(path_str)
            .execute()
            .await
            .map_err(|e| format!("Failed to connect to LanceDB: {}", e))?;

        // Check if table exists, create if not
        let tables = db
            .table_names()
            .execute()
            .await
            .map_err(|e| format!("Failed to list tables: {}", e))?;

        if !tables.contains(&self.table_name) {
            info!("Creating tool_outputs table in LanceDB");

            // Create schema for tool outputs
            let schema = arrow::datatypes::Schema::new(vec![
                arrow::datatypes::Field::new("id", arrow::datatypes::DataType::Utf8, false),
                arrow::datatypes::Field::new("tool_name", arrow::datatypes::DataType::Utf8, false),
                arrow::datatypes::Field::new("parameters", arrow::datatypes::DataType::Utf8, false),
                arrow::datatypes::Field::new("success", arrow::datatypes::DataType::Boolean, false),
                arrow::datatypes::Field::new("output", arrow::datatypes::DataType::Utf8, false),
                arrow::datatypes::Field::new("error", arrow::datatypes::DataType::Utf8, true),
                arrow::datatypes::Field::new("duration_ms", arrow::datatypes::DataType::UInt64, false),
                arrow::datatypes::Field::new("timestamp", arrow::datatypes::DataType::Int64, false),
                arrow::datatypes::Field::new(
                    "vector",
                    arrow::datatypes::DataType::FixedSizeList(
                        Arc::new(arrow::datatypes::Field::new(
                            "item",
                            arrow::datatypes::DataType::Float32,
                            true,
                        )),
                        1536, // OpenAI embedding dimension
                    ),
                    true,
                ),
            ]);

            db.create_table(&self.table_name, schema)
                .execute()
                .await
                .map_err(|e| format!("Failed to create table: {}", e))?;
        }

        Ok(())
    }

    /// Store a tool execution record
    pub async fn store_execution(&self, record: &ToolExecutionRecord) -> Result<(), String> {
        let path_str = self.db_path.to_string_lossy().to_string();
        let db = lancedb::connect(path_str)
            .execute()
            .await
            .map_err(|e| format!("Failed to connect to LanceDB: {}", e))?;

        let table = db
            .open_table(&self.table_name)
            .await
            .map_err(|e| format!("Failed to open table: {}", e))?;

        // Convert record to Arrow batch
        let id_array = arrow::array::StringArray::from(vec![record.id.to_string()]);
        let tool_name_array = arrow::array::StringArray::from(vec![record.tool_name.clone()]);
        let parameters_array =
            arrow::array::StringArray::from(vec![serde_json::to_string(&record.parameters).unwrap()]);
        let success_array = arrow::array::BooleanArray::from(vec![record.success]);
        let output_array = arrow::array::StringArray::from(vec![record.output.clone()]);
        let error_array = arrow::array::StringArray::from(vec![record.error.clone().unwrap_or_default()]);
        let duration_ms_array = arrow::array::UInt64Array::from(vec![record.duration_ms]);
        let timestamp_array = arrow::array::Int64Array::from(vec![record.timestamp]);

        // Create vector array (placeholder - would need embedding service)
        let vector_data: Vec<f32> = vec![0.0; 1536]; // Placeholder embedding
        let vector_array = arrow::array::FixedSizeListArray::from_iter_primitive::<
            arrow::datatypes::Float32Type,
            _,
            _,
        >(Some(vec![vector_data.into_iter()]), 1536);

        let batch = arrow::record_batch::RecordBatch::try_new(
            Arc::new(arrow::datatypes::Schema::new(vec![
                arrow::datatypes::Field::new("id", arrow::datatypes::DataType::Utf8, false),
                arrow::datatypes::Field::new("tool_name", arrow::datatypes::DataType::Utf8, false),
                arrow::datatypes::Field::new("parameters", arrow::datatypes::DataType::Utf8, false),
                arrow::datatypes::Field::new("success", arrow::datatypes::DataType::Boolean, false),
                arrow::datatypes::Field::new("output", arrow::datatypes::DataType::Utf8, false),
                arrow::datatypes::Field::new("error", arrow::datatypes::DataType::Utf8, true),
                arrow::datatypes::Field::new("duration_ms", arrow::datatypes::DataType::UInt64, false),
                arrow::datatypes::Field::new("timestamp", arrow::datatypes::DataType::Int64, false),
                arrow::datatypes::Field::new(
                    "vector",
                    arrow::datatypes::DataType::FixedSizeList(
                        Arc::new(arrow::datatypes::Field::new(
                            "item",
                            arrow::datatypes::DataType::Float32,
                            true,
                        )),
                        1536,
                    ),
                    true,
                ),
            ])),
            vec![
                Arc::new(id_array),
                Arc::new(tool_name_array),
                Arc::new(parameters_array),
                Arc::new(success_array),
                Arc::new(output_array),
                Arc::new(error_array),
                Arc::new(duration_ms_array),
                Arc::new(timestamp_array),
                Arc::new(vector_array),
            ],
        )
        .map_err(|e| format!("Failed to create record batch: {}", e))?;

        table
            .add(Box::new(batch))
            .execute()
            .await
            .map_err(|e| format!("Failed to add record: {}", e))?;

        debug!("Stored tool execution record: {}", record.tool_name);
        Ok(())
    }

    /// Search for similar tool executions
    pub async fn search_similar(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<ToolExecutionRecord>, String> {
        let path_str = self.db_path.to_string_lossy().to_string();
        let db = lancedb::connect(path_str)
            .execute()
            .await
            .map_err(|e| format!("Failed to connect to LanceDB: {}", e))?;

        let table = db
            .open_table(&self.table_name)
            .await
            .map_err(|e| format!("Failed to open table: {}", e))?;

        let stream = table
            .query()
            .nearest_to(embedding)
            .map_err(|e| format!("Failed to create query: {}", e))?
            .limit(limit)
            .execute()
            .await
            .map_err(|e| format!("Failed to execute query: {}", e))?;

        use futures_util::TryStreamExt;
        let batches: Vec<_> = stream
            .try_collect()
            .await
            .map_err(|e: lancedb::Error| format!("Failed to collect results: {}", e))?;

        let mut records = Vec::new();
        for batch in batches {
            use arrow_array::Array;

            let id_col = batch
                .column_by_name("id")
                .ok_or("missing id column")?;
            let tool_name_col = batch
                .column_by_name("tool_name")
                .ok_or("missing tool_name column")?;
            let parameters_col = batch
                .column_by_name("parameters")
                .ok_or("missing parameters column")?;
            let success_col = batch
                .column_by_name("success")
                .ok_or("missing success column")?;
            let output_col = batch
                .column_by_name("output")
                .ok_or("missing output column")?;
            let error_col = batch.column_by_name("error");
            let duration_ms_col = batch
                .column_by_name("duration_ms")
                .ok_or("missing duration_ms column")?;
            let timestamp_col = batch
                .column_by_name("timestamp")
                .ok_or("missing timestamp column")?;

            for i in 0..batch.num_rows() {
                let id_str = id_col
                    .as_any()
                    .downcast_ref::<arrow_array::StringArray>()
                    .map(|a| a.value(i).to_string())
                    .unwrap_or_default();
                let tool_name = tool_name_col
                    .as_any()
                    .downcast_ref::<arrow_array::StringArray>()
                    .map(|a| a.value(i).to_string())
                    .unwrap_or_default();
                let parameters_str = parameters_col
                    .as_any()
                    .downcast_ref::<arrow_array::StringArray>()
                    .map(|a| a.value(i).to_string())
                    .unwrap_or_default();
                let success = success_col
                    .as_any()
                    .downcast_ref::<arrow_array::BooleanArray>()
                    .map(|a| a.value(i))
                    .unwrap_or(false);
                let output = output_col
                    .as_any()
                    .downcast_ref::<arrow_array::StringArray>()
                    .map(|a| a.value(i).to_string())
                    .unwrap_or_default();
                let error = error_col.and_then(|c| {
                    c.as_any()
                        .downcast_ref::<arrow_array::StringArray>()
                        .map(|a| {
                            let val = a.value(i);
                            if val.is_empty() {
                                None
                            } else {
                                Some(val.to_string())
                            }
                        })
                });
                let duration_ms = duration_ms_col
                    .as_any()
                    .downcast_ref::<arrow_array::UInt64Array>()
                    .map(|a| a.value(i))
                    .unwrap_or(0);
                let timestamp = timestamp_col
                    .as_any()
                    .downcast_ref::<arrow_array::Int64Array>()
                    .map(|a| a.value(i))
                    .unwrap_or(0);

                let parameters: serde_json::Value =
                    serde_json::from_str(&parameters_str).unwrap_or(serde_json::json!({}));

                records.push(ToolExecutionRecord {
                    id: uuid::Uuid::parse_str(&id_str).unwrap_or(uuid::Uuid::nil()),
                    tool_name,
                    parameters,
                    success,
                    output,
                    error,
                    duration_ms,
                    timestamp,
                    embedding: None,
                });
            }
        }

        Ok(records)
    }

    /// Get recent executions for a specific tool
    pub async fn get_recent_executions(
        &self,
        tool_name: &str,
        limit: usize,
    ) -> Result<Vec<ToolExecutionRecord>, String> {
        let path_str = self.db_path.to_string_lossy().to_string();
        let db = lancedb::connect(path_str)
            .execute()
            .await
            .map_err(|e| format!("Failed to connect to LanceDB: {}", e))?;

        let table = db
            .open_table(&self.table_name)
            .await
            .map_err(|e| format!("Failed to open table: {}", e))?;

        let stream = table
            .query()
            .filter(format!("tool_name = '{}'", tool_name))
            .limit(limit)
            .execute()
            .await
            .map_err(|e| format!("Failed to execute query: {}", e))?;

        use futures_util::TryStreamExt;
        let batches: Vec<_> = stream
            .try_collect()
            .await
            .map_err(|e: lancedb::Error| format!("Failed to collect results: {}", e))?;

        let mut records = Vec::new();
        for batch in batches {
            use arrow_array::Array;

            let id_col = batch
                .column_by_name("id")
                .ok_or("missing id column")?;
            let tool_name_col = batch
                .column_by_name("tool_name")
                .ok_or("missing tool_name column")?;
            let parameters_col = batch
                .column_by_name("parameters")
                .ok_or("missing parameters column")?;
            let success_col = batch
                .column_by_name("success")
                .ok_or("missing success column")?;
            let output_col = batch
                .column_by_name("output")
                .ok_or("missing output column")?;
            let error_col = batch.column_by_name("error");
            let duration_ms_col = batch
                .column_by_name("duration_ms")
                .ok_or("missing duration_ms column")?;
            let timestamp_col = batch
                .column_by_name("timestamp")
                .ok_or("missing timestamp column")?;

            for i in 0..batch.num_rows() {
                let id_str = id_col
                    .as_any()
                    .downcast_ref::<arrow_array::StringArray>()
                    .map(|a| a.value(i).to_string())
                    .unwrap_or_default();
                let tool_name = tool_name_col
                    .as_any()
                    .downcast_ref::<arrow_array::StringArray>()
                    .map(|a| a.value(i).to_string())
                    .unwrap_or_default();
                let parameters_str = parameters_col
                    .as_any()
                    .downcast_ref::<arrow_array::StringArray>()
                    .map(|a| a.value(i).to_string())
                    .unwrap_or_default();
                let success = success_col
                    .as_any()
                    .downcast_ref::<arrow_array::BooleanArray>()
                    .map(|a| a.value(i))
                    .unwrap_or(false);
                let output = output_col
                    .as_any()
                    .downcast_ref::<arrow_array::StringArray>()
                    .map(|a| a.value(i).to_string())
                    .unwrap_or_default();
                let error = error_col.and_then(|c| {
                    c.as_any()
                        .downcast_ref::<arrow_array::StringArray>()
                        .map(|a| {
                            let val = a.value(i);
                            if val.is_empty() {
                                None
                            } else {
                                Some(val.to_string())
                            }
                        })
                });
                let duration_ms = duration_ms_col
                    .as_any()
                    .downcast_ref::<arrow_array::UInt64Array>()
                    .map(|a| a.value(i))
                    .unwrap_or(0);
                let timestamp = timestamp_col
                    .as_any()
                    .downcast_ref::<arrow_array::Int64Array>()
                    .map(|a| a.value(i))
                    .unwrap_or(0);

                let parameters: serde_json::Value =
                    serde_json::from_str(&parameters_str).unwrap_or(serde_json::json!({}));

                records.push(ToolExecutionRecord {
                    id: uuid::Uuid::parse_str(&id_str).unwrap_or(uuid::Uuid::nil()),
                    tool_name,
                    parameters,
                    success,
                    output,
                    error,
                    duration_ms,
                    timestamp,
                    embedding: None,
                });
            }
        }

        Ok(records)
    }
}

/// Stub implementation when LanceDB feature is not enabled
#[cfg(not(feature = "lancedb"))]
pub struct ToolMemoryManager {
    _db_path: std::path::PathBuf,
}

#[cfg(not(feature = "lancedb"))]
impl ToolMemoryManager {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Self {
        Self {
            _db_path: db_path.as_ref().to_path_buf(),
        }
    }

    pub async fn initialize(&self) -> Result<(), String> {
        warn!("LanceDB support is disabled. Tool memory will not be persisted.");
        Ok(())
    }

    pub async fn store_execution(&self, _record: &ToolExecutionRecord) -> Result<(), String> {
        warn!("LanceDB support is disabled. Tool execution not stored.");
        Ok(())
    }

    pub async fn search_similar(
        &self,
        _embedding: &[f32],
        _limit: usize,
    ) -> Result<Vec<ToolExecutionRecord>, String> {
        warn!("LanceDB support is disabled. Cannot search tool memory.");
        Ok(Vec::new())
    }

    pub async fn get_recent_executions(
        &self,
        _tool_name: &str,
        _limit: usize,
    ) -> Result<Vec<ToolExecutionRecord>, String> {
        warn!("LanceDB support is disabled. Cannot retrieve tool executions.");
        Ok(Vec::new())
    }
}

