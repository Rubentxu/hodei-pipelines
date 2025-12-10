//! HWP Agent Entry Point
//!
//! This is the main binary for the HWP (Hodei Worker Protocol) agent.

use hodei_pipelines_domain::{Uuid, WorkerId};
use hwp_pipelines_agent::{Config, Result, connection, monitor};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting HWP Agent...");

    // Load configuration
    let mut config = load_config().await?;

    // Ensure worker_id is a UUID
    if config.worker_id.parse::<Uuid>().is_err() {
        let uuid = Uuid::new_v5(&Uuid::NAMESPACE_DNS, config.worker_id.as_bytes());
        info!(
            "Converted worker_id '{}' to UUID '{}'",
            config.worker_id, uuid
        );
        config.worker_id = uuid.to_string();
    }

    info!(
        "Configuration loaded: server={}, tls={}",
        config.server_url, config.tls_enabled
    );

    // Create agent instance
    let mut agent = connection::Client::new(config.clone());

    // Connect to server with retry
    connect_with_retry(&mut agent, &config).await?;

    // Main execution loop
    run_agent_loop(agent, config).await?;

    info!("HWP Agent shutting down");
    Ok(())
}

async fn load_config() -> Result<Config> {
    match Config::from_env() {
        Ok(config) => {
            config.validate()?;
            Ok(config)
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            Err(e.into())
        }
    }
}

async fn connect_with_retry(agent: &mut connection::Client, config: &Config) -> Result<()> {
    let mut delay_ms = config.reconnect_initial_delay_ms;
    let max_delay = config.reconnect_max_delay_ms;

    loop {
        match agent.connect().await {
            Ok(_) => {
                info!("Connected to HWP server at {}", config.server_url);
                return Ok(());
            }
            Err(e) => {
                warn!("Failed to connect to server: {}", e);
                warn!("Retrying in {}ms...", delay_ms);

                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

                // Exponential backoff
                delay_ms = (delay_ms * 2).min(max_delay);
            }
        }
    }
}

async fn run_agent_loop(mut agent: connection::Client, config: Config) -> Result<()> {
    info!("Entering main agent loop");

    loop {
        // Start heartbeat sender
        let heartbeat_agent = agent.clone();
        let heartbeat_config = monitor::HeartbeatConfig {
            interval_ms: config.resource_sampling_interval_ms,
            max_failures: 3,
            failure_timeout_ms: 30000,
        };
        // We know worker_id is a valid UUID string now
        let worker_uuid = config.worker_id.parse::<Uuid>().unwrap();
        let worker_id = WorkerId(worker_uuid);
        let sampling_interval = config.resource_sampling_interval_ms;

        let heartbeat_handle = tokio::spawn(async move {
            let resource_monitor = monitor::ResourceMonitor::new(sampling_interval);
            let mut sender = monitor::HeartbeatSender::new(
                heartbeat_config,
                resource_monitor,
                worker_id,
                Box::new(heartbeat_agent),
            );
            let pids = vec![std::process::id()];
            if let Err(e) = sender.start(pids).await {
                error!("Heartbeat sender failed: {}", e);
            }
        });

        match agent.handle_stream().await {
            Ok(_) => {
                // Stream ended normally, will auto-reconnect
                warn!("Stream ended, attempting to reconnect...");
                heartbeat_handle.abort();
                connect_with_retry(&mut agent, &config).await?;
            }
            Err(e) => {
                error!("Stream error: {}", e);
                // Attempt to reconnect on error
                heartbeat_handle.abort();
                connect_with_retry(&mut agent, &config).await?;
            }
        }
    }
}
