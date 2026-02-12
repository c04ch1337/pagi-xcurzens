//! Sentinel: Active monitoring and intervention capabilities
//! 
//! The Sentinel module provides the "nervous system" for the Sovereign Operator,
//! enabling physical influence over the bare-metal environment through:
//! 
//! - **PhysicalGuard**: Window minimization and input locking during critical states
//! - **HistoryHarvester**: Browser history extraction for behavior analysis
//! - **FileSentinel**: File system monitoring with automatic maintenance triggers
//! - **InputVelocitySensor**: Rage detection through input velocity tracking
//! 
//! These capabilities transform the system from a passive observer to an active sentinel
//! that can detect and respond to critical situations.

pub mod physical_guard;
pub mod history_harvester;
pub mod file_sentinel;
pub mod input_velocity;
pub mod counselor;

// Re-export key types for convenience
pub use physical_guard::{
    SentinelPhysicalGuardAction,
    SentinelPhysicalGuardResult,
    SentinelPhysicalGuardSensor,
};

pub use history_harvester::{
    BrowserType,
    SentinelHistoryEntry,
    SentinelHistoryHarvestResult,
    SentinelHistoryHarvesterAction,
};

pub use file_sentinel::{
    SentinelFileEvent,
    SentinelFileSentinelConfig,
    SentinelFileSentinelResult,
    SentinelFileSentinelSensor,
    create_default_sentinel,
    create_sentinel_for_path,
};

pub use input_velocity::{
    SentinelInputVelocityConfig,
    SentinelInputVelocityMetrics,
    SentinelInputVelocitySensor,
};

pub use counselor::{CounselorPayload, CounselorSkill, CounselorVelocityInput};
