//! Hodei Pipelines Server - Production Bootstrap

use axum::routing::get;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use hodei_server::bootstrap::{initialize_server, log_config_summary};

// Re-export the pipeline API types for the binary
use hodei_server::api_router::create_api_router;

/// Mock Scheduler for development/testing
#[derive(Debug, Clone)]
pub struct MockScheduler {
    transmitters: Arc<
        RwLock<
            HashMap<
                hodei_pipelines_core::WorkerId,
                tokio::sync::mpsc::UnboundedSender<
                    Result<hodei_pipelines_proto::ServerMessage, hodei_pipelines_ports::scheduler_port::SchedulerError>,
                >,
            >,
        >,
    >,
}

impl MockScheduler {
    pub fn new() -> Self {
        Self {
            transmitters: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl hodei_pipelines_ports::scheduler_port::SchedulerPort for MockScheduler {
    async fn register_worker(
        &self,
        _worker: &hodei_pipelines_core::Worker,
    ) -> Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        info!("MockScheduler: Registering worker");
        Ok(())
    }

    async fn unregister_worker(
        &self,
        worker_id: &hodei_pipelines_core::WorkerId,
    ) -> Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        info!("MockScheduler: Unregistering worker {}", worker_id);
        let mut transmitters = self.transmitters.write().await;
        transmitters.remove(worker_id);
        Ok(())
    }

    async fn get_registered_workers(
        &self,
    ) -> Result<Vec<hodei_pipelines_core::WorkerId>, hodei_pipelines_ports::scheduler_port::SchedulerError> {
        let transmitters = self.transmitters.read().await;
        Ok(transmitters.keys().cloned().collect())
    }

    async fn register_transmitter(
        &self,
        worker_id: &hodei_pipelines_core::WorkerId,
        transmitter: tokio::sync::mpsc::UnboundedSender<
            Result<hodei_pipelines_proto::ServerMessage, hodei_pipelines_ports::scheduler_port::SchedulerError>,
        >,
    ) -> Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        info!(
            "MockScheduler: Registering transmitter for worker {}",
            worker_id
        );
        let mut transmitters = self.transmitters.write().await;
        transmitters.insert(worker_id.clone(), transmitter);
        Ok(())
    }

    async fn unregister_transmitter(
        &self,
        worker_id: &hodei_pipelines_core::WorkerId,
    ) -> Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        info!(
            "MockScheduler: Unregistering transmitter for worker {}",
            worker_id
        );
        let mut transmitters = self.transmitters.write().await;
        transmitters.remove(worker_id);
        Ok(())
    }

    async fn send_to_worker(
        &self,
        worker_id: &hodei_pipelines_core::WorkerId,
        message: hodei_pipelines_proto::ServerMessage,
    ) -> Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        let transmitters = self.transmitters.read().await;
        if let Some(tx) = transmitters.get(worker_id) {
            tx.send(Ok(message)).map_err(|_| {
                hodei_pipelines_ports::scheduler_port::SchedulerError::Internal(format!(
                    "Failed to send to worker {}",
                    worker_id
                ))
            })?;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    info!("🚀 Starting Hodei Pipelines Server");

    let server_components = initialize_server().await.map_err(|e| {
        tracing::error!("❌ Failed to initialize server: {}", e);
        e
    })?;

    log_config_summary(&server_components.config);
    info!("🌐 Setting up HTTP routes...");

    // Clone what we need before moving server_components
    let port = server_components.config.server.port;
    let host = server_components.config.server.host.clone();

    // Create the main API router with all routes
    let app = create_api_router(server_components.clone());
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
    info!("✅ Server listening on http://{}:{}", host, port);

    // Setup graceful shutdown using a broadcast channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Spawn task to listen for OS signals
    let mut shutdown_tx_sig = shutdown_tx.clone();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("🛑 Received Ctrl-C, initiating graceful shutdown...");
                let _ = shutdown_tx_sig.send(()).await;
            }
            Err(err) => {
                tracing::error!("Failed to listen for Ctrl-C signal: {}", err);
            }
        }
    });

    // Start gRPC server in a separate task
    let grpc_addr = format!("{}:50051", host).parse()?;
    let event_publisher = server_components.event_publisher.clone();

    tokio::spawn(async move {
        info!("🚀 Starting gRPC Server on {}", grpc_addr);
        // Initialize scheduler - with mock for now
        let scheduler = Arc::new(MockScheduler::new());
        let hwp_service = hodei_server::grpc::HwpService::new(scheduler, event_publisher);

        let grpc_future = tonic::transport::Server::builder()
            .add_service(hodei_pipelines_proto::WorkerServiceServer::new(hwp_service))
            .serve(grpc_addr);

        if let Err(e) = grpc_future.await {
            tracing::error!("❌ gRPC Server failed: {}", e);
        }
    });

    // Wait for shutdown signal
    let mut shutdown_rx = shutdown_rx;
    tokio::select! {
        result = axum::serve(listener, app) => {
            match result {
                Ok(()) => {
                    info!("🔄 HTTP server stopped");
                    let _ = shutdown_tx.send(()).await;
                }
                Err(e) => {
                    tracing::error!("❌ HTTP server error: {}", e);
                    return Err(e.into());
                }
            }
        }
        _ = shutdown_rx.recv() => {
            info!("🛑 Shutdown signal received, stopping server...");
            info!("🧹 Cleaning up resources...");
        }
    }

    info!("✅ Server shutdown complete");
    Ok(())
}

// Local mocks removed in favor of shared implementation in api_router.rs
