//! Federated Communication Layer for Phoenix AGI (SAO).
//!
//! gRPC-based nervous system connecting The Creator (Master Orchestrator) to remote Satellites.
//! Bare-metal, mTLS-secured, binary Protocol Buffers for bandwidth efficiency.

pub mod federation;
pub mod mtls;

// Generated gRPC types (package federation); flat module, no nested "federation".
#[allow(dead_code, unreachable_pub)]
pub mod phoenix_federation {
    include!(concat!(env!("OUT_DIR"), "/federation.rs"));
}

pub use federation::{
    FederatedBridgeSkill, FederationHandle, MasterServer, SatelliteClient, SatelliteInfo,
};
pub use mtls::{client_tls_config, server_tls_config};
pub use phoenix_federation::{
    phoenix_service_client::PhoenixServiceClient,
    phoenix_service_server::{PhoenixService, PhoenixServiceServer},
    HardwareSpecs, HeartbeatRequest, HeartbeatResponse, RegisterNodeRequest,
    RegisterNodeResponse, Task, TaskResult,
};
