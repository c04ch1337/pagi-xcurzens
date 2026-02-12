//! Dynamic skill trait and error type.

use serde_json::Value;
use thiserror::Error;

/// Error from a dynamically loaded or in-process skill.
#[derive(Error, Debug)]
pub enum SkillError {
    #[error("skill execution failed: {0}")]
    Execution(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("skill not loaded: {0}")]
    NotLoaded(String),
    #[error("load error: {0}")]
    Load(String),
}

/// Trait for skills that can be executed with JSON args and return JSON.
/// Used both by in-process adapters and by the loader when wrapping a loaded .so/.dll.
pub trait DynamicSkill: Send + Sync {
    /// Execute the skill with the given JSON arguments.
    fn execute(&self, args: Value) -> Result<Value, SkillError>;
}
