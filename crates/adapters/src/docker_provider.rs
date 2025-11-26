//! Docker Provider Adapter
//!
//! This module provides a concrete implementation of the WorkerProvider port
//! using bollard-next for dynamic worker provisioning.

use async_trait::async_trait;
use bollard_next::container::{
    Config, CreateContainerOptions, InspectContainerOptions, KillContainerOptions,
    ListContainersOptions, RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
};
use bollard_next::image::CreateImageOptions;
use bollard_next::{API_DEFAULT_VERSION, Docker};
use futures::StreamExt;
use hodei_core::{Worker, WorkerId};
use hodei_ports::worker_provider::{
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

        // Use custom image if provided, otherwise default HWP Agent image
        let image = config.custom_image.as_deref().unwrap_or("hwp-agent:latest");

        self.ensure_image(image).await?;

        let container_config = Config {
            image: Some(image.to_string()),
            env: Some(vec![
                "WORKER_ID=placeholder".to_string(),
                "HODEI_SERVER_GRPC_URL=http://hodei-server:50051".to_string(),
            ]),
            labels: {
                let mut labels = HashMap::new();
                labels.insert("hodei.worker".to_string(), "true".to_string());
                labels.insert("hodei.worker.id".to_string(), worker_id.to_string());
                Some(labels)
            },
            host_config: Some(bollard_next::service::HostConfig {
                memory: Some(4 * 1024 * 1024 * 1024),
                nano_cpus: Some(2 * 1_000_000_000),
                auto_remove: Some(true),
                ..Default::default()
            }),
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
            hodei_core::WorkerCapabilities::new(2, 4096),
        );

        Ok(worker)
    }

    async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<hodei_core::WorkerStatus, ProviderError> {
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
                hodei_core::WorkerStatus::new(
                    worker_id.clone(),
                    hodei_core::WorkerStatus::IDLE.to_string(),
                )
            }
            _ => hodei_core::WorkerStatus::new(
                worker_id.clone(),
                hodei_core::WorkerStatus::OFFLINE.to_string(),
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
            if let Some(labels) = container.labels {
                if let Some(worker_id_str) = labels.get("hodei.worker.id") {
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
        }

        Ok(worker_ids)
    }
}
