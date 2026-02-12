//! LanceDB semantic layer for the 8 Knowledge Bases.
//!
//! When the `lancedb` feature is enabled, provides high-density vector search over KB records.
//! **Oikos (Task)** and **Soma (State)** slots are persisted in this layer (kb2_oikos, kb8_soma)
//! for semantic query and dashboard; Sled remains the primary key-value store until full migration.
//!
//! **Folder paths for the 8 KBs:** use a single LanceDB database directory, e.g. `./data/pagi_lancedb`,
//! with one table per slot (see [LANCEDB_TABLE_NAMES]).

use crate::knowledge::store::KbRecord;
use std::path::Path;

/// Slot table names for LanceDB (one table per KB 1..8).
/// Use these when creating or opening tables for Pneuma, Oikos, Logos, Chronos, Techne, Ethos, Kardia, Soma.
pub const LANCEDB_TABLE_NAMES: [&str; 8] = [
    "kb1_pneuma",
    "kb2_oikos",
    "kb3_logos",
    "kb4_chronos",
    "kb5_techne",
    "kb6_ethos",
    "kb7_kardia",
    "kb8_soma",
];

/// LanceDB semantic layer: vector search and persistence for Oikos/Soma.
/// With the `lancedb` feature disabled, all methods return an error directing the user to enable the feature.
#[cfg(not(feature = "lancedb"))]
pub struct LanceDbLayer;

#[cfg(not(feature = "lancedb"))]
impl LanceDbLayer {
    /// Connect to a LanceDB database. Enable the `lancedb` feature to use.
    pub async fn connect<P: AsRef<Path>>(_path: P) -> Result<Self, String> {
        Err("LanceDB support is disabled. Build with --features lancedb.".to_string())
    }

    /// Ensure all 8 KB tables exist.
    pub async fn ensure_tables(&self, _vector_dims: Option<u32>) -> Result<(), String> {
        Ok(())
    }

    /// Vector search in the given slot (1..8).
    pub async fn vector_search(
        &self,
        _slot_id: u8,
        _embedding: &[f32],
        _limit: usize,
    ) -> Result<Vec<KbRecord>, String> {
        Err("LanceDB support is disabled. Build with --features lancedb.".to_string())
    }
}

#[cfg(feature = "lancedb")]
pub struct LanceDbLayer {
    _path: std::path::PathBuf,
}

#[cfg(feature = "lancedb")]
impl LanceDbLayer {
    /// Connect to a LanceDB database at the given path (e.g. `./data/pagi_lancedb`).
    pub async fn connect<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let _db = lancedb::connect(path_str.clone())
            .execute()
            .await
            .map_err(|e| e.to_string())?;
        Ok(Self {
            _path: path.as_ref().to_path_buf(),
        })
    }

    /// Ensure all 8 KB tables exist. Tables are created on first write; this is a no-op until migration populates LanceDB.
    pub async fn ensure_tables(&self, _vector_dims: Option<u32>) -> Result<(), String> {
        Ok(())
    }

    /// Vector search in the given slot (1..8). Returns up to `limit` nearest KbRecords.
    /// Requires tables to be created and populated (e.g. via migration from Sled).
    pub async fn vector_search(
        &self,
        slot_id: u8,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<KbRecord>, String> {
        if slot_id == 0 || slot_id > 8 {
            return Err("slot_id must be 1..8".to_string());
        }
        let path_str = self._path.to_string_lossy().to_string();
        let db = lancedb::connect(path_str).execute().await.map_err(|e| e.to_string())?;
        let table_name = LANCEDB_TABLE_NAMES[(slot_id - 1) as usize];
        let table = db.open_table(table_name).await.map_err(|e| e.to_string())?;
        let stream = table
            .query()
            .nearest_to(embedding)
            .map_err(|e| e.to_string())?
            .limit(limit)
            .execute()
            .await
            .map_err(|e| e.to_string())?;
        use futures_util::TryStreamExt;
        let batches: Vec<_> = stream.try_collect().await.map_err(|e: lancedb::Error| e.to_string())?;
        let mut out = Vec::new();
        for batch in batches {
            use arrow_array::Array;
            let id_col = batch.column_by_name("id").ok_or("missing id column")?;
            let content_col = batch.column_by_name("content").ok_or("missing content column")?;
            let ts_col = batch.column_by_name("timestamp");
            for i in 0..batch.num_rows() {
                let id_str = id_col.as_any().downcast_ref::<arrow_array::StringArray>().map(|a| a.value(i).to_string()).unwrap_or_default();
                let content = content_col.as_any().downcast_ref::<arrow_array::StringArray>().map(|a| a.value(i).to_string()).unwrap_or_default();
                let timestamp = ts_col.and_then(|c| c.as_any().downcast_ref::<arrow_array::Int64Array>().map(|a| a.value(i))).unwrap_or(0);
                out.push(KbRecord {
                    id: uuid::Uuid::parse_str(&id_str).unwrap_or(uuid::Uuid::nil()),
                    content,
                    metadata: serde_json::json!({}),
                    embedding: None,
                    timestamp,
                });
            }
        }
        Ok(out)
    }
}
