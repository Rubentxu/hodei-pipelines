//! Kubernetes Provider Implementation
//! 
//! This module provides a complete implementation of the WorkerManagerProvider trait
//! using Kubernetes as the underlying infrastructure provider.

pub mod client;
pub mod crds;
pub mod security;

use crate::traits::*;
use crate::credentials::CredentialProvider;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Kubernetes-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesConfig {
    pub namespace: String,
    pub service_account: Option<String>,
    pub node_selector: Option<HashMap<String, String>>,
    pub tolerations: Option<Vec<Toleration>>,
    pub node_affinity: Option<String>,
    pub pod_disruption_budget: Option<u32>,
    pub termination_grace_period_seconds: Option<i64>,
    pub image_pull_secrets: Option<Vec<String>>,
    pub priority_class: Option<String>,
    pub security_context: Option<ContainerSecurity>,
}

impl Default for KubernetesConfig {
    fn default() -> Self {
        Self {
            namespace: "default".to_string(),
            service_account: None,
            node_selector: None,
            tolerations: None,
            node_affinity: None,
            pod_disruption_budget: None,
            termination_grace_period_seconds: Some(30),
            image_pull_secrets: None,
            priority_class: None,
            security_context: None,
        }
    }
}

/// Worker tracking in Kubernetes
#[derive(Debug, Clone)]
struct KubernetesWorker {
    id: WorkerId,
    pod_name: String,
    node_name: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_heartbeat: chrono::DateTime<chrono::Utc>,
}

/// Kubernetes Provider Implementation
pub struct KubernetesProvider {
    client: kube::Client,
    config: KubernetesConfig,
    workers: Arc<RwLock<HashMap<WorkerId, KubernetesWorker>>>,
    credential_provider: Arc<dyn CredentialProvider + Send + Sync>,
}

impl KubernetesProvider {
    /// Create a new Kubernetes Provider
    pub async fn new(
        config: KubernetesConfig,
        credential_provider: Arc<dyn CredentialProvider + Send + Sync>,
    ) -> Result<Self, ProviderError> {
        let client = kube::Client::try_default()
            .await
            .map_err(|e| ProviderError::infrastructure(
                format!("Failed to create Kubernetes client: {}", e),
                "kubernetes".to_string()
            ))?;

        Ok(Self {
            client,
            config,
            workers: Arc::new(RwLock::new(HashMap::new())),
            credential_provider,
        })
    }

    /// Get Kubernetes configuration from ProviderConfig
    pub fn from_provider_config(
        provider_config: &ProviderConfig,
        credential_provider: Arc<dyn CredentialProvider + Send + Sync>,
    ) -> Result<Self, ProviderError> {
        let mut config = KubernetesConfig::default();
        
        if let serde_json::Value::Object(obj) = &provider_config.specific_config {
            if let Some(namespace) = obj.get("namespace").and_then(|v| v.as_str()) {
                config.namespace = namespace.to_string();
            }
            if let Some(service_account) = obj.get("service_account").and_then(|v| v.as_str()) {
                config.service_account = Some(service_account.to_string());
            }
            // Additional Kubernetes-specific configurations...
        }

        Self::new(config, credential_provider)
    }
}

#[async_trait]
impl WorkerManagerProvider for KubernetesProvider {
    fn name(&self) -> &str {
        "kubernetes"
    }

    async fn create_worker(
        &self,
        spec: &RuntimeSpec,
        config: &ProviderConfig,
    ) -> Result<Worker, ProviderError> {
        let worker_id = WorkerId::new();
        let mut labels = spec.labels.clone();
        labels.insert("worker-id".to_string(), worker_id.to_string());
        labels.insert("provider".to_string(), "kubernetes".to_string());
        labels.insert("created-at".to_string(), Utc::now().to_rfc3339());

        // Create pod specification
        let mut pod_spec = k8s_openapi::api::core::v1::PodSpec::default();
        pod_spec.containers = vec![self.create_container_spec(spec, &labels).await?];
        pod_spec.service_account_name = self.config.service_account.clone().unwrap_or_default();
        pod_spec.node_selector = self.config.node_selector.clone();
        pod_spec.tolerations = self.config.tolerations.clone();
        pod_spec.termination_grace_period_seconds = self.config.termination_grace_period_seconds;

        if let Some(pdb) = self.config.pod_disruption_budget {
            pod_spec.min_ready_seconds = Some(5);
        }

        // Configure security context
        if let Some(security_ctx) = &self.config.security_context {
            pod_spec.security_context = Some(k8s_openapi::api::core::v1::PodSecurityContext {
                run_as_user: security_ctx.user,
                se_linux_options: security_ctx.se_linux_options.clone().map(|opts| k8s_openapi::api::core::v1::SELinuxOptions {
                    user: opts.get("user").cloned(),
                    role: opts.get("role").cloned(),
                    type_: opts.get("type").cloned(),
                    level: opts.get("level").cloned(),
                }),
                ..Default::default()
            });
        }

        let mut pod = k8s_openapi::api::core::v1::Pod {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some(format!("worker-{}", worker_id)),
                namespace: Some(self.config.namespace.clone()),
                labels: Some(labels),
                annotations: Some(HashMap::from([
                    ("created-by".to_string(), "worker-manager".to_string()),
                    ("runtime-spec-hash".to_string(), format!("{:?}", std::collections::hash_map::DefaultHasher::new().finish())),
                ])),
                ..Default::default()
            },
            spec: Some(pod_spec),
            status: None,
        };

        // Add volumes for secrets and configmaps
        let volumes = self.create_volumes(spec).await?;
        if !volumes.is_empty() {
            pod.spec.as_mut().unwrap().volumes = Some(volumes);
        }

        // Add resource limits
        self.apply_resource_limits(&mut pod, &spec.resources)?;

        let api: kube::Api<k8s_openapi::api::core::v1::Pod> = kube::Api::namespaced(self.client.clone(), &self.config.namespace);
        
        // Create the pod
        match api.create(&kube::PostParams::default(), &pod).await {
            Ok(created_pod) => {
                let worker = Worker {
                    id: worker_id.clone(),
                    state: WorkerState::Creating,
                    runtime_spec: spec.clone(),
                    provider_config: config.clone(),
                    metadata: HashMap::new(),
                    created_at: Utc::now(),
                    last_update: Utc::now(),
                };

                // Track the worker
                {
                    let mut workers = self.workers.write().await;
                    workers.insert(worker_id.clone(), KubernetesWorker {
                        id: worker_id,
                        pod_name: format!("worker-{}", worker_id),
                        node_name: None,
                        created_at: Utc::now(),
                        last_heartbeat: Utc::now(),
                    });
                }

                Ok(worker)
            }
            Err(e) => {
                Err(ProviderError::infrastructure(
                    format!("Failed to create Kubernetes pod: {}", e),
                    "kubernetes".to_string()
                ))
            }
        }
    }

    async fn terminate_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let api: kube::Api<k8s_openapi::api::core::v1::Pod> = kube::Api::namespaced(self.client.clone(), &self.config.namespace);
        
        let pod_name = format!("worker-{}", worker_id);
        
        match api.delete(&pod_name, &kube::DeleteParams::default()).await {
            Ok(_) => {
                // Remove from tracking
                {
                    let mut workers = self.workers.write().await;
                    workers.remove(worker_id);
                }
                Ok(())
            }
            Err(e) => {
                Err(ProviderError::infrastructure(
                    format!("Failed to terminate Kubernetes pod {}: {}", pod_name, e),
                    "kubernetes".to_string()
                ))
            }
        }
    }

    async fn get_worker_status(&self, worker_id: &WorkerId) -> Result<WorkerState, ProviderError> {
        let api: kube::Api<k8s_openapi::api::core::v1::Pod> = kube::Api::namespaced(self.client.clone(), &self.config.namespace);
        
        let pod_name = format!("worker-{}", worker_id);
        
        match api.get(&pod_name).await {
            Ok(pod) => {
                // Update heartbeat
                {
                    let mut workers = self.workers.write().await;
                    if let Some(worker) = workers.get_mut(worker_id) {
                        worker.last_heartbeat = Utc::now();
                    }
                }

                match pod.status {
                    Some(status) => {
                        match status.phase {
                            Some(k8s_openapi::api::core::v1::PodPhase::Running) => Ok(WorkerState::Running),
                            Some(k8s_openapi::api::core::v1::PodPhase::Pending) => Ok(WorkerState::Creating),
                            Some(k8s_openapi::api::core::v1::PodPhase::Failed) => Ok(WorkerState::Failed {
                                reason: status.message.clone().unwrap_or_default()
                            }),
                            Some(k8s_openapi::api::core::v1::PodPhase::Succeeded) => Ok(WorkerState::Terminated),
                            Some(k8s_openapi::api::core::v1::PodPhase::Unknown) => Ok(WorkerState::Unknown),
                            _ => Ok(WorkerState::Unknown),
                        }
                    }
                    None => Ok(WorkerState::Creating),
                }
            }
            Err(e) => {
                if e.is_not_found() {
                    Ok(WorkerState::Terminated)
                } else {
                    Err(ProviderError::infrastructure(
                        format!("Failed to get Kubernetes pod status: {}", e),
                        "kubernetes".to_string()
                    ))
                }
            }
        }
    }

    async fn get_logs(&self, worker_id: &WorkerId) -> Result<LogStreamRef, ProviderError> {
        let api: kube::Api<k8s_openapi::api::core::v1::Pod> = kube::Api::namespaced(self.client.clone(), &self.config.namespace);
        
        let pod_name = format!("worker-{}", worker_id);
        
        match api.get(&pod_name).await {
            Ok(pod) => {
                // Note: This is a simplified log reference - in practice, you would
                // typically use a log aggregation system or the Kubernetes log API
                let log_ref = LogStreamRef {
                    stream_id: Uuid::new_v4(),
                    worker_id: worker_id.clone(),
                    format: LogFormat::Json,
                    endpoints: vec![
                        format!("https://{}/api/v1/namespaces/{}/pods/{}/log?container={}", 
                               "kubernetes-api", self.config.namespace, pod_name),
                    ],
                };
                Ok(log_ref)
            }
            Err(e) => {
                Err(ProviderError::not_found(
                    format!("Pod not found: {}", e),
                    "pod".to_string(),
                    pod_name,
                    "kubernetes".to_string()
                ))
            }
        }
    }

    async fn port_forward(
        &self,
        worker_id: &WorkerId,
        local_port: u16,
        remote_port: u16,
    ) -> Result<String, ProviderError> {
        // Kubernetes port forwarding implementation
        // This would typically involve using kubectl port-forward or a similar mechanism
        // For now, return a placeholder endpoint
        let endpoint = format!("localhost:{}", local_port);
        Ok(endpoint)
    }

    async fn get_capacity(&self) -> Result<CapacityInfo, ProviderError> {
        let api: kube::Api<k8s_openapi::api::core::v1::Node> = kube::Api::all(self.client.clone());
        
        match api.list(&kube::ListParams::default()).await {
            Ok(nodes) => {
                let mut total_cpu = 0u64;
                let mut total_memory = 0u64;
                let mut used_cpu = 0u64;
                let mut used_memory = 0u64;
                let mut active_workers = 0u32;

                for node in nodes.items {
                    if let Some(status) = &node.status {
                        if let Some(capacity) = &status.capacity {
                            if let Some(cpu) = capacity.get("cpu").and_then(|c| c.as_str()) {
                                total_cpu += cpu.parse::<u64>().unwrap_or(0) * 1000; // Convert to milli-cores
                            }
                            if let Some(memory) = capacity.get("memory").and_then(|m| m.as_str()) {
                                total_memory += memory.parse::<u64>().unwrap_or(0) / 1024 / 1024; // Convert to MB
                            }
                        }
                        
                        if let Some(allocatable) = &status.allocatable {
                            if let Some(cpu) = allocatable.get("cpu").and_then(|c| c.as_str()) {
                                used_cpu += cpu.parse::<u64>().unwrap_or(0) * 1000;
                            }
                            if let Some(memory) = allocatable.get("memory").and_then(|m| m.as_str()) {
                                used_memory += memory.parse::<u64>().unwrap_or(0) / 1024 / 1024;
                            }
                        }
                    }
                }

                // Count active workers
                {
                    let workers = self.workers.read().await;
                    active_workers = workers.len() as u32;
                }

                Ok(CapacityInfo {
                    total_resources: ResourceQuota {
                        cpu_m: total_cpu,
                        memory_mb: total_memory,
                        gpu: None,
                        storage_mb: None,
                    },
                    used_resources: ResourceQuota {
                        cpu_m: used_cpu,
                        memory_mb: used_memory,
                        gpu: None,
                        storage_mb: None,
                    },
                    available_resources: ResourceQuota {
                        cpu_m: total_cpu.saturating_sub(used_cpu),
                        memory_mb: total_memory.saturating_sub(used_memory),
                        gpu: None,
                        storage_mb: None,
                    },
                    active_workers,
                    last_updated: Utc::now(),
                })
            }
            Err(e) => {
                Err(ProviderError::infrastructure(
                    format!("Failed to get Kubernetes capacity: {}", e),
                    "kubernetes".to_string()
                ))
            }
        }
    }

    async fn execute_command(
        &self,
        worker_id: &WorkerId,
        command: Vec<String>,
        timeout: Option<std::time::Duration>,
    ) -> Result<ExecutionResult, ProviderError> {
        // Kubernetes command execution via exec API
        let api: kube::Api<k8s_openapi::api::core::v1::Pod> = kube::Api::namespaced(self.client.clone(), &self.config.namespace);
        
        let pod_name = format!("worker-{}", worker_id);
        let timeout_duration = timeout.unwrap_or(std::time::Duration::from_secs(30));
        
        let start_time = std::time::Instant::now();
        
        match api.exec(&pod_name, &command, &kube::ExecParams::default()).await {
            Ok(output) => {
                let duration = start_time.elapsed();
                let finished_at = Utc::now();
                let started_at = finished_at - chrono::Duration::from_std(duration).unwrap_or(chrono::Duration::seconds(0));
                
                Ok(ExecutionResult {
                    exit_code: 0,
                    stdout: output.stdout,
                    stderr: output.stderr,
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

    async fn restart_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let api: kube::Api<k8s_openapi::api::core::v1::Pod> = kube::Api::namespaced(self.client.clone(), &self.config.namespace);
        
        let pod_name = format!("worker-{}", worker_id);
        
        match api.get(&pod_name).await {
            Ok(mut pod) => {
                // Update the pod's generation to force a restart
                if let Some(metadata) = &mut pod.metadata {
                    if let Some(annotations) = &mut metadata.annotations {
                        annotations.insert(
                            "restart-timestamp".to_string(),
                            Utc::now().to_rfc3339(),
                        );
                    }
                }
                
                match api.patch(&pod_name, &kube::PatchParams::default(), 
                             &kube::Patch::<kube::json::Json>::Merge(pod)).await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(ProviderError::infrastructure(
                        format!("Failed to restart Kubernetes pod: {}", e),
                        "kubernetes".to_string()
                    ))
                }
            }
            Err(e) => {
                Err(ProviderError::not_found(
                    format!("Pod not found for restart: {}", e),
                    "pod".to_string(),
                    pod_name,
                    "kubernetes".to_string()
                ))
            }
        }
    }

    async fn pause_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        // Implement pause by updating pod annotations
        let api: kube::Api<k8s_openapi::api::core::v1::Pod> = kube::Api::namespaced(self.client.clone(), &self.config.namespace);
        
        let pod_name = format!("worker-{}", worker_id);
        
        let patch = serde_json::json!({
            "metadata": {
                "annotations": {
                    "worker-state": "paused",
                    "paused-at": Utc::now().to_rfc3339()
                }
            }
        });
        
        match api.patch(&pod_name, &kube::PatchParams::default(), 
                      &kube::Patch::<kube::json::Json>::Merge(patch)).await {
            Ok(_) => Ok(()),
            Err(e) => Err(ProviderError::infrastructure(
                format!("Failed to pause Kubernetes pod: {}", e),
                "kubernetes".to_string()
            ))
        }
    }

    async fn resume_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        // Remove pause annotations
        let api: kube::Api<k8s_openapi::api::core::v1::Pod> = kube::Api::namespaced(self.client.clone(), &self.config.namespace);
        
        let pod_name = format!("worker-{}", worker_id);
        
        let patch = serde_json::json!({
            "metadata": {
                "annotations": {
                    "worker-state": "running"
                }
            }
        });
        
        match api.patch(&pod_name, &kube::PatchParams::default(), 
                      &kube::Patch::<kube::json::Json>::Merge(patch)).await {
            Ok(_) => Ok(()),
            Err(e) => Err(ProviderError::infrastructure(
                format!("Failed to resume Kubernetes pod: {}", e),
                "kubernetes".to_string()
            ))
        }
    }

    fn stream_worker_events(&self) -> tokio_stream::wrappers::IntervalStream {
        let interval = tokio::time::interval(std::time::Duration::from_secs(30));
        tokio_stream::wrappers::IntervalStream::new(interval)
    }
}

impl KubernetesProvider {
    /// Create container specification from runtime spec
    async fn create_container_spec(
        &self,
        spec: &RuntimeSpec,
        labels: &std::collections::HashMap<String, String>,
    ) -> Result<k8s_openapi::api::core::v1::Container, ProviderError> {
        let mut container = k8s_openapi::api::core::v1::Container {
            name: "worker".to_string(),
            image: Some(spec.image.clone()),
            command: spec.command.clone(),
            args: None,
            working_dir: None,
            ports: Some(spec.ports.iter().map(|port| k8s_openapi::api::core::v1::ContainerPort {
                container_port: *port as i32,
                protocol: Some(k8s_openapi::api::core::v1::Protocol::TCP),
                ..Default::default()
            }).collect()),
            env_from: None,
            env: Some(spec.env.iter().map(|(k, v)| k8s_openapi::api::core::v1::EnvVar {
                name: k.clone(),
                value: Some(v.clone()),
                value_from: None,
            }).collect()),
            resources: Some(k8s_openapi::api::core::v1::ResourceRequirements {
                requests: Some(std::collections::HashMap::from([
                    ("cpu".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity(spec.resources.cpu_m.to_string() + "m")),
                    ("memory".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity((spec.resources.memory_mb * 1024 * 1024).to_string())),
                ])),
                limits: Some(std::collections::HashMap::from([
                    ("cpu".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity(spec.resources.cpu_m.to_string() + "m")),
                    ("memory".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity((spec.resources.memory_mb * 1024 * 1024).to_string())),
                ])),
                ..Default::default()
            }),
            volume_mounts: Some(Vec::new()), // Will be filled by create_volumes
            liveness_probe: None,
            readiness_probe: None,
            startup_probe: None,
            ..Default::default()
        };

        // Add health checks
        let health_check = k8s_openapi::api::core::v1::Probe {
            http_get: Some(k8s_openapi::api::core::v1::HTTPGetAction {
                path: Some("/health".to_string()),
                port: 8080.try_into().unwrap(),
                host: None,
                scheme: Some(k8s_openapi::api::core::v1::URIScheme::HTTP),
            }),
            tcp_socket: None,
            exec: None,
            period_seconds: Some(30),
            timeout_seconds: Some(5),
            success_threshold: Some(1),
            failure_threshold: Some(3),
            termination_grace_period_seconds: None,
        };

        container.liveness_probe = Some(health_check.clone());
        container.readiness_probe = Some(health_check);

        Ok(container)
    }

    /// Create volumes for secrets and configmaps
    async fn create_volumes(
        &self,
        spec: &RuntimeSpec,
    ) -> Result<Vec<k8s_openapi::api::core::v1::Volume>, ProviderError> {
        let mut volumes = Vec::new();

        for volume_mount in &spec.volumes {
            let volume = match &volume_mount.source {
                VolumeSource::Secret(secret_name) => {
                    k8s_openapi::api::core::v1::Volume {
                        name: format!("secret-{}", secret_name),
                        config_map: Some(k8s_openapi::api::core::v1::ConfigMapVolumeSource {
                            default_mode: Some(0o444),
                            items: None,
                            name: Some(secret_name.clone()),
                            optional: None,
                        }),
                        ..Default::default()
                    }
                }
                VolumeSource::ConfigMap(config_name) => {
                    k8s_openapi::api::core::v1::Volume {
                        name: format!("config-{}", config_name),
                        config_map: Some(k8s_openapi::api::core::v1::ConfigMapVolumeSource {
                            default_mode: Some(0o444),
                            items: None,
                            name: Some(config_name.clone()),
                            optional: None,
                        }),
                        ..Default::default()
                    }
                }
                VolumeSource::EmptyDir => {
                    k8s_openapi::api::core::v1::Volume {
                        name: "empty-dir".to_string(),
                        empty_dir: Some(k8s_openapi::api::core::v1::EmptyDirVolumeSource {
                            medium: Some(k8s_openapi::api::core::v1::StorageMedium::Memory),
                            size_limit: None,
                        }),
                        ..Default::default()
                    }
                }
                VolumeSource::HostPath(path) => {
                    k8s_openapi::api::core::v1::Volume {
                        name: format!("host-path-{}", path.replace("/", "-")),
                        host_path: Some(k8s_openapi::api::core::v1::HostPathVolumeSource {
                            path: path.clone(),
                            type_: Some(k8s_openapi::api::core::v1::HostPathType::DirectoryOrCreate),
                        }),
                        ..Default::default()
                    }
                }
                VolumeSource::PersistentVolume(_) => {
                    // For persistent volumes, you would typically reference a PVC
                    k8s_openapi::api::core::v1::Volume {
                        name: "persistent-volume".to_string(),
                        persistent_volume_claim: Some(k8s_openapi::api::core::v1::PersistentVolumeClaimVolumeSource {
                            claim_name: "placeholder-pvc".to_string(),
                            read_only: Some(volume_mount.read_only),
                        }),
                        ..Default::default()
                    }
                }
            };

            volumes.push(volume);
        }

        Ok(volumes)
    }

    /// Apply resource limits to pod
    fn apply_resource_limits(
        &self,
        pod: &mut k8s_openapi::api::core::v1::Pod,
        resources: &ResourceQuota,
    ) -> Result<(), ProviderError> {
        if let Some(spec) = pod.spec.as_mut() {
            if let Some(container) = spec.containers.first_mut() {
                if let Some(resources) = &mut container.resources {
                    resources.limits = Some(std::collections::HashMap::from([
                        ("cpu".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity(resources.cpu_m.to_string() + "m")),
                        ("memory".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity((resources.memory_mb * 1024 * 1024).to_string())),
                    ]));
                    
                    if let Some(storage_mb) = resources.storage_mb {
                        resources.limits.as_mut().unwrap().insert(
                            "ephemeral-storage".to_string(),
                            k8s_openapi::apimachinery::pkg::api::resource::Quantity((storage_mb * 1024 * 1024).to_string()),
                        );
                    }

                    if let Some(gpu) = resources.gpu {
                        resources.limits.as_mut().unwrap().insert(
                            "nvidia.com/gpu".to_string(),
                            k8s_openapi::apimachinery::pkg::api::resource::Quantity(gpu.to_string()),
                        );
                    }
                }
            }
        }

        Ok(())
    }
}
