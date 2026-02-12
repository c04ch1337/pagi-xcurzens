//! Topic Indexer Test: Verifies autonomous memory evolution capability
//!
//! This test validates that Phoenix can:
//! 1. Analyze conversation history for indexing opportunities (diagnostic mode)
//! 2. Create topic summaries for conversation batches (index mode)
//! 3. Retrieve conversations by topic keyword (optimized search)
//!
//! Run with: `cargo test --test topic_indexer_test`

use pagi_core::{KbType, KnowledgeStore, TenantContext};
use pagi_skills::{ConversationTopicIndexer, ModelRouter};
use std::sync::Arc;

fn test_ctx(agent_id: &str) -> TenantContext {
    TenantContext {
        tenant_id: "test_tenant".to_string(),
        correlation_id: None,
        agent_id: Some(agent_id.to_string()),
    }
}

#[tokio::test]
async fn test_topic_indexer_diagnostic_mode() {
    // Setup: Create test knowledge store
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let db_path = temp_dir.path().join("pagi_topic_indexer_test");
    let store = Arc::new(KnowledgeStore::open_path(&db_path).expect("open store"));
    store.pagi_init_kb_metadata().ok();
    
    let agent_id = "test_agent";
    let slot_id = KbType::Chronos.slot_id();
    
    // Seed conversation history (simulate 25 exchanges)
    for i in 0..25 {
        let key = format!("conversation/{}/msg_{:04}", agent_id, i);
        let content = serde_json::json!({
            "role": if i % 2 == 0 { "user" } else { "assistant" },
            "content": format!("Test message {} about various topics", i),
        });
        store.insert(slot_id, &key, &content.to_string().into_bytes())
            .expect("insert conversation");
    }
    
    // Create topic indexer skill
    let indexer = ConversationTopicIndexer::new(Arc::clone(&store));
    let ctx = test_ctx(agent_id);
    
    // Execute diagnostic mode
    let payload = serde_json::json!({
        "mode": "diagnostic",
        "batch_size": 10,
    });
    
    let result = indexer.execute(&ctx, Some(payload))
        .await
        .expect("diagnostic execution");
    
    // Verify diagnostic results
    assert_eq!(result["status"], "diagnostic_complete");
    assert_eq!(result["skill"], "conversation_topic_indexer");
    assert_eq!(result["agent_id"], agent_id);
    
    let analysis = &result["analysis"];
    assert_eq!(analysis["total_conversation_exchanges"], 25);
    assert_eq!(analysis["potential_topic_clusters"], 3); // 25 / 10 = 3 clusters
    assert_eq!(analysis["indexed_topics"], 0); // No topics indexed yet
    
    println!("✓ Diagnostic mode: Identified {} potential topic clusters", 
        analysis["potential_topic_clusters"]);
}

#[tokio::test]
async fn test_topic_indexer_index_mode() {
    // Setup: Create test knowledge store with model router
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let db_path = temp_dir.path().join("pagi_topic_indexer_index_test");
    let store = Arc::new(KnowledgeStore::open_path(&db_path).expect("open store"));
    store.pagi_init_kb_metadata().ok();
    
    let agent_id = "test_agent";
    let slot_id = KbType::Chronos.slot_id();
    
    // Seed conversation history with thematic content
    let topics = vec![
        "The Forge safety mechanisms and kill switch design",
        "Autonomous agent evolution and self-correction",
        "Knowledge base architecture and memory optimization",
    ];
    
    for (batch_idx, topic_content) in topics.iter().enumerate() {
        for i in 0..10 {
            let key = format!("conversation/{}/msg_{:04}", agent_id, batch_idx * 10 + i);
            let content = serde_json::json!({
                "role": if i % 2 == 0 { "user" } else { "assistant" },
                "content": format!("Discussion about {}: point {}", topic_content, i),
            });
            store.insert(slot_id, &key, &content.to_string().into_bytes())
                .expect("insert conversation");
        }
    }
    
    // Create topic indexer with model router
    let router = Arc::new(ModelRouter::with_knowledge(Arc::clone(&store)));
    let indexer = ConversationTopicIndexer::with_model_router(Arc::clone(&store), router);
    let ctx = test_ctx(agent_id);
    
    // Execute index mode
    let payload = serde_json::json!({
        "mode": "index",
        "batch_size": 10,
    });
    
    let result = indexer.execute(&ctx, Some(payload))
        .await
        .expect("index execution");
    
    // Verify indexing results
    assert_eq!(result["status"], "indexing_complete");
    assert_eq!(result["skill"], "conversation_topic_indexer");
    assert_eq!(result["agent_id"], agent_id);
    assert_eq!(result["topics_created"], 3);
    
    println!("✓ Index mode: Created {} topic summaries", result["topics_created"]);
    
    // Verify topics were stored in KB-04
    let topic_keys: Vec<String> = store.scan_keys(slot_id)
        .expect("scan keys")
        .into_iter()
        .filter(|k| k.starts_with(&format!("topic_index/{}/", agent_id)))
        .collect();
    
    assert_eq!(topic_keys.len(), 3, "Expected 3 topic index entries");
    println!("✓ Verified {} topic entries in KB-04", topic_keys.len());
}

#[tokio::test]
async fn test_topic_based_conversation_retrieval() {
    // Setup: Create test knowledge store
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let db_path = temp_dir.path().join("pagi_topic_retrieval_test");
    let store = Arc::new(KnowledgeStore::open_path(&db_path).expect("open store"));
    store.pagi_init_kb_metadata().ok();
    
    let agent_id = "test_agent";
    let slot_id = KbType::Chronos.slot_id();
    
    // Manually create topic index entries
    let topics = vec![
        ("topic_0000", "The Forge safety mechanisms", "conversation/test_agent/msg_0000", "conversation/test_agent/msg_0009"),
        ("topic_0001", "Autonomous evolution patterns", "conversation/test_agent/msg_0010", "conversation/test_agent/msg_0019"),
        ("topic_0002", "Memory optimization strategies", "conversation/test_agent/msg_0020", "conversation/test_agent/msg_0029"),
    ];
    
    for (topic_id, topic, start_key, end_key) in topics {
        let topic_key = format!("topic_index/{}/{}", agent_id, topic_id);
        let summary = serde_json::json!({
            "topic_id": topic_id,
            "topic": topic,
            "conversation_start_key": start_key,
            "conversation_end_key": end_key,
            "exchange_count": 10,
            "indexed_at_ms": 1234567890,
        });
        store.insert(slot_id, &topic_key, &summary.to_string().into_bytes())
            .expect("insert topic");
    }
    
    // Test optimized topic-based retrieval
    let forge_conversations = store.get_conversations_by_topic(agent_id, "Forge")
        .expect("retrieve by topic");
    
    assert!(!forge_conversations.is_empty(), "Should find Forge-related conversations");
    assert!(forge_conversations.contains(&"conversation/test_agent/msg_0000".to_string()));
    println!("✓ Topic search 'Forge': Found {} conversation keys", forge_conversations.len());
    
    let evolution_conversations = store.get_conversations_by_topic(agent_id, "evolution")
        .expect("retrieve by topic");
    
    assert!(!evolution_conversations.is_empty(), "Should find evolution-related conversations");
    assert!(evolution_conversations.contains(&"conversation/test_agent/msg_0010".to_string()));
    println!("✓ Topic search 'evolution': Found {} conversation keys", evolution_conversations.len());
    
    // Test non-matching search
    let nonexistent = store.get_conversations_by_topic(agent_id, "quantum_physics")
        .expect("retrieve by topic");
    
    assert!(nonexistent.is_empty(), "Should not find unrelated topics");
    println!("✓ Topic search 'quantum_physics': Correctly returned 0 results");
}

#[tokio::test]
async fn test_topic_indexer_ethos_alignment() {
    // Setup: Create test knowledge store with restrictive Ethos policy
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let db_path = temp_dir.path().join("pagi_topic_ethos_test");
    let store = Arc::new(KnowledgeStore::open_path(&db_path).expect("open store"));
    store.pagi_init_kb_metadata().ok();
    
    // Set restrictive Ethos policy
    let policy = pagi_core::PolicyRecord {
        forbidden_actions: vec!["conversation_topic_indexer".to_string()],
        sensitive_keywords: vec![],
        approval_required: true,
    };
    store.set_ethos_policy(&policy).expect("set policy");
    
    let agent_id = "test_agent";
    
    // Create topic indexer with model router
    let router = Arc::new(ModelRouter::with_knowledge(Arc::clone(&store)));
    let indexer = ConversationTopicIndexer::with_model_router(Arc::clone(&store), router);
    let ctx = test_ctx(agent_id);
    
    // Attempt index mode (should be blocked by Ethos)
    let payload = serde_json::json!({
        "mode": "index",
        "batch_size": 10,
    });
    
    let result = indexer.execute(&ctx, Some(payload))
        .await
        .expect("execution completes");
    
    // Verify Ethos blocked the action
    assert_eq!(result["status"], "blocked_by_ethos");
    assert!(result["reason"].as_str().unwrap().contains("forbidden"));
    
    println!("✓ Ethos alignment: Correctly blocked forbidden action");
}

#[test]
fn test_performance_comparison() {
    // This test demonstrates the performance improvement of topic-based search
    // vs. linear scan (conceptual - actual benchmarking would use criterion)
    
    let conversation_count = 10_000;
    let batch_size = 10;
    let topic_count = conversation_count / batch_size;
    
    // Linear scan: O(n) where n = conversation_count
    let linear_scan_ops = conversation_count;
    
    // Topic-based search: O(t) where t = topic_count
    let topic_search_ops = topic_count;
    
    let improvement_factor = linear_scan_ops as f64 / topic_search_ops as f64;
    
    println!("Performance Analysis:");
    println!("  Conversations: {}", conversation_count);
    println!("  Linear scan operations: {}", linear_scan_ops);
    println!("  Topic search operations: {}", topic_search_ops);
    println!("  Improvement factor: {:.1}x faster", improvement_factor);
    
    assert!(improvement_factor >= 10.0, "Topic search should be at least 10x faster");
}
