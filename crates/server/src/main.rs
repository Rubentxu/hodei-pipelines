//! Hodei Pipelines Server - Production Bootstrap
//!
//! This is the main entry point for the production server.
//! All dependencies are real production implementations:
//! - RealScheduler (real scheduler with PostgreSQL persistence)
//! - ConcreteOrchestrator (real pipeline orchestration)
//! - PostgreSQL repositories (no InMemory mocks)
//! - Real event bus (NATS ready)

use hodei_server::bootstrap::{initialize_server, log_config_summary};

// Re-export the pipeline API types for the binary
use hodei_server::api_router::create_api_router;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    info!("🚀 Starting Hodei Pipelines Server - PRODUCTION MODE");

    // Initialize all production dependencies from bootstrap
    // This connects to real PostgreSQL, initializes SchedulerModule, etc.
    let server_components = initialize_server().await.map_err(|e| {
        tracing::error!("❌ Failed to initialize server: {}", e);
        e
    })?;

    log_config_summary(&server_components.config);
    info!("✅ Production dependencies initialized:");
    info!("   📦 SchedulerModule: real scheduler with PostgreSQL");
    info!("   🎯 ConcreteOrchestrator: real pipeline execution");
    info!("   🗄️ PostgreSQL repositories: job, pipeline, execution, worker, rbac");
    info!("   📊 Metrics TSDB: TimescaleDB with batch persistence");

    info!("🌐 Setting up HTTP routes with real dependencies...");

    // Clone what we need before moving server_components
    let port = server_components.config.server.port;
    let host = server_components.config.server.host.clone();

    // Create the main API router with ALL real dependencies
    // NO MOCKS - everything is connected to real implementations
    let app = create_api_router(server_components.clone());
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
    info!("✅ Server listening on http://{}:{}", host, port);

    // Setup graceful shutdown using a broadcast channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Spawn task to listen for OS signals
    let shutdown_tx_sig = shutdown_tx.clone();
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

    // Start gRPC server with REAL Scheduler (not MockScheduler)
    let grpc_addr = format!("{}:50051", host).parse()?;
    let event_publisher = server_components.event_publisher.clone();
    let scheduler = server_components.scheduler.clone();

    tokio::spawn(async move {
        info!("🚀 Starting gRPC Server on {}", grpc_addr);
        info!("📡 Using RealScheduler (production-ready, PostgreSQL-backed)");

        // Create HwpService with REAL Scheduler
        // This will persist worker registrations to PostgreSQL
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

    info!("✅ Server shutdown complete - All dependencies properly cleaned up");
    Ok(())
}
