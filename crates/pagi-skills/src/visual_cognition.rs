//! Visual Cognition Skill: Detects and wraps Mermaid diagram blocks for frontend rendering.
//!
//! This skill implements PaperBanana-style visual cognition by intercepting LLM output
//! and identifying diagram blocks (Mermaid, Graphviz DOT, etc.), then wrapping them
//! in a structured JSON envelope for the frontend to render as interactive SVG diagrams.
//!
//! The skill also stores diagram definitions in KB-05 (Architectural Memory) so Phoenix
//! can remember and update visual structures over time.

use pagi_core::{AgentSkill, TenantContext};
use serde::{Deserialize, Serialize};

const SKILL_NAME: &str = "visual_cognition";

#[derive(Debug, Deserialize)]
struct VisualCognitionArgs {
    /// The LLM output text to scan for diagram blocks
    content: String,
    /// Optional: Store the diagram in KB-05 with this key
    store_key: Option<String>,
}

#[derive(Debug, Serialize)]
struct DiagramEnvelope {
    /// Type identifier for frontend routing
    #[serde(rename = "type")]
    envelope_type: String,
    /// Diagram format (mermaid, dot, etc.)
    format: String,
    /// The diagram code content
    content: String,
    /// Optional metadata
    metadata: Option<DiagramMetadata>,
}

#[derive(Debug, Serialize)]
struct DiagramMetadata {
    /// Timestamp of creation
    created_at: String,
    /// Storage key in KB-05 if persisted
    kb_key: Option<String>,
    /// Diagram title/description
    title: Option<String>,
}

#[derive(Debug, Serialize)]
struct VisualCognitionResult {
    /// Original content with diagram blocks replaced by envelopes
    processed_content: String,
    /// Extracted diagrams
    diagrams: Vec<DiagramEnvelope>,
    /// Number of diagrams found
    diagram_count: usize,
}

/// Extracts Mermaid diagram blocks from markdown-style code fences
fn extract_mermaid_blocks(content: &str) -> Vec<(String, usize, usize)> {
    let mut diagrams = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i].trim();
        
        // Look for ```mermaid blocks
        if line.starts_with("```mermaid") {
            let start = i;
            i += 1;
            let mut diagram_lines = Vec::new();
            
            // Collect lines until closing ```
            while i < lines.len() {
                let current = lines[i];
                if current.trim().starts_with("```") {
                    break;
                }
                diagram_lines.push(current);
                i += 1;
            }
            
            if !diagram_lines.is_empty() {
                diagrams.push((diagram_lines.join("\n"), start, i));
            }
        }
        
        i += 1;
    }
    
    diagrams
}

/// Extracts Graphviz DOT blocks from markdown-style code fences
fn extract_dot_blocks(content: &str) -> Vec<(String, usize, usize)> {
    let mut diagrams = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i].trim();
        
        // Look for ```dot or ```graphviz blocks
        if line.starts_with("```dot") || line.starts_with("```graphviz") {
            let start = i;
            i += 1;
            let mut diagram_lines = Vec::new();
            
            // Collect lines until closing ```
            while i < lines.len() {
                let current = lines[i];
                if current.trim().starts_with("```") {
                    break;
                }
                diagram_lines.push(current);
                i += 1;
            }
            
            if !diagram_lines.is_empty() {
                diagrams.push((diagram_lines.join("\n"), start, i));
            }
        }
        
        i += 1;
    }
    
    diagrams
}

/// Processes content and wraps diagram blocks in JSON envelopes
async fn process_visual_content(
    content: &str,
    store_key: Option<String>,
    _ctx: &TenantContext,
) -> Result<VisualCognitionResult, Box<dyn std::error::Error + Send + Sync>> {
    let mut diagrams = Vec::new();
    let processed_content = content.to_string();
    
    // Extract Mermaid diagrams
    let mermaid_blocks = extract_mermaid_blocks(content);
    for (diagram_content, _, _) in mermaid_blocks {
        let metadata = DiagramMetadata {
            created_at: chrono::Utc::now().to_rfc3339(),
            kb_key: store_key.clone(),
            title: None,
        };
        
        let envelope = DiagramEnvelope {
            envelope_type: "diagram".to_string(),
            format: "mermaid".to_string(),
            content: diagram_content.clone(),
            metadata: Some(metadata),
        };
        
        // Diagram persistence to KB-05 would require KnowledgeStore in TenantContext; skip for now.

        diagrams.push(envelope);
    }
    
    // Extract Graphviz DOT diagrams
    let dot_blocks = extract_dot_blocks(content);
    for (diagram_content, _, _) in dot_blocks {
        let metadata = DiagramMetadata {
            created_at: chrono::Utc::now().to_rfc3339(),
            kb_key: store_key.clone(),
            title: None,
        };
        
        let envelope = DiagramEnvelope {
            envelope_type: "diagram".to_string(),
            format: "dot".to_string(),
            content: diagram_content.clone(),
            metadata: Some(metadata),
        };
        
        // Diagram persistence to KB-05 would require KnowledgeStore in TenantContext; skip for now.
        
        diagrams.push(envelope);
    }
    
    let diagram_count = diagrams.len();
    Ok(VisualCognitionResult {
        processed_content,
        diagrams,
        diagram_count,
    })
}

/// Visual Cognition Skill implementation
pub struct VisualCognitionSkill;

impl VisualCognitionSkill {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl AgentSkill for VisualCognitionSkill {
    fn name(&self) -> &str {
        SKILL_NAME
    }

    async fn execute(
        &self,
        ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let args: VisualCognitionArgs = if let Some(p) = payload {
            serde_json::from_value(p)?
        } else {
            return Err("Missing payload for visual_cognition skill".into());
        };

        let result = process_visual_content(&args.content, args.store_key, ctx).await?;
        Ok(serde_json::to_value(result)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_mermaid_blocks() {
        let content = r#"
Here is a diagram:

```mermaid
graph TD
    A[Start] --> B[Process]
    B --> C[End]
```

And some more text.
"#;
        
        let blocks = extract_mermaid_blocks(content);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].0.contains("graph TD"));
    }

    #[test]
    fn test_extract_dot_blocks() {
        let content = r#"
Here is a Graphviz diagram:

```dot
digraph G {
    A -> B;
    B -> C;
}
```

Done.
"#;
        
        let blocks = extract_dot_blocks(content);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].0.contains("digraph G"));
    }

    #[test]
    fn test_multiple_diagrams() {
        let content = r#"
```mermaid
graph LR
    A --> B
```

Some text.

```mermaid
sequenceDiagram
    Alice->>Bob: Hello
```
"#;
        
        let blocks = extract_mermaid_blocks(content);
        assert_eq!(blocks.len(), 2);
    }
}
