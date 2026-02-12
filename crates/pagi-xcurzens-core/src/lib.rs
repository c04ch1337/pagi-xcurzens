//! PAGI XCURZENS — Core library.
//! Shared types and utilities for the XCURZENS authority perimeter.

pub mod identity_orchestrator;
pub mod lead_dispatcher;
pub mod nexus_bridge;
pub mod relations;

pub use identity_orchestrator::{brand_filter, BRAND_NAVY, BRAND_ORANGE, ROOT_SOVEREIGN};
pub use lead_dispatcher::LeadDispatcher;
pub use nexus_bridge::{intent_level, stream_scout_interaction, GeoContext, Intent, NexusBridgeError};
pub use relations::KB07Relations;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
