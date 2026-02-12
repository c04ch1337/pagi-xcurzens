//! SAO security utilities: redaction of protected terms in meeting minutes and transcripts.

pub mod redaction;

pub use redaction::{SAORedactor, PROTECTED_PLACEHOLDER};
