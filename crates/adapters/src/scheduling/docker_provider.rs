//! Docker Provider Adapter
//!
//! This module provides a concrete implementation of the WorkerProvider port
//! using bollard-next for dynamic worker provisioning.

use async_trait::async_trait;
use bollard_next::Docker;
use bollard_next::container::{
    Config, CreateContainerOptions, InspectContainerOptions, KillContainerOptions,
    ListContainersOptions, RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
};
use bollard_next::image::CreateImageOptions;
use futures::StreamExt;
use hodei_pipelines_domain::{Worker, WorkerId};
use hodei_pipelines_ports::worker_provider::{
    ProviderCapabilities, ProviderConfig, ProviderError, ProviderType, WorkerProvider,
};
use std::collections::HashMap;
use tracing::info;

/// Docker worker provider implementation
#[derive(Debug, Clone)]
pub struct DockerProvider {
    docker: Docker,
    name: String,
}

impl DockerProvider {
    pub async fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        if config.provider_type != ProviderType::Docker {
            return Err(ProviderError::InvalidConfiguration(
                "Expected Docker provider configuration".to_string(),
            ));
        }

        #[cfg(unix)]
        let docker = Docker::connect_with_socket_defaults()
            .map_err(|e| ProviderError::Provider(format!("Failed to connect to Docker: {}", e)))?;

        #[cfg(windows)]
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| ProviderError::Provider(format!("Failed to connect to Docker: {}", e)))?;

        info!("Docker provider initialized with bollard-next client");

        Ok(Self {
            docker,
            name: config.name,
        })
    }

    async fn ensure_image(&self, image: &str) -> Result<(), ProviderError> {
        let mut stream = self.docker.create_image(
            Some(CreateImageOptions {
                from_image: image,
                ..Default::default()
            }),
            None,
            None,
        );

        while let Some(result) = stream.next().await {
            match result {
                Ok(_) => {}
                Err(e) => {
                    return Err(ProviderError::Provider(format!(
                        "Failed to pull image '{}': {}",
                        image, e
                    )));
                }
            }
        }

        Ok(())
    }

    fn create_container_name(worker_id: &WorkerId) -> String {
        format!("hodei-worker-{}", worker_id)
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

        // Validate and get image from template
        let template = &config.template;
        if let Err(e) = template.validate() {
            return Err(ProviderError::InvalidConfiguration(format!(
                "Template validation failed: {}",
                e
            )));
        }

        // Use custom image if provided, otherwise use template image
        let image = config
            .custom_image
            .as_deref()
            .unwrap_or(&template.container.image);

        self.ensure_image(image).await?;

        // Convert template env vars to Vec<String>
        let mut env_vars = Vec::new();
        for env in &template.env {
            env_vars.push(format!("{}={}", env.name, env.value));
        }

        // Add default env vars if not already present
        if !env_vars.iter().any(|e| e.starts_with("WORKER_ID=")) {
            env_vars.push(format!("WORKER_ID={}", worker_id));
        }
        if !env_vars
            .iter()
            .any(|e| e.starts_with("HODEI_SERVER_GRPC_URL="))
        {
            env_vars.push("HODEI_SERVER_GRPC_URL=http://hodei-server:50051".to_string());
        }

        // Convert resource requirements to Docker format
        let mut memory = None;
        let mut nano_cpus = None;

        if let Some(mem) = &template.resources.memory {
            // Parse memory string like "4Gi", "512Mi"
            if mem.ends_with("Gi") {
                let size_gb: i64 = mem.trim_end_matches("Gi").parse().unwrap_or(4);
                memory = Some(size_gb * 1024 * 1024 * 1024);
            } else if mem.ends_with("Mi") {
                let size_mb: i64 = mem.trim_end_matches("Mi").parse().unwrap_or(4096);
                memory = Some(size_mb * 1024 * 1024);
            }
        }

        if let Some(cpu) = &template.resources.cpu {
            // Parse CPU string like "2", "2000m"
            if cpu.ends_with("m") {
                let millicores: i64 = cpu.trim_end_matches("m").parse().unwrap_or(2000);
                nano_cpus = Some(millicores * 1_000_000);
            } else {
                let cores: i64 = cpu.parse().unwrap_or(2);
                nano_cpus = Some(cores * 1_000_000_000);
            }
        }

        // Build labels from template
        let mut labels = HashMap::new();
        labels.insert("hodei.worker".to_string(), "true".to_string());
        labels.insert("hodei.worker.id".to_string(), worker_id.to_string());

        // Add template labels
        for (key, value) in &template.labels {
            labels.insert(key.clone(), value.clone());
        }

        let container_config = Config {
            image: Some(image.to_string()),
            env: Some(env_vars),
            labels: Some(labels),
            host_config: Some(bollard_next::service::HostConfig {
                memory,
                nano_cpus,
                auto_remove: Some(true),
                ..Default::default()
            }),
            working_dir: template.working_dir.clone(),
            ..Default::default()
        };

        let create_options = CreateContainerOptions {
            name: container_name.clone(),
            ..Default::default()
        };

        self.docker
            .create_container(Some(create_options), container_config)
            .await
            .map_err(|e| {
                ProviderError::Provider(format!(
                    "Failed to create container '{}': {}",
                    container_name, e
                ))
            })?;

        self.docker
            .start_container::<&str>(&container_name, Some(StartContainerOptions::default()))
            .await
            .map_err(|e| {
                ProviderError::Provider(format!(
                    "Failed to start container '{}': {}",
                    container_name, e
                ))
            })?;

        let worker = Worker::new(
            worker_id.clone(),
            format!("worker-{}", worker_id),
            hodei_pipelines_domain::WorkerCapabilities::new(2, 4096),
        );

        Ok(worker)
    }

    async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<hodei_pipelines_domain::WorkerStatus, ProviderError> {
        let container_name = Self::create_container_name(worker_id);

        let container_info = self
            .docker
            .inspect_container(&container_name, Some(InspectContainerOptions::default()))
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    ProviderError::NotFound(format!("Container '{}' not found", container_name))
                } else {
                    ProviderError::Provider(format!("Failed to inspect container: {}", e))
                }
            })?;

        let status = match container_info
            .state
            .as_ref()
            .and_then(|s| s.status.as_ref())
        {
            Some(bollard_next::models::ContainerStateStatusEnum::RUNNING) => {
                hodei_pipelines_domain::WorkerStatus::new(
                    worker_id.clone(),
                    hodei_pipelines_domain::WorkerStatus::IDLE.to_string(),
                )
            }
            _ => hodei_pipelines_domain::WorkerStatus::new(
                worker_id.clone(),
                hodei_pipelines_domain::WorkerStatus::OFFLINE.to_string(),
            ),
        };

        Ok(status)
    }

    async fn stop_worker(&self, worker_id: &WorkerId, graceful: bool) -> Result<(), ProviderError> {
        let container_name = Self::create_container_name(worker_id);

        if graceful {
            self.docker
                .stop_container(&container_name, Some(StopContainerOptions { t: 30 }))
                .await
                .map_err(|e| {
                    ProviderError::Provider(format!(
                        "Failed to stop container '{}': {}",
                        container_name, e
                    ))
                })?;
        } else {
            self.docker
                .kill_container::<&str>(
                    &container_name,
                    Some(KillContainerOptions { signal: "SIGKILL" }),
                )
                .await
                .map_err(|e| {
                    ProviderError::Provider(format!(
                        "Failed to kill container '{}': {}",
                        container_name, e
                    ))
                })?;
        }

        Ok(())
    }

    async fn delete_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let container_name = Self::create_container_name(worker_id);

        let _ = self
            .docker
            .stop_container(&container_name, Some(StopContainerOptions { t: 5 }))
            .await;

        self.docker
            .remove_container(
                &container_name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| {
                ProviderError::Provider(format!(
                    "Failed to remove container '{}': {}",
                    container_name, e
                ))
            })?;

        Ok(())
    }

    async fn list_workers(&self) -> Result<Vec<WorkerId>, ProviderError> {
        let mut filters = HashMap::new();
        filters.insert("label".to_string(), vec!["hodei.worker=true".to_string()]);

        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions {
                all: false,
                filters,
                ..Default::default()
            }))
            .await
            .map_err(|e| ProviderError::Provider(format!("Failed to list containers: {}", e)))?;

        let mut worker_ids = Vec::new();
        for container in containers {
            if let Some(labels) = container.labels
                && let Some(worker_id_str) = labels.get("hodei.worker.id")
            {
                match uuid::Uuid::parse_str(worker_id_str) {
                    Ok(uuid) => {
                        let worker_id = WorkerId::from_uuid(uuid);
                        worker_ids.push(worker_id);
                    }
                    Err(_) => {
                        // Skip invalid worker IDs
                    }
                }
            }
        }

        Ok(worker_ids)
    }

    async fn create_ephemeral_worker(
        &self,
        worker_id: WorkerId,
        config: ProviderConfig,
        auto_cleanup_seconds: Option<u64>,
    ) -> Result<Worker, ProviderError> {
        let container_name = Self::create_container_name(&worker_id);

        // Validate and get image from template
        let template = &config.template;
        if let Err(e) = template.validate() {
            return Err(ProviderError::InvalidConfiguration(format!(
                "Template validation failed: {}",
                e
            )));
        }

        // Use custom image if provided, otherwise use template image
        let image = config
            .custom_image
            .as_deref()
            .unwrap_or(&template.container.image);

        self.ensure_image(image).await?;

        // Convert template env vars to Vec<String>
        let mut env_vars = Vec::new();
        for env in &template.env {
            env_vars.push(format!("{}={}", env.name, env.value));
        }

        // Add default env vars if not already present
        if !env_vars.iter().any(|e| e.starts_with("WORKER_ID=")) {
            env_vars.push(format!("WORKER_ID={}", worker_id));
        }
        if !env_vars
            .iter()
            .any(|e| e.starts_with("HODEI_SERVER_GRPC_URL="))
        {
            env_vars.push("HODEI_SERVER_GRPC_URL=http://hodei-server:50051".to_string());
        }

        // Add auto-cleanup environment variable if specified
        if let Some(cleanup_seconds) = auto_cleanup_seconds {
            env_vars.push(format!("AUTO_CLEANUP_SECONDS={}", cleanup_seconds));
        }

        // Convert resource requirements to Docker format
        let mut memory = None;
        let mut nano_cpus = None;

        if let Some(mem) = &template.resources.memory {
            // Parse memory string like "4Gi", "512Mi"
            if mem.ends_with("Gi") {
                let size_gb: i64 = mem.trim_end_matches("Gi").parse().unwrap_or(4);
                memory = Some(size_gb * 1024 * 1024 * 1024);
            } else if mem.ends_with("Mi") {
                let size_mb: i64 = mem.trim_end_matches("Mi").parse().unwrap_or(4096);
                memory = Some(size_mb * 1024 * 1024);
            }
        }

        if let Some(cpu) = &template.resources.cpu {
            // Parse CPU string like "2", "2000m"
            if cpu.ends_with("m") {
                let millicores: i64 = cpu.trim_end_matches("m").parse().unwrap_or(2000);
                nano_cpus = Some(millicores * 1_000_000);
            } else {
                let cores: i64 = cpu.parse().unwrap_or(2);
                nano_cpus = Some(cores * 1_000_000_000);
            }
        }

        // Build labels from template
        let mut labels = HashMap::new();
        labels.insert("hodei.worker".to_string(), "true".to_string());
        labels.insert("hodei.worker.id".to_string(), worker_id.to_string());
        labels.insert("hodei.worker.type".to_string(), "ephemeral".to_string());

        // Add template labels
        for (key, value) in &template.labels {
            labels.insert(key.clone(), value.clone());
        }

        let container_config = Config {
            image: Some(image.to_string()),
            env: Some(env_vars),
            labels: Some(labels),
            host_config: Some(bollard_next::service::HostConfig {
                memory,
                nano_cpus,
                auto_remove: Some(true), // Always auto-remove for ephemeral workers
                ..Default::default()
            }),
            working_dir: template.working_dir.clone(),
            ..Default::default()
        };

        let create_options = CreateContainerOptions {
            name: container_name.clone(),
            ..Default::default()
        };

        self.docker
            .create_container(Some(create_options), container_config)
            .await
            .map_err(|e| {
                ProviderError::Provider(format!(
                    "Failed to create ephemeral container '{}': {}",
                    container_name, e
                ))
            })?;

        self.docker
            .start_container::<&str>(&container_name, Some(StartContainerOptions::default()))
            .await
            .map_err(|e| {
                ProviderError::Provider(format!(
                    "Failed to start ephemeral container '{}': {}",
                    container_name, e
                ))
            })?;

        let worker = Worker::new(
            worker_id.clone(),
            format!("ephemeral-worker-{}", worker_id),
            hodei_pipelines_domain::WorkerCapabilities::new(2, 4096),
        );

        Ok(worker)
    }
}
