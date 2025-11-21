//! Docker Provider Implementation
//! 
//! This module provides a complete implementation of the WorkerManagerProvider trait
//! using Docker as the underlying infrastructure provider.

use crate::traits::*;
use crate::credentials::CredentialProvider;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Docker-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    pub host: Option<String>,
    pub network_mode: String,
    pub registry_auth: Option<String>,
    pub cpu_shares: Option<u64>,
    pub memory_limit: Option<u64>,
    pub ulimits: Option<Vec<String>>,
    pub log_driver: Option<String>,
    pub log_options: Option<HashMap<String, String>>,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            host: None,
            network_mode: "bridge".to_string(),
            registry_auth: None,
            cpu_shares: Some(1024),
            memory_limit: Some(1024 * 1024 * 1024), // 1GB
            ulimits: None,
            log_driver: Some("json-file".to_string()),
            log_options: Some(HashMap::from([
                ("max-size".to_string(), "10m".to_string()),
                ("max-file".to_string(), "3".to_string()),
            ])),
        }
    }
}

/// Container tracking information
#[derive(Debug, Clone)]
struct DockerWorker {
    id: WorkerId,
    container_id: String,
    network_name: Option<String>,
    volume_mounts: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_health_check: chrono::DateTime<chrono::Utc>,
}

/// Docker Provider Implementation
pub struct DockerProvider {
    client: docker::Docker,
    config: DockerConfig,
    workers: Arc<RwLock<HashMap<WorkerId, DockerWorker>>>,
    credential_provider: Arc<dyn CredentialProvider + Send + Sync>,
}

impl DockerProvider {
    /// Create a new Docker Provider
    pub async fn new(
        config: DockerConfig,
        credential_provider: Arc<dyn CredentialProvider + Send + Sync>,
    ) -> Result<Self, ProviderError> {
        let client = docker::Docker::connect_with_unix_socket(&config.host.unwrap_or_else(|| "/var/run/docker.sock".to_string()), 115, docker::Docker::default_timeout())
            .map_err(|e| ProviderError::infrastructure(
                format!("Failed to create Docker client: {}", e),
                "docker".to_string()
            ))?;

        Ok(Self {
            client,
            config,
            workers: Arc::new(RwLock::new(HashMap::new())),
            credential_provider,
        })
    }

    /// Get Docker configuration from ProviderConfig
    pub fn from_provider_config(
        provider_config: &ProviderConfig,
        credential_provider: Arc<dyn CredentialProvider + Send + Sync>,
    ) -> Result<Self, ProviderError> {
        let mut config = DockerConfig::default();
        
        if let serde_json::Value::Object(obj) = &provider_config.specific_config {
            if let Some(host) = obj.get("host").and_then(|v| v.as_str()) {
                config.host = Some(host.to_string());
            }
            if let Some(network_mode) = obj.get("network_mode").and_then(|v| v.as_str()) {
                config.network_mode = network_mode.to_string();
            }
            if let Some(registry_auth) = obj.get("registry_auth").and_then(|v| v.as_str()) {
                config.registry_auth = Some(registry_auth.to_string());
            }
        }

        Self::new(config, credential_provider)
    }

    /// Create Docker network for isolation
    async fn create_network(&self, worker_id: &WorkerId) -> Result<String, ProviderError> {
        let network_name = format!("worker-network-{}", worker_id);
        
        let network_config = docker::models::NetworkCreateRequest::builder()
            .name(&network_name)
            .driver("bridge")
            .internal(false)
            .ipam(docker::models::IPAMConfig::builder().build())
            .options({
                let mut options = HashMap::new();
                options.insert("com.docker.network.bridge.enable_icc".to_string(), "true".to_string());
                options.insert("com.docker.network.bridge.enable_ip_masquerade".to_string(), "true".to_string());
                options
            })
            .build();

        match self.client.create_network(network_config).await {
            Ok(_) => Ok(network_name),
            Err(e) => Err(ProviderError::infrastructure(
                format!("Failed to create Docker network: {}", e),
                "docker".to_string()
            ))
        }
    }

    /// Get container logs
    async fn get_container_logs(&self, container_id: &str) -> Result<String, ProviderError> {
        let mut logs = String::new();
        
        match self.client.logs(container_id, &docker::Docker::default_timeout()).await {
            Ok(log_stream) => {
                for result in log_stream {
                    match result {
                        Ok(log) => {
                            match log {
                                docker::LogOutput::StdOut { message } => {
                                    logs.push_str(&String::from_utf8_lossy(&message));
                                }
                                docker::LogOutput::StdErr { message } => {
                                    logs.push_str(&format!("STDERR: {}", String::from_utf8_lossy(&message)));
                                }
                                docker::LogOutput::StdIn { message } => {
                                    logs.push_str(&format!("STDIN: {}", String::from_utf8_lossy(&message)));
                                }
                                docker::LogOutput::Unknown { data } => {
                                    logs.push_str(&format!("UNKNOWN: {}", String::from_utf8_lossy(&data)));
                                }
                            }
                        }
                        Err(e) => return Err(ProviderError::infrastructure(
                            format!("Failed to read log: {}", e),
                            "docker".to_string()
                        ))
                    }
                }
                Ok(logs)
            }
            Err(e) => Err(ProviderError::infrastructure(
                format!("Failed to get container logs: {}", e),
                "docker".to_string()
            ))
        }
    }

    /// Create volume mount configuration
    async fn create_volume_mounts(
        &self,
        spec: &RuntimeSpec,
    ) -> Result<Vec<docker::models::Mount>, ProviderError> {
        let mut mounts = Vec::new();

        for volume_mount in &spec.volumes {
            let mount = match &volume_mount.source {
                VolumeSource::EmptyDir => {
                    docker::models::Mount::builder()
                        .type_(docker::models::MountTypeEnum::TMPFS)
                        .source(None)
                        .target(&volume_mount.mount_path)
                        .tmpfs_options(Some(docker::models::TmpfsOptions {
                            size_bytes: None,
                            mode: Some(0o755),
                        }))
                        .build()
                }
                VolumeSource::HostPath(path) => {
                    docker::models::Mount::builder()
                        .type_(docker::models::MountTypeEnum::BIND)
                        .source(Some(path.clone()))
                        .target(&volume_mount.mount_path)
                        .read_only(Some(volume_mount.read_only))
                        .build()
                }
                VolumeSource::Secret(secret_name) => {
                    docker::models::Mount::builder()
                        .type_(docker::models::MountTypeEnum::VOLUME)
                        .source(Some(format!("secret-{}", secret_name)))
                        .target(&volume_mount.mount_path)
                        .read_only(Some(volume_mount.read_only))
                        .build()
                }
                VolumeSource::ConfigMap(config_name) => {
                    docker::models::Mount::builder()
                        .type_(docker::models::MountTypeEnum::VOLUME)
                        .source(Some(format!("config-{}", config_name)))
                        .target(&volume_mount.mount_path)
                        .read_only(Some(volume_mount.read_only))
                        .build()
                }
                VolumeSource::PersistentVolume(pv_name) => {
                    docker::models::Mount::builder()
                        .type_(docker::models::MountTypeEnum::VOLUME)
                        .source(Some(pv_name.clone()))
                        .target(&volume_mount.mount_path)
                        .read_only(Some(volume_mount.read_only))
                        .build()
                }
            };
            mounts.push(mount);
        }

        Ok(mounts)
    }

    /// Create health check configuration
    async fn create_health_check(&self, spec: &RuntimeSpec) -> Option<docker::models::HealthConfig> {
        Some(docker::models::HealthConfig {
            test: Some(vec!["CMD".to_string(), "curl".to_string(), "-f".to_string(), "http://localhost:8080/health".to_string()]),
            interval: Some(30_000_000_000), // 30 seconds in nanoseconds
            timeout: Some(5_000_000_000),  // 5 seconds
            retries: Some(3),
            start_period: Some(0),
        })
    }
}

#[async_trait]
impl WorkerManagerProvider for DockerProvider {
    fn name(&self) -> &str {
        "docker"
    }

    async fn create_worker(
        &self,
        spec: &RuntimeSpec,
        config: &ProviderConfig,
    ) -> Result<Worker, ProviderError> {
        let worker_id = WorkerId::new();
        
        // Create network for isolation
        let network_name = self.create_network(&worker_id).await?;
        
        // Create volume mounts
        let mounts = self.create_volume_mounts(spec).await?;

        // Prepare container creation options
        let mut create_options = docker::models::CreateContainerOptions {
            name: Some(format!("worker-{}", worker_id)),
            ..Default::default()
        };

        // Prepare host configuration
        let mut host_config = docker::models::HostConfig::builder()
            .network_mode(Some(network_name.clone()))
            .memory(self.config.memory_limit)
            .cpu_shares(self.config.cpu_shares)
            .restart_policy(Some(docker::models::RestartPolicy {
                name: Some(docker::models::RestartPolicyNameEnum::ALWAYS),
                maximum_retry_count: None,
            }))
            .build();

        // Add volume mounts
        host_config.mounts = Some(mounts);

        // Add log configuration
        if let Some(log_driver) = &self.config.log_driver {
            let mut log_config = docker::models::LogConfig {
                type_: Some(log_driver.clone()),
                config: self.config.log_options.clone(),
            };
            host_config.log_config = Some(log_config);
        }

        // Add environment variables
        let mut env_vars = Vec::new();
        for (key, value) in &spec.env {
            env_vars.push(format!("{}={}", key, value));
        }

        // Add secrets as environment variables if not using volume mounts
        for secret_ref in &spec.secret_refs {
            match self.credential_provider.get_secret(secret_ref, None).await {
                Ok(secret) => {
                    for (key, value) in &secret.values {
                        env_vars.push(format!("{}_{}={}", secret_ref.to_uppercase(), key, value));
                    }
                }
                Err(e) => {
                    return Err(ProviderError::credentials(
                        format!("Failed to get secret {}: {}", secret_ref, e),
                        "docker".to_string()
                    ));
                }
            }
        }

        // Create container
        let container_config = docker::models::ContainerCreateBody {
            image: spec.image.clone(),
            cmd: spec.command.clone(),
            env: Some(env_vars),
            host_config: Some(host_config),
            healthcheck: self.create_health_check(spec).await,
            ..Default::default()
        };

        match self.client.create_container(&create_options, &container_config).await {
            Ok(container_info) => {
                // Start the container
                match self.client.start_container(&container_info.id).await {
                    Ok(_) => {
                        let worker = Worker {
                            id: worker_id.clone(),
                            state: WorkerState::Running,
                            runtime_spec: spec.clone(),
                            provider_config: config.clone(),
                            metadata: HashMap::new(),
                            created_at: Utc::now(),
                            last_update: Utc::now(),
                        };

                        // Track the worker
                        {
                            let mut workers = self.workers.write().await;
                            workers.insert(worker_id.clone(), DockerWorker {
                                id: worker_id,
                                container_id: container_info.id.clone(),
                                network_name: Some(network_name),
                                volume_mounts: Vec::new(),
                                created_at: Utc::now(),
                                last_health_check: Utc::now(),
                            });
                        }

                        Ok(worker)
                    }
                    Err(e) => {
                        // Clean up container if start failed
                        let _ = self.client.remove_container(&container_info.id, &docker::Docker::default_timeout()).await;
                        Err(ProviderError::infrastructure(
                            format!("Failed to start Docker container: {}", e),
                            "docker".to_string()
                        ))
                    }
                }
            }
            Err(e) => {
                // Clean up network if container creation failed
                let _ = self.client.remove_network(&network_name).await;
                Err(ProviderError::infrastructure(
                    format!("Failed to create Docker container: {}", e),
                    "docker".to_string()
                ))
            }
        }
    }

    async fn terminate_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let workers = self.workers.read().await;
        if let Some(worker) = workers.get(worker_id) {
            let container_id = worker.container_id.clone();
            let network_name = worker.network_name.clone();
            drop(workers);

            // Stop and remove container
            let remove_options = docker::models::RemoveContainerOptions {
                force: true,
                v: false,
                link: false,
            };

            match self.client.stop_container(&container_id, &docker::Docker::default_timeout()).await {
                Ok(_) => {
                    match self.client.remove_container(&container_id, &remove_options).await {
                        Ok(_) => {
                            // Remove network if it exists
                            if let Some(network) = network_name {
                                let _ = self.client.remove_network(&network).await;
                            }

                            // Remove from tracking
                            {
                                let mut workers = self.workers.write().await;
                                workers.remove(worker_id);
                            }

                            Ok(())
                        }
                        Err(e) => Err(ProviderError::infrastructure(
                            format!("Failed to remove Docker container: {}", e),
                            "docker".to_string()
                        ))
                    }
                }
                Err(e) => Err(ProviderError::infrastructure(
                    format!("Failed to stop Docker container: {}", e),
                    "docker".to_string()
                ))
            }
        } else {
            Err(ProviderError::not_found(
                "Worker not found".to_string(),
                "worker".to_string(),
                worker_id.to_string(),
                "docker".to_string()
            ))
        }
    }

    async fn get_worker_status(&self, worker_id: &WorkerId) -> Result<WorkerState, ProviderError> {
        let workers = self.workers.read().await;
        if let Some(worker) = workers.get(worker_id) {
            let container_id = worker.container_id.clone();
            drop(workers);

            match self.client.inspect_container(&container_id).await {
                Ok(inspect_info) => {
                    // Update last health check
                    {
                        let mut workers = self.workers.write().await;
                        if let Some(w) = workers.get_mut(worker_id) {
                            w.last_health_check = Utc::now();
                        }
                    }

                    match inspect_info.state {
                        Some(state) => {
                            match state.status {
                                Some(docker::models::ContainerStateStatusEnum::RUNNING) => {
                                    // Check if container has a health check and if it's healthy
                                    if let Some(health) = &inspect_info.health {
                                        match health.status {
                                            Some(docker::models::HealthStatus::HEALTHY) => Ok(WorkerState::Running),
                                            Some(docker::models::HealthStatus::UNHEALTHY) => Ok(WorkerState::Failed {
                                                reason: "Health check failed".to_string()
                                            }),
                                            _ => Ok(WorkerState::Running),
                                        }
                                    } else {
                                        Ok(WorkerState::Running)
                                    }
                                }
                                Some(docker::models::ContainerStateStatusEnum::EXITED) => {
                                    Ok(WorkerState::Terminated)
                                }
                                Some(docker::models::ContainerStateStatusEnum::PAUSED) => {
                                    Ok(WorkerState::Paused)
                                }
                                Some(docker::models::ContainerStateStatusEnum::DEAD) => {
                                    Ok(WorkerState::Failed {
                                        reason: "Container is dead".to_string()
                                    })
                                }
                                _ => Ok(WorkerState::Unknown),
                            }
                        }
                        None => Ok(WorkerState::Unknown),
                    }
                }
                Err(e) => {
                    if e.is_not_found() {
                        Ok(WorkerState::Terminated)
                    } else {
                        Err(ProviderError::infrastructure(
                            format!("Failed to inspect Docker container: {}", e),
                            "docker".to_string()
                        ))
                    }
                }
            }
        } else {
            Err(ProviderError::not_found(
                "Worker not found".to_string(),
                "worker".to_string(),
                worker_id.to_string(),
                "docker".to_string()
            ))
        }
    }

    async fn get_logs(&self, worker_id: &WorkerId) -> Result<LogStreamRef, ProviderError> {
        let workers = self.workers.read().await;
        if let Some(worker) = workers.get(worker_id) {
            let container_id = worker.container_id.clone();
            drop(workers);

            // Get logs through Docker API
            let logs = self.get_container_logs(&container_id).await?;

            Ok(LogStreamRef {
                stream_id: Uuid::new_v4(),
                worker_id: worker_id.clone(),
                format: LogFormat::Json,
                endpoints: vec![
                    format!("docker://logs/{}", container_id),
                ],
            })
        } else {
            Err(ProviderError::not_found(
                "Worker not found".to_string(),
                "worker".to_string(),
                worker_id.to_string(),
                "docker".to_string()
            ))
        }
    }

    async fn port_forward(
        &self,
        worker_id: &WorkerId,
        local_port: u16,
        remote_port: u16,
    ) -> Result<String, ProviderError> {
        let workers = self.workers.read().await;
        if let Some(worker) = workers.get(worker_id) {
            let container_id = worker.container_id.clone();
            let network_name = worker.network_name.clone().unwrap_or_default();
            drop(workers);

            // Docker port forwarding through exec or attach
            let endpoint = format!("localhost:{} (container:{})", local_port, container_id);
            
            // In practice, you would use Docker's port forwarding mechanisms
            // This is a simplified implementation
            Ok(endpoint)
        } else {
            Err(ProviderError::not_found(
                "Worker not found".to_string(),
                "worker".to_string(),
                worker_id.to_string(),
                "docker".to_string()
            ))
        }
    }

    async fn get_capacity(&self) -> Result<CapacityInfo, ProviderError> {
        match self.client.info().await {
            Ok(info) => {
                let active_workers = {
                    let workers = self.workers.read().await;
                    workers.len() as u32
                };

                Ok(CapacityInfo {
                    total_resources: ResourceQuota {
                        cpu_m: (info.cpu_count.unwrap_or(1) * 1000) as u64,
                        memory_mb: (info.mem_total.unwrap_or(1024 * 1024 * 1024) / 1024 / 1024) as u64,
                        gpu: None,
                        storage_mb: None,
                    },
                    used_resources: ResourceQuota {
                        cpu_m: active_workers as u64 * 500, // Estimate
                        memory_mb: active_workers as u64 * 256, // Estimate
                        gpu: None,
                        storage_mb: None,
                    },
                    available_resources: ResourceQuota {
                        cpu_m: ((info.cpu_count.unwrap_or(1) * 1000) as u64).saturating_sub(active_workers as u64 * 500),
                        memory_mb: ((info.mem_total.unwrap_or(1024 * 1024 * 1024) / 1024 / 1024) as u64).saturating_sub(active_workers as u64 * 256),
                        gpu: None,
                        storage_mb: None,
                    },
                    active_workers,
                    last_updated: Utc::now(),
                })
            }
            Err(e) => Err(ProviderError::infrastructure(
                format!("Failed to get Docker capacity: {}", e),
                "docker".to_string()
            ))
        }
    }

    async fn execute_command(
        &self,
        worker_id: &WorkerId,
        command: Vec<String>,
        timeout: Option<std::time::Duration>,
    ) -> Result<ExecutionResult, ProviderError> {
        let workers = self.workers.read().await;
        if let Some(worker) = workers.get(worker_id) {
            let container_id = worker.container_id.clone();
            drop(workers);

            let start_time = std::time::Instant::now();
            
            // Execute command in container
            let exec_config = docker::models::ExecCreateBody {
                cmd: Some(command),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                attach_stdin: Some(false),
                working_dir: None,
                user: None,
                env: None,
            };

            match self.client.exec_create(&container_id, &exec_config).await {
                Ok(exec_info) => {
                    match self.client.exec_start(&exec_info.id, &docker::Docker::default_timeout()).await {
                        Ok(output) => {
                            let duration = start_time.elapsed();
                            let finished_at = Utc::now();
                            let started_at = finished_at - chrono::Duration::from_std(duration).unwrap_or(chrono::Duration::seconds(0));

                            let mut stdout = String::new();
                            let mut stderr = String::new();

                            for result in output {
                                match result {
                                    Ok(output) => {
                                        match output {
                                            docker::LogOutput::StdOut { message } => {
                                                stdout.push_str(&String::from_utf8_lossy(&message));
                                            }
                                            docker::LogOutput::StdErr { message } => {
                                                stderr.push_str(&String::from_utf8_lossy(&message));
                                            }
                                            _ => {}
                                        }
                                    }
                                    Err(e) => {
                                        stderr.push_str(&format!("Error reading output: {}", e));
                                    }
                                }
                            }

                            Ok(ExecutionResult {
                                exit_code: 0,
                                stdout,
                                stderr,
                                duration,
                                started_at,
                                finished_at,
                            })
                        }
                        Err(e) => {
                            let duration = start_time.elapsed();
                            let finished_at = Utc::now();
                            let started_at = finished_at - chrono::Duration::from_std(duration).unwrap_or(chrono::Duration::seconds(0));

                            Ok(ExecutionResult {
                                exit_code: -1,
                                stdout: String::new(),
                                stderr: format!("Command execution failed: {}", e),
                                duration,
                                started_at,
                                finished_at,
                            })
                        }
                    }
                }
                Err(e) => {
                    let duration = start_time.elapsed();
                    let finished_at = Utc::now();
                    let started_at = finished_at - chrono::Duration::from_std(duration).unwrap_or(chrono::Duration::seconds(0));

                    Ok(ExecutionResult {
                        exit_code: -1,
                        stdout: String::new(),
                        stderr: format!("Failed to create exec: {}", e),
                        duration,
                        started_at,
                        finished_at,
                    })
                }
            }
        } else {
            Err(ProviderError::not_found(
                "Worker not found".to_string(),
                "worker".to_string(),
                worker_id.to_string(),
                "docker".to_string()
            ))
        }
    }

    async fn restart_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let workers = self.workers.read().await;
        if let Some(worker) = workers.get(worker_id) {
            let container_id = worker.container_id.clone();
            drop(workers);

            // Restart container
            match self.client.restart_container(&container_id, &docker::Docker::default_timeout()).await {
                Ok(_) => Ok(()),
                Err(e) => Err(ProviderError::infrastructure(
                    format!("Failed to restart Docker container: {}", e),
                    "docker".to_string()
                ))
            }
        } else {
            Err(ProviderError::not_found(
                "Worker not found".to_string(),
                "worker".to_string(),
                worker_id.to_string(),
                "docker".to_string()
            ))
        }
    }

    async fn pause_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let workers = self.workers.read().await;
        if let Some(worker) = workers.get(worker_id) {
            let container_id = worker.container_id.clone();
            drop(workers);

            // Pause container
            match self.client.pause_container(&container_id).await {
                Ok(_) => Ok(()),
                Err(e) => Err(ProviderError::infrastructure(
                    format!("Failed to pause Docker container: {}", e),
                    "docker".to_string()
                ))
            }
        } else {
            Err(ProviderError::not_found(
                "Worker not found".to_string(),
                "worker".to_string(),
                worker_id.to_string(),
                "docker".to_string()
            ))
        }
    }

    async fn resume_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let workers = self.workers.read().await;
        if let Some(worker) = workers.get(worker_id) {
            let container_id = worker.container_id.clone();
            drop(workers);

            // Unpause container
            match self.client.unpause_container(&container_id).await {
                Ok(_) => Ok(()),
                Err(e) => Err(ProviderError::infrastructure(
                    format!("Failed to resume Docker container: {}", e),
                    "docker".to_string()
                ))
            }
        } else {
            Err(ProviderError::not_found(
                "Worker not found".to_string(),
                "worker".to_string(),
                worker_id.to_string(),
                "docker".to_string()
            ))
        }
    }

    fn stream_worker_events(&self) -> tokio_stream::wrappers::IntervalStream {
        let interval = tokio::time::interval(std::time::Duration::from_secs(30));
        tokio_stream::wrappers::IntervalStream::new(interval)
    }
}

impl Drop for DockerProvider {
    fn drop(&mut self) {
        // Clean up any remaining networks on drop
        // In practice, you might want to implement a proper cleanup mechanism
    }
}
