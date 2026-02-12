//! Knowledge Router with VectorKB integration.
//!
//! This module provides the routing layer that decides whether to use
//! the semantic vector store or fall back to local KB file search.

use super::vector_store::{VectorError, VectorResult, VectorSearchResult, VectorStore};
use super::store::{EventRecord, KnowledgeStore, ABSURDITY_LOG_PREFIX};
use std::sync::Arc;
use tracing::{info, warn};

/// Knowledge Router that orchestrates semantic search with fallback.
pub struct KnowledgeRouter {
    /// The vector store (Qdrant or local fallback).
    vector_store: Arc<dyn VectorStore>,
    
    /// The local knowledge store (KB-01 through KB-08).
    knowledge_store: Arc<KnowledgeStore>,
}

impl KnowledgeRouter {
    /// Create a new knowledge router with the given stores.
    pub fn new(
        vector_store: Arc<dyn VectorStore>,
        knowledge_store: Arc<KnowledgeStore>,
    ) -> Self {
        Self {
            vector_store,
            knowledge_store,
        }
    }
    
    /// Perform a semantic search across all knowledge bases.
    ///
    /// Enriches the query with Sovereign Identity (highest_rank, operational_domain) when
    /// available so semantic results are biased toward the user's domain (e.g. "Coach", "21 Acres").
    /// Attempts vector store first, then falls back to local keyword search.
    pub async fn semantic_search(
        &self,
        query: &str,
        limit: usize,
    ) -> VectorResult<Vec<VectorSearchResult>> {
        let enriched_query = match self.knowledge_store.get_identity() {
            Ok(Some(ref p)) if !p.highest_rank.is_empty() || !p.operational_domain.is_empty() => {
                let rank = p.highest_rank.trim();
                let domain = p.operational_domain.trim();
                if rank.is_empty() && domain.is_empty() {
                    query.to_string()
                } else {
                    format!("{} {} {}", rank, domain, query)
                }
            }
            _ => query.to_string(),
        };
        // Check if vector store is available
        if self.vector_store.is_available().await {
            match self.vector_store.search(&enriched_query, limit).await {
                Ok(results) => {
                    info!("✓ VectorKB search completed: {} results", results.len());
                    return Ok(results);
                }
                Err(VectorError::FallbackRequired) => {
                    warn!("⚠ VectorKB fallback triggered - using local search");
                    self.log_connection_anomaly("VectorKB fallback required").await;
                }
                Err(e) => {
                    warn!("✗ VectorKB search error: {:?} - falling back to local", e);
                    self.log_connection_anomaly(&format!("VectorKB error: {:?}", e)).await;
                }
            }
        } else {
            warn!("⚠ VectorKB unavailable - using local search");
            self.log_connection_anomaly("VectorKB unavailable").await;
        }
        
        // Fallback to local search (use enriched query for consistency)
        self.local_keyword_search(&enriched_query, limit).await
    }
    
    /// Index content into the vector store for semantic search.
    ///
    /// Falls back gracefully if the vector store is unavailable.
    pub async fn index_content(
        &self,
        content: &str,
        metadata: serde_json::Value,
    ) -> VectorResult<()> {
        if self.vector_store.is_available().await {
            match self.vector_store.index(content, metadata).await {
                Ok(()) => {
                    info!("✓ Content indexed to VectorKB");
                    return Ok(());
                }
                Err(VectorError::FallbackRequired) => {
                    warn!("⚠ VectorKB indexing fallback - content stored locally only");
                }
                Err(e) => {
                    warn!("✗ VectorKB indexing error: {:?} - content stored locally only", e);
                    self.log_connection_anomaly(&format!("VectorKB indexing error: {:?}", e)).await;
                }
            }
        }
        
        // Local storage is handled by individual KB slots
        Ok(())
    }
    
    /// Get the status of the vector store.
    pub fn vector_status(&self) -> super::vector_store::VectorStoreStatus {
        self.vector_store.status()
    }
    
    /// Perform local keyword-based search across KB slots.
    ///
    /// This is the fallback when VectorKB is unavailable.
    /// Note: This is a placeholder implementation. Full keyword search would require
    /// exposing a scan API on KnowledgeStore or using the individual KB slot interfaces.
    async fn local_keyword_search(
        &self,
        query: &str,
        _limit: usize,
    ) -> VectorResult<Vec<VectorSearchResult>> {
        info!("Performing local keyword search for: {}", query);
        
        // Placeholder: In a full implementation, this would scan KB-01 through KB-08
        // using the KnowledgeStore API. For now, return empty results to indicate
        // that local search is available but not yet fully implemented.
        
        info!("Local search completed: 0 results (placeholder implementation)");
        Ok(Vec::new())
    }
    
    /// Log a connection anomaly to KB-08 (Soma) for audit trail.
    async fn log_connection_anomaly(&self, message: &str) {
        let event = EventRecord::now("VectorKB", message)
            .with_outcome("Connection Anomaly");
        
        let key = format!(
            "{}vectorkb_{}",
            ABSURDITY_LOG_PREFIX,
            chrono::Utc::now().timestamp_millis()
        );
        
        if let Err(e) = self.knowledge_store.insert(8, &key, &event.to_bytes()) {
            warn!("Failed to log VectorKB anomaly to KB-08: {}", e);
        } else {
            info!("✓ VectorKB anomaly logged to KB-08: {}", message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::vector_store::LocalVectorStore;
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_router_fallback() {
        let temp_dir = tempfile::tempdir().unwrap();
        let kb_path = temp_dir.path().to_path_buf();
        
        let vector_store = Arc::new(LocalVectorStore::new(kb_path.clone()));
        let knowledge_store = Arc::new(
            // KnowledgeStore does not expose `open`; use `open_path` for a specific directory.
            KnowledgeStore::open_path(kb_path.join("test_kb")).unwrap()
        );
        
        let router = KnowledgeRouter::new(vector_store, knowledge_store);
        
        // Test search with empty KB
        let results = router.semantic_search("test query", 10).await.unwrap();
        assert_eq!(results.len(), 0);
        
        // Test status
        let status = router.vector_status();
        assert_eq!(status.backend, "Local File System");
        assert!(status.connected);
    }
}
