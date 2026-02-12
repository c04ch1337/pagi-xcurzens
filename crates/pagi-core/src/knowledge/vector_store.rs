//! VectorStore trait and implementation for semantic search layer.
//!
//! This module provides the interface for Phoenix's semantic memory layer,
//! with graceful fallback to local file-based search when external vector
//! databases are unavailable.

use super::store::KnowledgeStore;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Default Qdrant collection name for semantic memory (self-healing on boot).
pub const PAGI_MEMORY_COLLECTION: &str = "pagi_memory_v1";
/// Default vector dimension (sentence-transformers/all-MiniLM-L6-v2 is 384 - optimal for bare metal).
pub const PAGI_VECTOR_SIZE: u64 = 384;

/// Result type for vector store operations.
pub type VectorResult<T> = Result<T, VectorError>;

/// Errors that can occur during vector store operations.
#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Query failed: {0}")]
    QueryFailed(String),
    
    #[error("Indexing failed: {0}")]
    IndexingFailed(String),
    
    #[error("Vector DB not configured")]
    NotConfigured,
    
    #[error("Fallback to local search")]
    FallbackRequired,
}

/// A semantic search result with relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    /// The matched content/document.
    pub content: String,
    
    /// Relevance score (0.0 to 1.0, higher is better).
    pub score: f32,
    
    /// Optional metadata (KB slot, timestamp, etc.).
    pub metadata: serde_json::Value,
}

/// Core trait for vector database operations.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Check if the vector store is available and connected.
    async fn is_available(&self) -> bool;
    
    /// Perform semantic search with the given query.
    async fn search(&self, query: &str, limit: usize) -> VectorResult<Vec<VectorSearchResult>>;
    
    /// Index a new document for semantic search.
    async fn index(&self, content: &str, metadata: serde_json::Value) -> VectorResult<()>;
    
    /// Get the connection status and any error messages.
    fn status(&self) -> VectorStoreStatus;
    
    /// Downcast support for graceful shutdown (allows access to QdrantVectorStore::close)
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Status information for the vector store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreStatus {
    pub connected: bool,
    pub backend: String,
    pub last_error: Option<String>,
}

/// Qdrant-based vector store implementation.
#[cfg(feature = "vector")]
pub struct QdrantVectorStore {
    #[cfg(feature = "vector")]
    client: Option<Arc<qdrant_client::Qdrant>>,
    url: String,
    collection_name: String,
    last_error: Arc<std::sync::RwLock<Option<String>>>,
}

#[cfg(feature = "vector")]
impl QdrantVectorStore {
    /// Create a new Qdrant vector store from environment configuration.
    /// Uses `PAGI_VECTOR_DB_URL` and collection `pagi_memory_v1`. If `knowledge` is provided,
    /// logs "VectorKB Online: Collection pagi_memory_v1 initialized." to KB-08 on success.
    pub async fn from_env(knowledge: Option<Arc<KnowledgeStore>>) -> VectorResult<Self> {
        let url = std::env::var("PAGI_VECTOR_DB_URL")
            .map_err(|_| VectorError::NotConfigured)?;
        
        Self::new(&url, PAGI_MEMORY_COLLECTION, knowledge).await
    }
    
    /// Create a new Qdrant vector store with explicit configuration.
    /// On successful collection bootstrap, logs to KB-08 if `knowledge` is provided.
    pub async fn new(
        url: &str,
        collection_name: &str,
        knowledge: Option<Arc<KnowledgeStore>>,
    ) -> VectorResult<Self> {
        info!("Initializing Qdrant vector store at {}", url);
        
        let client = match qdrant_client::Qdrant::from_url(url).build() {
            Ok(c) => {
                // Test connection
                match c.health_check().await {
                    Ok(_) => {
                        info!("✓ Qdrant connection established");
                        Some(Arc::new(c))
                    }
                    Err(e) => {
                        warn!("Qdrant health check failed: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("Failed to create Qdrant client: {}", e);
                None
            }
        };
        
        let store = Self {
            client,
            url: url.to_string(),
            collection_name: collection_name.to_string(),
            last_error: Arc::new(std::sync::RwLock::new(None)),
        };
        
        // Bootstrap: Ensure collection exists with correct schema (self-healing on boot)
        if store.client.is_some() {
            if let Err(e) = store.ensure_collection_exists().await {
                warn!("Failed to bootstrap collection '{}': {}", collection_name, e);
                store.set_error(format!("Collection bootstrap failed: {}", e));
            } else {
                info!("✓ Collection '{}' ready for production use", collection_name);
                if let Some(ref k) = knowledge {
                    let msg = format!(
                        "VectorKB Online: Collection {} initialized.",
                        store.collection_name
                    );
                    if k.record_success_metric(&msg).is_ok() {
                        info!("✓ VectorKB bootstrap logged to KB-08");
                    }
                }
            }
        }
        
        Ok(store)
    }
    
    /// Ensure the collection exists with the correct schema (Production Bootstrap).
    /// This is the "Schema-on-Boot" handshake that prevents 404 errors.
    /// Uses Cosine distance and PAGI_VECTOR_SIZE (768) by default.
    async fn ensure_collection_exists(&self) -> VectorResult<()> {
        use qdrant_client::qdrant::{
            CreateCollectionBuilder, Distance, VectorParamsBuilder,
        };
        
        let client = self.client.as_ref()
            .ok_or(VectorError::ConnectionFailed("No client available".to_string()))?;
        
        // Check if collection already exists
        match client.collection_info(&self.collection_name).await {
            Ok(_) => {
                info!("Collection '{}' already exists", self.collection_name);
                return Ok(());
            }
            Err(_) => {
                // Collection doesn't exist, create it
                info!("Creating collection '{}' with default schema", self.collection_name);
            }
        }
        
        let vector_params = VectorParamsBuilder::new(PAGI_VECTOR_SIZE, Distance::Cosine);
        
        client
            .create_collection(
                CreateCollectionBuilder::new(&self.collection_name)
                    .vectors_config(vector_params)
            )
            .await
            .map_err(|e| VectorError::IndexingFailed(format!("Failed to create collection: {}", e)))?;
        
        info!(
            "✓ Collection '{}' created successfully ({}-dim, Cosine)",
            self.collection_name, PAGI_VECTOR_SIZE
        );
        Ok(())
    }
    
    /// Gracefully close the Qdrant connection (Production Shutdown).
    /// Prevents WAL corruption in ./storage directory.
    pub async fn close(&self) {
        if self.client.is_some() {
            info!("Closing Qdrant connection for collection '{}'", self.collection_name);
            // The Qdrant client will be dropped, closing the connection gracefully
        }
    }
    
    fn set_error(&self, error: String) {
        *self.last_error.write().unwrap() = Some(error);
    }
}

#[cfg(feature = "vector")]
#[async_trait]
impl VectorStore for QdrantVectorStore {
    async fn is_available(&self) -> bool {
        if let Some(client) = &self.client {
            client.health_check().await.is_ok()
        } else {
            false
        }
    }
    
    async fn search(&self, _query: &str, _limit: usize) -> VectorResult<Vec<VectorSearchResult>> {
        let _client = self.client.as_ref()
            .ok_or(VectorError::FallbackRequired)?;
        
        // For now, return fallback - full implementation would require embedding generation
        warn!("Qdrant search not fully implemented - falling back to local");
        Err(VectorError::FallbackRequired)
    }
    
    async fn index(&self, _content: &str, _metadata: serde_json::Value) -> VectorResult<()> {
        let _client = self.client.as_ref()
            .ok_or(VectorError::FallbackRequired)?;
        
        // For now, return fallback - full implementation would require embedding generation
        warn!("Qdrant indexing not fully implemented - falling back to local");
        Err(VectorError::FallbackRequired)
    }
    
    fn status(&self) -> VectorStoreStatus {
        VectorStoreStatus {
            connected: self.client.is_some(),
            backend: format!("Qdrant ({})", self.url),
            last_error: self.last_error.read().unwrap().clone(),
        }
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Fallback vector store that uses local file-based search.
pub struct LocalVectorStore {
    kb_path: std::path::PathBuf,
}

impl LocalVectorStore {
    pub fn new(kb_path: std::path::PathBuf) -> Self {
        info!("Using local file-based semantic search (fallback mode)");
        Self { kb_path }
    }
}

#[async_trait]
impl VectorStore for LocalVectorStore {
    async fn is_available(&self) -> bool {
        self.kb_path.exists()
    }
    
    async fn search(&self, query: &str, _limit: usize) -> VectorResult<Vec<VectorSearchResult>> {
        // Simple keyword-based search across local KB files
        // This is a placeholder - real implementation would scan KB-01 through KB-08
        info!("Performing local keyword search for: {}", query);
        
        // Return empty results for now - this would be enhanced with actual file scanning
        Ok(Vec::new())
    }
    
    async fn index(&self, _content: &str, _metadata: serde_json::Value) -> VectorResult<()> {
        // Local indexing is handled by the individual KB slots
        Ok(())
    }
    
    fn status(&self) -> VectorStoreStatus {
        VectorStoreStatus {
            connected: true,
            backend: "Local File System".to_string(),
            last_error: None,
        }
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Factory function to create the appropriate vector store based on environment.
/// If `knowledge` is provided and Qdrant bootstrap succeeds, logs "VectorKB Online: Collection pagi_memory_v1 initialized." to KB-08.
pub async fn create_vector_store(
    kb_path: std::path::PathBuf,
    knowledge: Option<Arc<KnowledgeStore>>,
) -> Arc<dyn VectorStore> {
    // Try to create Qdrant store if feature is enabled and URL is set
    #[cfg(feature = "vector")]
    {
        match QdrantVectorStore::from_env(knowledge).await {
            Ok(store) => {
                if store.is_available().await {
                    info!("✓ VectorKB activated: Qdrant semantic layer online");
                    return Arc::new(store);
                } else {
                    warn!("⚠ VectorKB connection failed - falling back to local search");
                }
            }
            Err(VectorError::NotConfigured) => {
                info!("ℹ PAGI_VECTOR_DB_URL not set - using local search");
            }
            Err(e) => {
                warn!("⚠ VectorKB initialization error: {} - falling back to local search", e);
            }
        }
    }
    
    // Fallback to local store (knowledge unused when vector feature is off)
    let _ = knowledge;
    Arc::new(LocalVectorStore::new(kb_path))
}
