//! Prompt templates for orchestrator and Mimir minute generation.

pub mod mimir_minutes;
pub mod mimir_title;

pub use mimir_minutes::{
    mimir_minutes_user_prompt, MIMIR_MINUTES_SYSTEM, MIMIR_MINUTES_USER_TEMPLATE,
};
pub use mimir_title::{
    mimir_title_user_prompt, MIMIR_TITLE_SYSTEM, MIMIR_TITLE_USER_TEMPLATE,
};
