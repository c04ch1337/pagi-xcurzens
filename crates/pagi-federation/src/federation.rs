//! Federation bridge: MasterServer (The Creator), SatelliteClient, and FederatedBridgeSkill.
//! TaskResults are logged as "Remote Intelligence" into The Creator's multi-layer memory.

use tokio_stream::StreamExt;

use crate::phoenix_federation::{
    phoenix_service_server::PhoenixService, HeartbeatRequest, HeartbeatResponse,
    RegisterNodeRequest, RegisterNodeResponse, Task, TaskResult,
};
use dashmap::DashMap;
use pagi_core::{AgentSkill, MemoryManager, TenantContext};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tonic::{Request, Response, Status};
use tracing::{info, instrument};

const REMOTE_INTELLIGENCE_PREFIX: &str = "remote_intelligence/";
const THE_CREATOR_IDENTITY: &str = "The Creator";
/// First message from Satellite in SubmitTask: task_id empty, summary "READY", node_id set.
const READY_SUMMARY: &str = "READY";

/// Per-satellite info after RegisterNode.
#[derive(Debug, Clone)]
pub struct SatelliteInfo {
    pub node_id: String,
    pub role: String,
    pub host: String,
    pub port: u32,
    pub capabilities: Vec<String>,
}

/// Pending task: oneshot to complete when TaskResult arrives.
type PendingTask = oneshot::Sender<TaskResult>;

/// Master (The Creator) state: registered satellites and pending task completions.
pub struct MasterState {
    pub satellites: DashMap<String, SatelliteInfo>,
    pub pending: DashMap<String, PendingTask>,
    pub memory: Option<Arc<MemoryManager>>,
}

impl MasterState {
    pub fn new() -> Self {
        Self {
            satellites: DashMap::new(),
            pending: DashMap::new(),
            memory: None,
        }
    }

    pub fn with_memory(memory: Arc<MemoryManager>) -> Self {
        Self {
            satellites: DashMap::new(),
            pending: DashMap::new(),
            memory: Some(memory),
        }
    }

    fn log_remote_intelligence(&self, result: &TaskResult, ctx: &TenantContext) {
        let path = format!("{}{}", REMOTE_INTELLIGENCE_PREFIX, result.task_id);
        let value = serde_json::json!({
            "task_id": result.task_id,
            "success": result.success,
            "summary": result.summary,
            "details_json": result.details_json,
            "node_id": result.node_id,
            "role": result.role,
            "completed_at_ms": result.completed_at_ms,
            "energy_used": result.energy_used,
        });
        if let Some(ref mem) = self.memory {
            if let Ok(()) = mem.save_path(ctx, &path, value.to_string().as_bytes()) {
                info!(task_id = %result.task_id, node_id = %result.node_id, "Remote Intelligence logged");
            }
        } else {
            info!(task_id = %result.task_id, summary = %result.summary, "Remote Intelligence (no MemoryManager)");
        }
    }
}

impl Default for MasterState {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle for submitting tasks from The Creator's orchestrator (used by FederatedBridgeSkill).
pub struct FederationHandle {
    state: Arc<MasterState>,
    task_tx: DashMap<String, mpsc::Sender<Result<Task, Status>>>,
}

impl FederationHandle {
    pub fn new(state: Arc<MasterState>) -> Self {
        Self {
            state,
            task_tx: DashMap::new(),
        }
    }

    pub fn register_node_tx(&self, node_id: String, tx: mpsc::Sender<Result<Task, Status>>) {
        self.task_tx.insert(node_id, tx);
    }

    pub fn unregister_node_tx(&self, node_id: &str) {
        self.task_tx.remove(node_id);
    }

    #[instrument(skip(self, ctx))]
    pub async fn submit_task(
        &self,
        goal: &str,
        context_json: &str,
        ctx: &TenantContext,
    ) -> Result<TaskResult, FederationError> {
        let node_id = self
            .state
            .satellites
            .iter()
            .find(|s| s.value().capabilities.iter().any(|c| c == goal))
            .map(|s| s.key().clone())
            .ok_or(FederationError::NoSatelliteForCapability(goal.to_string()))?;

        let tx = self
            .task_tx
            .get(&node_id)
            .ok_or(FederationError::SatelliteDisconnected(node_id.clone()))?;

        let task_id = uuid::Uuid::new_v4().to_string();
        let (result_tx, result_rx) = oneshot::channel();
        self.state.pending.insert(task_id.clone(), result_tx);

        let task = Task {
            task_id: task_id.clone(),
            goal: goal.to_string(),
            context_json: context_json.to_string(),
            tenant_id: ctx.tenant_id.clone(),
            created_at_ms: chrono::Utc::now().timestamp_millis(),
        };

        tx.try_send(Ok(task))
            .map_err(|_| FederationError::SatelliteDisconnected(node_id))?;

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(300),
            result_rx,
        )
        .await
        .map_err(|_| FederationError::Timeout)?
        .map_err(|_| FederationError::ChannelClosed)?;

        self.state.pending.remove(&task_id);
        Ok(result)
    }

    pub fn available_capabilities(&self) -> Vec<String> {
        let mut set = std::collections::HashSet::new();
        for s in self.state.satellites.iter() {
            for c in &s.value().capabilities {
                set.insert(c.clone());
            }
        }
        set.into_iter().collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FederationError {
    #[error("no satellite with capability: {0}")]
    NoSatelliteForCapability(String),
    #[error("satellite disconnected: {0}")]
    SatelliteDisconnected(String),
    #[error("timeout waiting for task result")]
    Timeout,
    #[error("channel closed")]
    ChannelClosed,
    #[error("connect: {0}")]
    Connect(String),
}

// ---------------------------------------------------------------------------
// MasterServer: tonic service implementation
// ---------------------------------------------------------------------------

pub struct MasterServer {
    state: Arc<MasterState>,
    handle: Arc<FederationHandle>,
}

impl MasterServer {
    pub fn new(state: Arc<MasterState>) -> Self {
        let handle = Arc::new(FederationHandle::new(Arc::clone(&state)));
        Self { state, handle }
    }

    pub fn handle(&self) -> Arc<FederationHandle> {
        Arc::clone(&self.handle)
    }

    /// Run the Phoenix gRPC server (The Creator's listener). Use a port in 8001–8099 per architecture.
    /// Call with `Arc::clone(&server)`. For mTLS, use `tonic::transport::Server::builder().tls_config(...)`.
    pub async fn serve(
        this: Arc<Self>,
        addr: std::net::SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::phoenix_federation::phoenix_service_server::PhoenixServiceServer;
        let svc = PhoenixServiceServer::new(this);
        tonic::transport::Server::builder()
            .add_service(svc)
            .serve(addr)
            .await
            .map_err(Into::into)
    }
}

#[tonic::async_trait]
impl PhoenixService for Arc<MasterServer> {
    async fn register_node(
        &self,
        request: Request<RegisterNodeRequest>,
    ) -> Result<Response<RegisterNodeResponse>, Status> {
        self.as_ref().register_node(request).await
    }

    type SubmitTaskStream = tokio_stream::wrappers::ReceiverStream<Result<Task, Status>>;

    async fn submit_task(
        &self,
        request: Request<tonic::Streaming<TaskResult>>,
    ) -> Result<Response<Self::SubmitTaskStream>, Status> {
        self.as_ref().submit_task(request).await
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        self.as_ref().heartbeat(request).await
    }
}

#[tonic::async_trait]
impl PhoenixService for MasterServer {
    async fn register_node(
        &self,
        request: Request<RegisterNodeRequest>,
    ) -> Result<Response<RegisterNodeResponse>, Status> {
        let req = request.into_inner();
        let info = SatelliteInfo {
            node_id: req.node_id.clone(),
            role: req.role.clone(),
            host: req.host.clone(),
            port: req.port,
            capabilities: req.capabilities.clone(),
        };
        self.state.satellites.insert(req.node_id.clone(), info);
        info!(node_id = %req.node_id, role = %req.role, "Satellite registered");
        Ok(Response::new(RegisterNodeResponse {
            accepted: true,
            message: format!("Welcome, {} registered as {}", req.node_id, req.role),
            master_identity: THE_CREATOR_IDENTITY.to_string(),
        }))
    }

    type SubmitTaskStream = tokio_stream::wrappers::ReceiverStream<Result<Task, Status>>;

    async fn submit_task(
        &self,
        request: Request<tonic::Streaming<TaskResult>>,
    ) -> Result<
        Response<tokio_stream::wrappers::ReceiverStream<Result<Task, Status>>>,
        Status,
    > {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::channel::<Result<Task, Status>>(256);
        let tx_reg = tx.clone();
        let state = Arc::clone(&self.state);
        let handle = Arc::clone(&self.handle);

        tokio::spawn(async move {
            while let Some(Ok(result)) = stream.next().await {
                if result.task_id.is_empty() && result.summary == READY_SUMMARY {
                    handle.register_node_tx(result.node_id.clone(), tx_reg.clone());
                    continue;
                }
                if let Some((_, pending_tx)) = state.pending.remove(&result.task_id) {
                    let _ = pending_tx.send(result.clone());
                }
                let ctx = TenantContext {
                    tenant_id: result.tenant_id.clone(),
                    correlation_id: None,
                    agent_id: None,
                };
                state.log_remote_intelligence(&result, &ctx);
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();
        info!(
            node_id = %req.node_id,
            bandwidth_mbps = %req.bandwidth_mbps,
            cpu_percent = %req.cpu_percent,
            "Heartbeat"
        );
        Ok(Response::new(HeartbeatResponse { ack: true }))
    }
}

// FederatedBridgeSkill: implements AgentSkill so The Creator can use_tool("red_team_scan", ...) transparently.
/// One skill per capability (e.g. "red_team_scan"); execute() submits task to a Satellite and returns the result.
pub struct FederatedBridgeSkill {
    capability: String,
    handle: Arc<FederationHandle>,
}

impl FederatedBridgeSkill {
    pub fn new(capability: String, handle: Arc<FederationHandle>) -> Self {
        Self { capability, handle }
    }
}

#[async_trait::async_trait]
impl AgentSkill for FederatedBridgeSkill {
    fn name(&self) -> &str {
        &self.capability
    }

    async fn execute(
        &self,
        ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let context_json = payload
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .ok()
            .flatten()
            .unwrap_or_default();
        let result = self
            .handle
            .submit_task(&self.capability, &context_json, ctx)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        Ok(serde_json::json!({
            "success": result.success,
            "summary": result.summary,
            "details": result.details_json,
            "node_id": result.node_id,
            "role": result.role,
            "energy_used": result.energy_used,
        }))
    }
}

// ---------------------------------------------------------------------------
// SatelliteClient: runs on a Satellite, connects to The Creator, stays persistent.
// Sends heartbeat every 30s; receives Tasks and returns TaskResults via executor.
// ---------------------------------------------------------------------------

use crate::phoenix_federation::phoenix_service_client::PhoenixServiceClient;
use std::future::Future;
use std::time::Duration;

const HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// Satellite client: connect to The Creator's IP, register, maintain SubmitTask stream, heartbeat every 30s.
pub struct SatelliteClient {
    pub node_id: String,
    pub role: String,
    pub host: String,
    pub port: u32,
    pub capabilities: Vec<String>,
}

impl SatelliteClient {
    pub fn new(
        node_id: String,
        role: String,
        host: String,
        port: u32,
        capabilities: Vec<String>,
    ) -> Self {
        Self {
            node_id,
            role,
            host,
            port,
            capabilities,
        }
    }

    /// Run the satellite: connect to The Creator at `creator_addr` (e.g. "https://192.168.1.2:50052"),
    /// register, open bi-di stream (send READY, then receive Task -> run executor -> send TaskResult),
    /// and send heartbeat every 30s.
    /// `executor` runs each Task locally and returns the TaskResult to send back.
    pub async fn run<F, Fut>(
        &self,
        creator_addr: &str,
        mut executor: F,
    ) -> Result<(), FederationError>
    where
        F: FnMut(Task) -> Fut + Send + 'static,
        Fut: Future<Output = TaskResult> + Send + 'static,
    {
        let channel = tonic::transport::Channel::from_shared(creator_addr.to_string())
            .map_err(|e| FederationError::Connect(e.to_string()))?
            .connect()
            .await
            .map_err(|e| FederationError::Connect(e.to_string()))?;

        let mut client = PhoenixServiceClient::new(channel);

        let req = RegisterNodeRequest {
            node_id: self.node_id.clone(),
            role: self.role.clone(),
            host: self.host.clone(),
            port: self.port,
            capabilities: self.capabilities.clone(),
            hardware: Some(crate::phoenix_federation::HardwareSpecs {
                cpu_cores: 0,
                ram_mb: 0,
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
            }),
        };
        let _ = client
            .register_node(req)
            .await
            .map_err(|e| FederationError::Connect(e.to_string()))?;

        let (tx, rx) = mpsc::unbounded_channel::<TaskResult>();
        let out_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

        let task_tx = tx.clone();
        let ready = TaskResult {
            task_id: String::new(),
            success: true,
            summary: READY_SUMMARY.to_string(),
            details_json: String::new(),
            node_id: self.node_id.clone(),
            role: self.role.clone(),
            completed_at_ms: chrono::Utc::now().timestamp_millis(),
            energy_used: 0,
            tenant_id: String::new(),
        };
        task_tx.send(ready).ok();

        let mut in_stream = client
            .submit_task(tonic::Request::new(out_stream))
            .await
            .map_err(|e| FederationError::Connect(e.to_string()))?
            .into_inner();

        let node_id = self.node_id.clone();
        let heartbeat_addr = creator_addr.to_string();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
            loop {
                interval.tick().await;
                if let Ok(ep) =
                    tonic::transport::Endpoint::from_shared(heartbeat_addr.clone())
                {
                    if let Ok(ch) = ep.connect().await {
                        let mut c = PhoenixServiceClient::new(ch);
                        let _ = c
                            .heartbeat(HeartbeatRequest {
                                node_id: node_id.clone(),
                                bandwidth_mbps: 0.0,
                                cpu_percent: 0.0,
                                ram_used_mb: 0,
                                at_ms: chrono::Utc::now().timestamp_millis(),
                            })
                            .await;
                    }
                }
            }
        });

        while let Some(Ok(task)) = in_stream.next().await {
            let result = executor(task).await;
            if tx.send(result).is_err() {
                break;
            }
        }

        Ok(())
    }
}
