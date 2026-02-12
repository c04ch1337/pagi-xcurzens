//! pagi-evolution: Self-Evolving AGI kernel.
//!
//! Dynamic skill synthesis: compile Rust code at runtime, load via `libloading`,
//! and swap implementations without taking the Gateway offline.
//!
//! ## Contract for generated skills (cdylib)
//!
//! The compiled library must export:
//! - `pagi_dynamic_skill_execute(args_json: *const c_char) -> *mut c_char`
//!   Returns a JSON string (allocated; caller frees via below). Null on error.
//! - `pagi_dynamic_skill_free(ptr: *mut c_char)` to free the returned string.
//!
//! ## Evolutionary Versioning & Rollback
//!
//! The `RollbackManager` provides:
//! - **Versioned Storage:** Patches stored as `{skill}_v{timestamp}.rs` with `current_*` symlinks.
//! - **Atomic Rollback:** Symlink swap + hot-reload of previous `.dll`/`.so`.
//! - **Genetic Memory:** SHA-256 hashing of patch DNA to detect evolutionary dead-ends.
//!
//! ## Adversarial Peer Review (Red-Team)
//!
//! The `RedTeamAnalyzer` provides **Phase 4.75: Consensus Gating**:
//! - **Multi-Agent Review:** Sends proposed patches to a secondary LLM for security analysis.
//! - **CVE Checklist:** Injects common vulnerability patterns into the review prompt.
//! - **Consensus Gate:** Auto-rejects Critical/High findings; marks Critical as Lethal Mutations.

mod compiler;
mod loader;
pub mod operator;
pub mod red_team;
pub mod rollback;
mod skill;

pub use compiler::Compiler;
pub use loader::SkillLoader;
pub use operator::{
    ApprovalGate, ApprovalStatus, ChangeSeverity, ProposedChange,
    create_kb08_log_entry, log_forge_approval_to_kb08,
};
pub use red_team::{
    ConsensusGate, ConsensusResult, CveCheckList, RedTeamAnalyzer, RedTeamConfig,
    SecurityFinding, SecurityVerdict, Severity,
};
pub use rollback::{
    DeadEndRecord, GeneticMemory, PatchPerformanceDelta, PatchStatus, PatchVersion,
    RollbackConfig, RollbackManager, SecurityAuditSummary,
};
pub use skill::{DynamicSkill, SkillError};
pub use std::path::PathBuf;
