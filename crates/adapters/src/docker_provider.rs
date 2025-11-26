//! Docker Provider Adapter
//!
//! This module provides a concrete implementation of the WorkerProvider port
//! using Docker CLI for dynamic worker provisioning, avoiding dependency conflicts.

use async_trait::async_trait;
use hodei_core::{Worker, WorkerId};
use hodei_ports::worker_provider::{
    ProviderCapabilities, ProviderConfig, ProviderError, ProviderType, WorkerProvider,
};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::process::Command;

/// Docker worker provider implementation
#[derive(Debug, Clone)]
pub struct DockerProvider {
    name: String,
}

impl DockerProvider {
    pub async fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        if config.provider_type != ProviderType::Docker {
            return Err(ProviderError::InvalidConfiguration(
                "Expected Docker provider configuration".to_string(),
            ));
        }

        // Verify docker CLI is available
        let output = Command::new("docker")
            .arg("--version")
            .output()
            .await
            .map_err(|e| ProviderError::Provider(format!("Docker CLI not found: {}", e)))?;

        if !output.status.success() {
            return Err(ProviderError::Provider(
                "Docker CLI is not working properly".to_string(),
            ));
        }

        tracing::info!(
            "Docker provider initialized: {}",
            String::from_utf8_lossy(&output.stdout)
        );

        Ok(Self { name: config.name })
    }

    fn create_container_name(worker_id: &WorkerId) -> String {
        format!("hodei-worker-{}", worker_id)
    }

    async fn run_docker_command(&self, args: &[&str]) -> Result<String, ProviderError> {
        let output = Command::new("docker")
            .args(args)
            .output()
            .await
            .map_err(|e| {
                ProviderError::Provider(format!("Failed to execute docker command: {}", e))
            })?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ProviderError::Provider(format!(
                "Docker command failed: {}",
                error
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[async_trait]
impl WorkerProvider for DockerProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Docker
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn capabilities(&self) -> Result<ProviderCapabilities, ProviderError> {
        Ok(ProviderCapabilities {
            supports_auto_scaling: true,
            supports_health_checks: true,
            supports_volumes: true,
            max_workers: Some(100),
            estimated_provision_time_ms: 5000,
        })
    }

    async fn create_worker(
        &self,
        worker_id: WorkerId,
        config: ProviderConfig,
    ) -> Result<Worker, ProviderError> {
        let container_name = Self::create_container_name(&worker_id);

        // Use custom image if provided, otherwise default HWP Agent image
        let image = config.custom_image.as_deref().unwrap_or("hwp-agent:latest");

        // Ensure image exists
        tracing::info!("Pulling image {} if needed", image);
        self.run_docker_command(&["pull", image]).await.ok();

        // Build env vars
        let env_vars: Vec<String> = vec![
            "WORKER_ID=placeholder".to_string(),
            "HODEI_SERVER_GRPC_URL=http://hodei-server:50051".to_string(),
        ];

        // Create label for worker ID
        let worker_id_label = format!("hodei.worker.id={}", worker_id);

        // Create container
        let mut create_cmd = vec![
            "create",
            "--name",
            &container_name,
            "-e",
            &env_vars[0],
            "-e",
            &env_vars[1],
            "--label",
            "hodei.worker=true",
            "--label",
            &worker_id_label,
            "-m",
            "4g",
            "--cpus",
            "2",
            "--rm",
            image,
        ];

        self.run_docker_command(&create_cmd)
            .await
            .map_err(|e| ProviderError::Provider(format!("Failed to create container: {}", e)))?;

        // Start container
        self.run_docker_command(&["start", &container_name])
            .await
            .map_err(|e| ProviderError::Provider(format!("Failed to start container: {}", e)))?;

        tracing::info!("Container {} created and started", container_name);

        let worker = Worker::new(
            worker_id.clone(),
            format!("worker-{}", worker_id),
            hodei_core::WorkerCapabilities::new(2, 4096),
        );

        Ok(worker)
    }

    async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<hodei_core::WorkerStatus, ProviderError> {
        let container_name = Self::create_container_name(worker_id);

        // Get container status via inspect
        let output = self
            .run_docker_command(&["inspect", "--format", "{{.State.Status}}", &container_name])
            .await;

        let status_str = match output {
            Ok(status) => status.trim().to_string(),
            Err(_) => {
                return Ok(hodei_core::WorkerStatus::new(
                    worker_id.clone(),
                    hodei_core::WorkerStatus::OFFLINE.to_string(),
                ));
            }
        };

        let status = if status_str == "running" {
            hodei_core::WorkerStatus::new(
                worker_id.clone(),
                hodei_core::WorkerStatus::IDLE.to_string(),
            )
        } else {
            hodei_core::WorkerStatus::new(
                worker_id.clone(),
                hodei_core::WorkerStatus::OFFLINE.to_string(),
            )
        };

        Ok(status)
    }

    async fn stop_worker(&self, worker_id: &WorkerId, graceful: bool) -> Result<(), ProviderError> {
        let container_name = Self::create_container_name(worker_id);

        if graceful {
            self.run_docker_command(&["stop", "--time", "30", &container_name])
                .await?;
        } else {
            self.run_docker_command(&["kill", &container_name]).await?;
        }

        Ok(())
    }

    async fn delete_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let container_name = Self::create_container_name(worker_id);

        // Stop container first (ignore errors if already stopped)
        let _ = self
            .run_docker_command(&["stop", "--time", "5", &container_name])
            .await;

        // Remove container
        self.run_docker_command(&["rm", &container_name]).await?;

        Ok(())
    }

    async fn list_workers(&self) -> Result<Vec<WorkerId>, ProviderError> {
        // List containers with hodei.worker label
        let output = self
            .run_docker_command(&[
                "ps",
                "--filter",
                "label=hodei.worker=true",
                "--format",
                "{{.Label \"hodei.worker.id\"}}",
            ])
            .await?;

        let mut worker_ids = Vec::new();
        for line in output.lines() {
            let worker_id_str = line.trim();
            if !worker_id_str.is_empty() {
                let uuid = uuid::Uuid::parse_str(worker_id_str).map_err(|_| {
                    ProviderError::Provider(format!("Invalid worker ID: {}", worker_id_str))
                })?;
                let worker_id = WorkerId::from_uuid(uuid);
                worker_ids.push(worker_id);
            }
        }

        Ok(worker_ids)
    }
}
