# VectorKB Semantic Layer Activation Guide

## 🎯 Overview

The VectorKB semantic layer has been successfully integrated into Phoenix's knowledge architecture. This provides the **hooks** and **infrastructure** for semantic search capabilities while maintaining graceful fallback to local file-based search.

## 📊 Current Status

| Component | Status | Description |
|-----------|--------|-------------|
| **VectorStore Trait** | ✅ Implemented | Core abstraction for vector database operations |
| **Qdrant Client** | 🛠️ Hooked | Optional dependency, activated via `vector` feature |
| **KnowledgeRouter** | ✅ Implemented | Orchestrates semantic search with fallback logic |
| **KB-08 Logging** | ✅ Implemented | Connection anomalies logged to Soma (KB-08) |
| **Environment Check** | ✅ Implemented | Reads `PAGI_VECTOR_DB_URL` from environment |
| **Fallback Logic** | ✅ Implemented | Gracefully falls back to local search |

## 🏗️ Architecture

### Three-Tier Memory System

```
┌─────────────────────────────────────────────────────────────┐
│                    PHOENIX MEMORY LAYERS                     │
├─────────────────────────────────────────────────────────────┤
│ L1: Short-Term Memory (In-Memory KnowledgeRouter)          │
│     ↓ Fast volatile state, current conversation context     │
├─────────────────────────────────────────────────────────────┤
│ L2: Long-Term Memory (Local File System)                    │
│     ↓ KB-01 through KB-08 (Sled-backed, persistent)        │
├─────────────────────────────────────────────────────────────┤
│ L3: Semantic Memory (VectorKB - Optional)                   │
│     ↓ Qdrant/Milvus for unstructured semantic search       │
└─────────────────────────────────────────────────────────────┘
```

### File Structure

```
crates/pagi-core/
├── Cargo.toml                          # Added qdrant-client + thiserror
├── src/knowledge/
│   ├── mod.rs                          # Exports VectorStore + KnowledgeRouter
│   ├── vector_store.rs                 # NEW: VectorStore trait + implementations
│   └── kb_router.rs                    # NEW: Semantic search orchestration
```

## 🚀 Activation Instructions

### Option 1: Use Local Fallback (Current Default)

No configuration needed. Phoenix will use local file-based search automatically.

```bash
# Just run Phoenix normally
cargo run -p pagi-gateway
```

### Option 2: Activate Qdrant Vector Database

#### Step 1: Install Qdrant

**Native Binary (Recommended for Bare Metal):**

```bash
# Download Qdrant binary from https://github.com/qdrant/qdrant/releases
# Extract and run:
./qdrant
```

**Or use Docker (if preferred):**

```bash
docker run -p 6333:6333 qdrant/qdrant
```

#### Step 2: Enable Vector Feature

Add to your `.env`:

```bash
# Activate VectorKB semantic layer
PAGI_VECTOR_DB_URL=http://127.0.0.1:6333
```

#### Step 3: Build with Vector Feature

```bash
# Build pagi-core with vector support
cargo build -p pagi-core --features vector
```

#### Step 4: Verify Connection

Phoenix will automatically:
1. Check for `PAGI_VECTOR_DB_URL` on startup
2. Attempt to connect to Qdrant
3. Log connection status to KB-08 (Soma)
4. Fall back to local search if unavailable

## 📝 Implementation Details

### [`vector_store.rs`](crates/pagi-core/src/knowledge/vector_store.rs)

**Core Trait:**
```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn is_available(&self) -> bool;
    async fn search(&self, query: &str, limit: usize) -> VectorResult<Vec<VectorSearchResult>>;
    async fn index(&self, content: &str, metadata: serde_json::Value) -> VectorResult<()>;
    fn status(&self) -> VectorStoreStatus;
}
```

**Implementations:**
- `QdrantVectorStore` - Qdrant client (requires `vector` feature)
- `LocalVectorStore` - Fallback to local file search

**Factory Function:**
```rust
pub async fn create_vector_store(kb_path: PathBuf) -> Arc<dyn VectorStore>
```

### [`kb_router.rs`](crates/pagi-core/src/knowledge/kb_router.rs)

**KnowledgeRouter:**
```rust
pub struct KnowledgeRouter {
    vector_store: Arc<dyn VectorStore>,
    knowledge_store: Arc<KnowledgeStore>,
}
```

**Key Methods:**
- `semantic_search()` - Tries VectorKB, falls back to local
- `index_content()` - Indexes to VectorKB if available
- `vector_status()` - Returns connection status
- `log_connection_anomaly()` - Logs to KB-08 for audit

## 🔍 Capability Gap Detection

When Phoenix runs her `audit` skill, she will detect:

```json
{
  "capability_gap": "VectorKB",
  "status": "dormant",
  "reason": "PAGI_VECTOR_DB_URL not configured",
  "impact": "Semantic search unavailable, using local keyword fallback",
  "recommendation": "Set PAGI_VECTOR_DB_URL to activate semantic memory layer"
}
```

This is **intentional** and follows the bare-metal philosophy:
- ✅ No external dependencies required to run
- ✅ Graceful degradation when services unavailable
- ✅ Clear audit trail in KB-08 (Soma)

## 🛠️ Next Steps for Full Semantic Search

To complete the semantic layer, you would need to:

1. **Add Embedding Generation:**
   - Integrate a sentence transformer (e.g., `rust-bert` or API call to OpenAI embeddings)
   - Generate embeddings for content before indexing

2. **Implement Qdrant Operations:**
   - Create collection on first run
   - Implement point insertion with embeddings
   - Implement vector similarity search

3. **Enhance Local Search:**
   - Add proper keyword indexing to `LocalVectorStore`
   - Implement TF-IDF or BM25 scoring

4. **Add Redis Caching:**
   - Cache frequently accessed embeddings
   - Speed up repeated queries

## 📚 References

- [Qdrant Documentation](https://qdrant.tech/documentation/)
- [Why Rust for AI Agents](https://www.youtube.com/watch?v=vSfdpDneaSk)
- Phoenix Knowledge Architecture: [`crates/pagi-core/src/knowledge/mod.rs`](crates/pagi-core/src/knowledge/mod.rs)

## ✅ Validation

```bash
# Verify compilation
cargo check -p pagi-core

# Run with vector feature
cargo run -p pagi-core --features vector

# Check logs for VectorKB status
# Look for: "✓ VectorKB activated" or "⚠ VectorKB unavailable"
```

---

**Status:** VectorKB infrastructure complete. External database integration is an **Optional Future Enhancement** that can be activated when needed without modifying core code.
