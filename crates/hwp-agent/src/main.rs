//! HWP Agent Entry Point
//!
//! This is the main binary for the HWP (Hodei Worker Protocol) agent.

use hwp_agent::{Config, Result};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting HWP Agent...");

    // Load configuration
    let config = load_config().await?;
    info!(
        "Configuration loaded: server={}, tls={}",
        config.server_url, config.tls_enabled
    );

    // Create agent instance
    let mut agent = hwp_agent::connection::Client::new(config.clone());

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

async fn connect_with_retry(
    agent: &mut hwp_agent::connection::Client,
    config: &Config,
) -> Result<()> {
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

async fn run_agent_loop(mut agent: hwp_agent::connection::Client, config: Config) -> Result<()> {
    info!("Entering main agent loop");

    loop {
        match agent.handle_stream().await {
            Ok(_) => {
                // Stream ended normally, will auto-reconnect
                warn!("Stream ended, attempting to reconnect...");
                connect_with_retry(&mut agent, &config).await?;
            }
            Err(e) => {
                error!("Stream error: {}", e);
                // Attempt to reconnect on error
                connect_with_retry(&mut agent, &config).await?;
            }
        }
    }
}
