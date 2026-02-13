# pagi-federation

Federated Communication Layer for Phoenix AGI (SAO): gRPC-based nervous system connecting **The Creator** (Master Orchestrator) to remote Satellites. Bare-metal, optional mTLS, binary Protocol Buffers.

## Protocol

- **PhoenixService** (see `proto/phoenix.proto`):
  - `RegisterNode`: Satellites announce Role (RedTeam, Finance, etc.) and capabilities.
  - `SubmitTask`: Bi-directional stream — The Creator sends `Task`, Satellites return `TaskResult` (logged as "Remote Intelligence").
  - `Heartbeat`: Satellites report bandwidth/CPU every 30s.

## Usage

### The Creator (Master)

1. Create state (optionally with `MemoryManager` for Remote Intelligence logging):
   ```rust
   let state = Arc::new(MasterState::with_memory(memory));
   let server = Arc::new(MasterServer::new(state));
   let handle = server.handle();
   ```
2. Register federated skills with the orchestrator so The Creator can call them by name:
   ```rust
   registry.register(Arc::new(FederatedBridgeSkill::new(
       "red_team_scan".into(),
       handle,
   )));
   ```
3. Run the gRPC server (e.g. port 8002):
   ```rust
   server.clone().serve("0.0.0.0:8002".parse()?).await?;
   ```

### Satellite

1. Build a client and run with an executor that turns `Task` into `TaskResult`:
   ```rust
   let client = SatelliteClient::new(
       "kali-1".into(),
       "RedTeam".into(),
       "192.168.1.10".into(),
       0,
       vec!["red_team_scan".into()],
   );
   client.run("http://The Creator-ip:8002", |task| async move {
       // Run task locally, return TaskResult
       run_local_scan(&task).await
   }).await?;
   ```
2. First message on the SubmitTask stream must be a READY: `task_id=""`, `summary="READY"`, `node_id` set.

### mTLS

Use `server_tls_config` / `client_tls_config` from `pagi_federation::mtls` with server cert, key, and client CA (and for client: client cert, key, server CA). Wire the resulting `ServerConfig` / `ClientConfig` into tonic’s TLS helpers when building the server or channel.

## Dependencies (Cargo.toml)

- `tonic` (with `tls`, `tls-roots`), `prost`, `tokio`, `dashmap`, `rustls`, `rustls-pemfile`, `tokio-rustls`, `tokio-stream`, `futures-util`, `pagi-core`.
