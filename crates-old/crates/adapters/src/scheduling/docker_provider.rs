//! Docker Provider-as-Worker Adapter
//!
//! This module provides a concrete implementation of the ProviderWorker port
//! using bollard-next. The provider IS a worker - it executes jobs directly.

use async_trait::async_trait;
use bollard_next::Docker;
use bollard_next::container::{
    Config, CreateContainerOptions, InspectContainerOptions, KillContainerOptions,
    ListContainersOptions, LogOutput, RemoveContainerOptions, StartContainerOptions,
    StopContainerOptions,
};
use bollard_next::image::CreateImageOptions;
use futures::StreamExt;
use hodei_pipelines_domain::WorkerId;
use hodei_pipelines_ports::worker_provider::{
    ExecutionContext, ExecutionStatus, JobResult, JobSpec, LogEntry, LogStreamType,
    ProviderCapabilities, ProviderConfig, ProviderError, ProviderId, ProviderType, ProviderWorker,
};
use std::collections::HashMap;
use tracing::info;

/// Docker provider-as-worker implementation
#[derive(Debug, Clone)]
pub struct DockerProvider {
    provider_id: ProviderId,
    name: String,
    docker: Docker,
    capabilities: ProviderCapabilities,
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

        let provider_id = config.provider_id;

        // Build Docker-specific capabilities
        let mut capabilities = ProviderCapabilities::new();
        capabilities.max_cpu_cores = Some(64); // Docker can use host cores
        capabilities.max_memory_gb = Some(512 * 1024); // 512GB
        capabilities.supports_gpu = true;
        capabilities.supported_runtimes = vec!["containerd".to_string(), "runc".to_string()];
        capabilities.supported_architectures = vec!["x86_64".to_string(), "aarch64".to_string()];
        capabilities.max_execution_time = None; // No limit
        capabilities.supports_ephemeral_storage = true;
        capabilities.regions = vec!["local".to_string()];
        capabilities.max_concurrent_jobs = 100; // Depends on host resources

        info!(
            "Docker provider-as-worker initialized with ID: {}",
            provider_id
        );

        Ok(Self {
            provider_id,
            name: config.name,
            docker,
            capabilities,
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

    fn create_container_name(job_id: &WorkerId) -> String {
        format!("hodei-job-{}", job_id)
    }

    fn can_handle_resources(&self, spec: &JobSpec) -> bool {
        // Docker can handle most resource requirements
        // The main constraint is available host resources
        true
    }
}

#[async_trait]
impl ProviderWorker for DockerProvider {
    fn provider_id(&self) -> ProviderId {
        self.provider_id.clone()
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Docker
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    fn can_execute(&self, spec: &JobSpec) -> bool {
        // Docker can handle most jobs if image exists
        self.can_handle_resources(spec)
    }

    async fn submit_job(&self, spec: &JobSpec) -> Result<ExecutionContext, ProviderError> {
        let container_name = Self::create_container_name(&spec.id);

        // Ensure image is available
        self.ensure_image(&spec.image).await?;

        // Convert env vars
        let mut env_vars = Vec::new();
        for (key, value) in &spec.env {
            env_vars.push(format!("{}={}", key, value));
        }

        // Add default env vars if not already present
        if !env_vars.iter().any(|e| e.starts_with("JOB_ID=")) {
            env_vars.push(format!("JOB_ID={}", spec.id));
        }

        // Convert resource requirements to Docker format
        let mut memory = None;
        let mut nano_cpus = None;

        if let Some(mem) = &spec.resources.memory {
            // Parse memory string like "4Gi", "512Mi"
            if mem.ends_with("Gi") {
                let size_gb: i64 = mem.trim_end_matches("Gi").parse().unwrap_or(4);
                memory = Some(size_gb * 1024 * 1024 * 1024);
            } else if mem.ends_with("Mi") {
                let size_mb: i64 = mem.trim_end_matches("Mi").parse().unwrap_or(4096);
                memory = Some(size_mb * 1024 * 1024);
            }
        }

        if let Some(cpu) = &spec.resources.cpu {
            // Parse CPU string like "2", "2000m"
            if cpu.ends_with("m") {
                let millicores: i64 = cpu.trim_end_matches("m").parse().unwrap_or(2000);
                nano_cpus = Some(millicores * 1_000_000);
            } else {
                let cores: i64 = cpu.parse().unwrap_or(2);
                nano_cpus = Some(cores * 1_000_000_000);
            }
        }

        // Build labels
        let mut labels = HashMap::new();
        labels.insert("hodei.job".to_string(), "true".to_string());
        labels.insert("hodei.job.id".to_string(), spec.id.to_string());
        labels.insert("hodei.provider".to_string(), "docker".to_string());

        // Add spec labels
        for (key, value) in &spec.labels {
            labels.insert(key.clone(), value.clone());
        }

        let container_config = Config {
            image: Some(spec.image.clone()),
            env: Some(env_vars),
            cmd: Some(spec.command.clone()),
            labels: Some(labels),
            host_config: Some(bollard_next::service::HostConfig {
                memory,
                nano_cpus,
                auto_remove: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };

        let create_options = CreateContainerOptions {
            name: container_name.clone(),
            ..Default::default()
        };

        // Create container
        self.docker
            .create_container(Some(create_options), container_config)
            .await
            .map_err(|e| {
                ProviderError::ExecutionFailed(format!(
                    "Failed to create container '{}': {}",
                    container_name, e
                ))
            })?;

        // Start container
        self.docker
            .start_container::<&str>(&container_name, Some(StartContainerOptions::default()))
            .await
            .map_err(|e| {
                ProviderError::ExecutionFailed(format!(
                    "Failed to start container '{}': {}",
                    container_name, e
                ))
            })?;

        // Create execution context
        let context =
            ExecutionContext::new(spec.id.clone(), self.provider_id.clone(), container_name);

        info!(
            "Job {} submitted to Docker provider {}",
            spec.id, self.provider_id
        );

        Ok(context)
    }

    async fn check_status(
        &self,
        context: &ExecutionContext,
    ) -> Result<ExecutionStatus, ProviderError> {
        let container_info = self
            .docker
            .inspect_container(
                &context.provider_execution_id,
                Some(InspectContainerOptions::default()),
            )
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    ProviderError::NotFound(format!(
                        "Container '{}' not found",
                        context.provider_execution_id
                    ))
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
                ExecutionStatus::Running
            }
            Some(bollard_next::models::ContainerStateStatusEnum::EXITED) => {
                // Check exit code
                let exit_code = container_info
                    .state
                    .as_ref()
                    .and_then(|s| s.exit_code)
                    .unwrap_or(0);

                if exit_code == 0 {
                    ExecutionStatus::Succeeded
                } else {
                    ExecutionStatus::Failed
                }
            }
            Some(bollard_next::models::ContainerStateStatusEnum::CREATED) => {
                ExecutionStatus::Queued
            }
            _ => ExecutionStatus::Running, // Default to running for unknown states
        };

        Ok(status)
    }

    async fn wait_for_completion(
        &self,
        context: &ExecutionContext,
        timeout: std::time::Duration,
    ) -> Result<JobResult, ProviderError> {
        let start = std::time::Instant::now();
        let check_interval = std::time::Duration::from_millis(500);

        loop {
            if start.elapsed() > timeout {
                // Cancel the job on timeout
                let _ = self.cancel(context).await;
                return Ok(JobResult::Timeout);
            }

            let status = self.check_status(context).await?;

            match status {
                ExecutionStatus::Succeeded => return Ok(JobResult::Success),
                ExecutionStatus::Failed => {
                    // Get exit code
                    let container_info = self
                        .docker
                        .inspect_container(
                            &context.provider_execution_id,
                            Some(InspectContainerOptions::default()),
                        )
                        .await
                        .unwrap_or_default();

                    let exit_code = container_info
                        .state
                        .as_ref()
                        .and_then(|s| s.exit_code)
                        .map(|code| code as i32)
                        .unwrap_or(-1);

                    return Ok(JobResult::Failed { exit_code });
                }
                ExecutionStatus::Cancelled => return Ok(JobResult::Cancelled),
                _ => {} // Keep waiting
            }

            tokio::time::sleep(check_interval).await;
        }
    }

    async fn get_logs(&self, context: &ExecutionContext) -> Result<Vec<LogEntry>, ProviderError> {
        let mut log_stream = self.docker.logs(
            &context.provider_execution_id,
            Some(bollard_next::container::LogsOptions::<String> {
                stdout: true,
                stderr: true,
                ..Default::default()
            }),
        );

        let mut log_entries = Vec::new();
        while let Some(log_line_result) = log_stream.next().await {
            match log_line_result {
                Ok(log_output) => {
                    let message = match log_output {
                        LogOutput::StdOut { message } => {
                            String::from_utf8_lossy(&message).to_string()
                        }
                        LogOutput::StdErr { message } => {
                            String::from_utf8_lossy(&message).to_string()
                        }
                        _ => String::new(),
                    };
                    let timestamp = chrono::Utc::now();

                    // Determine stream type
                    let stream_type = match log_output {
                        LogOutput::StdOut { .. } => LogStreamType::Stdout,
                        LogOutput::StdErr { .. } => LogStreamType::Stderr,
                        _ => LogStreamType::Stdout,
                    };

                    log_entries.push(LogEntry {
                        job_id: context.job_id.clone(),
                        timestamp,
                        message,
                        stream_type,
                    });
                }
                Err(e) => {
                    tracing::warn!("Failed to read log line: {}", e);
                }
            }
        }

        Ok(log_entries)
    }

    async fn cancel(&self, context: &ExecutionContext) -> Result<(), ProviderError> {
        self.docker
            .kill_container::<&str>(
                &context.provider_execution_id,
                Some(KillContainerOptions { signal: "SIGTERM" }),
            )
            .await
            .map_err(|e| {
                ProviderError::Provider(format!(
                    "Failed to cancel container '{}': {}",
                    context.provider_execution_id, e
                ))
            })?;

        Ok(())
    }

    async fn health_check(&self) -> Result<bool, ProviderError> {
        // Try to list containers as a health check
        let result = self
            .docker
            .list_containers(Some(ListContainersOptions::<String> {
                all: false,
                limit: Some(1),
                ..Default::default()
            }))
            .await;

        match result {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::error!("Docker health check failed: {}", e);
                Ok(false)
            }
        }
    }

    fn estimate_cost(&self, spec: &JobSpec) -> Option<f64> {
        // Docker execution is essentially free (just uses local resources)
        // Return cost per GB of memory used
        let memory_gb = if let Some(mem) = &spec.resources.memory {
            if mem.ends_with("Gi") {
                mem.trim_end_matches("Gi").parse::<f64>().unwrap_or(1.0)
            } else if mem.ends_with("Mi") {
                mem.trim_end_matches("Mi").parse::<f64>().unwrap_or(1024.0) / 1024.0
            } else {
                1.0
            }
        } else {
            1.0
        };

        Some(memory_gb * 0.001) // $0.001 per GB-hour (very cheap for local Docker)
    }
}
