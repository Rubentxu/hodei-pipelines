//! Service management for E2E testing
//!
//! This module handles starting and stopping the application services
//! (orchestrator, scheduler, worker manager) in test mode.

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tracing::info;

use super::config::TestConfig;

/// Handle to a running Orchestrator service
#[derive(Debug)]
pub struct OrchestratorHandle {
    child: Arc<Child>,
    url: String,
}

impl Clone for OrchestratorHandle {
    fn clone(&self) -> Self {
        Self {
            child: Arc::clone(&self.child),
            url: self.url.clone(),
        }
    }
}

impl OrchestratorHandle {
    /// Starts the Orchestrator service
    pub async fn start(
        config: &TestConfig,
        nats_url: &str,
        tracing_url: &str,
        database_url: &str,
    ) -> Result<Self> {
        info!("🚀 Starting Orchestrator service");

        let child = Command::new("cargo")
            .args(&[
                "run",
                "--bin",
                "orchestrator",
                "--",
                "--log-level=info",
                &format!("--nats-url={}", nats_url),
                &format!("--database-url={}", database_url),
                &format!("--tracing-url={}", tracing_url),
                &format!("--port={}", config.orchestrator_port),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to start Orchestrator")?;

        // Wait for the service to be ready
        let url = format!("http://localhost:{}", config.orchestrator_port);
        Self::wait_for_service(&url, 30).await?;

        Ok(Self {
            child: Arc::new(child),
            url,
        })
    }

    /// Waits for a service to be ready
    async fn wait_for_service(url: &str, timeout_secs: u64) -> Result<()> {
        let client = Client::new();
        let start = std::time::Instant::now();

        loop {
            match client.get(&format!("{}/health", url)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("✓ Service at {} is ready", url);
                    return Ok(());
                }
                _ => {
                    if start.elapsed() > std::time::Duration::from_secs(timeout_secs) {
                        return Err(anyhow!(
                            "Service at {} not ready after {} seconds",
                            url,
                            timeout_secs
                        ));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Checks if the service is healthy
    pub async fn is_healthy(&self) -> bool {
        let client = Client::new();
        match client.get(&format!("{}/health", self.url)).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Stops the Orchestrator service
    pub async fn stop(self) -> Result<()> {
        info!("🛑 Stopping Orchestrator service");

        let mut child = Arc::try_unwrap(self.child)
            .unwrap_or_else(|_| panic!("Failed to unwrap Orchestrator child"));

        child
            .kill()
            .await
            .context("Failed to kill Orchestrator process")?;
        child
            .wait()
            .await
            .context("Failed to wait for Orchestrator process")?;

        Ok(())
    }

    /// Returns the service URL
    pub fn url(&self) -> &str {
        &self.url
    }
}

/// Handle to a running Scheduler service
#[derive(Debug)]
pub struct SchedulerHandle {
    child: Arc<Child>,
    url: String,
}

impl Clone for SchedulerHandle {
    fn clone(&self) -> Self {
        Self {
            child: Arc::clone(&self.child),
            url: self.url.clone(),
        }
    }
}

impl SchedulerHandle {
    /// Starts the Scheduler service
    pub async fn start(config: &TestConfig, nats_url: &str, tracing_url: &str) -> Result<Self> {
        info!("🚀 Starting Scheduler service");

        let child = Command::new("cargo")
            .args(&[
                "run",
                "--bin",
                "scheduler",
                "--",
                "--log-level=info",
                &format!("--nats-url={}", nats_url),
                &format!("--tracing-url={}", tracing_url),
                &format!("--port={}", config.scheduler_port),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to start Scheduler")?;

        // Wait for the service to be ready
        let url = format!("http://localhost:{}", config.scheduler_port);
        OrchestratorHandle::wait_for_service(&url, 30).await?;

        Ok(Self {
            child: Arc::new(child),
            url,
        })
    }

    /// Checks if the service is healthy
    pub async fn is_healthy(&self) -> bool {
        let client = Client::new();
        match client.get(&format!("{}/health", self.url)).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Stops the Scheduler service
    pub async fn stop(self) -> Result<()> {
        info!("🛑 Stopping Scheduler service");

        let mut child = Arc::try_unwrap(self.child)
            .unwrap_or_else(|_| panic!("Failed to unwrap Scheduler child"));

        child
            .kill()
            .await
            .context("Failed to kill Scheduler process")?;
        child
            .wait()
            .await
            .context("Failed to wait for Scheduler process")?;

        Ok(())
    }

    /// Returns the service URL
    pub fn url(&self) -> &str {
        &self.url
    }
}

/// Handle to a running Worker Manager service
#[derive(Debug)]
pub struct WorkerManagerHandle {
    child: Arc<Child>,
    url: String,
}

impl Clone for WorkerManagerHandle {
    fn clone(&self) -> Self {
        Self {
            child: Arc::clone(&self.child),
            url: self.url.clone(),
        }
    }
}

impl WorkerManagerHandle {
    /// Starts the Worker Manager service
    pub async fn start(config: &TestConfig, nats_url: &str, tracing_url: &str) -> Result<Self> {
        info!("🚀 Starting Worker Manager service");

        let child = Command::new("cargo")
            .args(&[
                "run",
                "--bin",
                "worker-manager",
                "--",
                "--log-level=info",
                &format!("--nats-url={}", nats_url),
                &format!("--tracing-url={}", tracing_url),
                &format!("--port={}", config.worker_manager_port),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to start Worker Manager")?;

        // Wait for the service to be ready
        let url = format!("http://localhost:{}", config.worker_manager_port);
        OrchestratorHandle::wait_for_service(&url, 30).await?;

        Ok(Self {
            child: Arc::new(child),
            url,
        })
    }

    /// Starts a worker instance
    pub async fn start_worker(&self, worker_type: &str, nats_url: &str) -> Result<String> {
        let client = Client::new();
        let response = client
            .post(&format!("{}/api/v1/workers", self.url))
            .json(&serde_json::json!({
                "type": worker_type,
                "nats_url": nats_url,
            }))
            .send()
            .await
            .context("Failed to start worker")?;

        let worker_info: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse worker response")?;

        let worker_id = worker_info["id"].as_str().unwrap_or("unknown").to_string();

        Ok(worker_id)
    }

    /// Stops a running worker
    pub async fn stop_worker(&self, worker_id: &str) -> Result<()> {
        let client = Client::new();
        let _ = client
            .delete(&format!("{}/api/v1/workers/{}", self.url, worker_id))
            .send()
            .await
            .context("Failed to stop worker")?;

        Ok(())
    }

    /// Checks if the service is healthy
    pub async fn is_healthy(&self) -> bool {
        let client = Client::new();
        match client.get(&format!("{}/health", self.url)).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Stops the Worker Manager service
    pub async fn stop(self) -> Result<()> {
        info!("🛑 Stopping Worker Manager service");

        let mut child = Arc::try_unwrap(self.child)
            .unwrap_or_else(|_| panic!("Failed to unwrap Worker Manager child"));

        child
            .kill()
            .await
            .context("Failed to kill Worker Manager process")?;
        child
            .wait()
            .await
            .context("Failed to wait for Worker Manager process")?;

        Ok(())
    }

    /// Returns the service URL
    pub fn url(&self) -> &str {
        &self.url
    }
}
