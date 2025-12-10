//! Kubernetes Provider-as-Worker Adapter
//!
//! This module provides a concrete implementation of the ProviderWorker port
//! using the Kubernetes REST API. The provider IS a worker - it executes jobs directly.

use async_trait::async_trait;
use hodei_pipelines_domain::WorkerId;
use hodei_pipelines_ports::worker_provider::{
    ExecutionContext, ExecutionStatus, JobResult, JobSpec, LogEntry, LogStreamType,
    ProviderCapabilities, ProviderConfig, ProviderError, ProviderId, ProviderType, ProviderWorker,
};
use reqwest::{Client as HttpClient, Response};
use serde_json::{Value, json};
use tracing::info;

/// Kubernetes provider-as-worker implementation using REST API
#[derive(Debug, Clone)]
pub struct KubernetesProvider {
    provider_id: ProviderId,
    name: String,
    client: HttpClient,
    namespace: String,
    server_url: String,
    capabilities: ProviderCapabilities,
}

impl KubernetesProvider {
    pub async fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        if config.provider_type != ProviderType::Kubernetes {
            return Err(ProviderError::InvalidConfiguration(
                "Expected Kubernetes provider configuration".to_string(),
            ));
        }

        // Professional Kubernetes config loading
        let (server_url, ca_cert, token) = Self::load_kube_config()
            .await
            .map_err(|e| ProviderError::Provider(format!("Failed to load kube config: {}", e)))?;

        // Build HTTP client with proper TLS and auth
        let mut client_builder = HttpClient::builder().danger_accept_invalid_certs(true); // For development, in production use proper certs

        if let Some(ca) = ca_cert {
            let cert = reqwest::Certificate::from_pem(&ca)
                .map_err(|e| ProviderError::Provider(format!("Failed to parse CA cert: {}", e)))?;
            client_builder = client_builder.add_root_certificate(cert);
        }

        let client = client_builder
            .build()
            .map_err(|e| ProviderError::Provider(format!("Failed to build HTTP client: {}", e)))?;

        let namespace = config.namespace.unwrap_or_else(|| "default".to_string());
        let provider_id = config.provider_id;

        // Build Kubernetes-specific capabilities
        let mut capabilities = ProviderCapabilities::new();
        capabilities.max_cpu_cores = Some(256);
        capabilities.max_memory_gb = Some(1024 * 1024); // 1TB
        capabilities.supports_gpu = true;
        capabilities.supported_runtimes = vec!["containerd".to_string(), "cri-o".to_string()];
        capabilities.supported_architectures = vec!["x86_64".to_string(), "aarch64".to_string()];
        capabilities.max_execution_time = None; // Kubernetes Jobs can run indefinitely
        capabilities.supports_ephemeral_storage = true;
        capabilities.regions = vec!["cluster".to_string()];
        capabilities.max_concurrent_jobs = 1000; // Depends on cluster size

        info!(
            "Kubernetes provider-as-worker initialized with ID: {} in namespace: {}",
            provider_id, namespace
        );

        Ok(Self {
            provider_id,
            name: config.name,
            client,
            namespace,
            server_url,
            capabilities,
        })
    }

    async fn load_kube_config() -> Result<(String, Option<Vec<u8>>, Option<String>), ProviderError>
    {
        // Try in-cluster config first (production standard)
        if std::env::var("KUBERNETES_SERVICE_HOST").is_ok() {
            let host = std::env::var("KUBERNETES_SERVICE_HOST").map_err(|_| {
                ProviderError::Provider("KUBERNETES_SERVICE_HOST not set".to_string())
            })?;
            let port =
                std::env::var("KUBERNETES_SERVICE_PORT").unwrap_or_else(|_| "443".to_string());
            let server_url = format!("https://{}:{}", host, port);

            let token =
                std::fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/token")
                    .ok()
                    .map(|t| t.trim().to_string());
            let ca_cert =
                std::fs::read("/var/run/secrets/kubernetes.io/serviceaccount/ca.crt").ok();

            return Ok((server_url, ca_cert, token));
        }

        // Fall back to kubeconfig file (development standard)
        let kubeconfig =
            std::env::var("KUBECONFIG").unwrap_or_else(|_| "~/.kube/config".to_string());
        let kubeconfig_path = shellexpand::tilde(&kubeconfig).into_owned();

        if std::path::Path::new(&kubeconfig_path).exists() {
            let content = std::fs::read_to_string(&kubeconfig_path).map_err(|e| {
                ProviderError::Provider(format!("Failed to read kubeconfig: {}", e))
            })?;

            let config: Value = serde_json::from_str(&content)
                .map_err(|e| ProviderError::Provider(format!("Invalid kubeconfig: {}", e)))?;

            // Extract server URL
            let server_url = config
                .get("clusters")
                .and_then(|c| c.as_array())
                .and_then(|c| c.first())
                .and_then(|c| c.get("cluster"))
                .and_then(|c| c.get("server"))
                .and_then(|s| s.as_str())
                .ok_or_else(|| {
                    ProviderError::Provider("Invalid kubeconfig: no server".to_string())
                })?
                .to_string();

            // Try to extract certificate
            let ca_cert = config
                .get("clusters")
                .and_then(|c| c.as_array())
                .and_then(|c| c.first())
                .and_then(|c| c.get("cluster"))
                .and_then(|c| c.get("certificate-authority"))
                .and_then(|s| s.as_str())
                .and_then(|path| std::fs::read(path).ok());

            // Try to extract token
            let token = config
                .get("users")
                .and_then(|u| u.as_array())
                .and_then(|u| u.first())
                .and_then(|u| u.get("user"))
                .and_then(|u| u.get("token"))
                .and_then(|t| t.as_str())
                .map(|t| t.to_string());

            return Ok((server_url, ca_cert, token));
        }

        Err(ProviderError::Provider(
            "Neither in-cluster config nor kubeconfig found".to_string(),
        ))
    }

    fn create_job_name(job_id: &WorkerId) -> String {
        format!("hodei-job-{}", job_id)
    }

    fn create_job_manifest(&self, spec: &JobSpec, namespace: &str) -> Result<Value, ProviderError> {
        // Build environment variables
        let mut env_vars = Vec::new();
        for (key, value) in &spec.env {
            env_vars.push(json!({
                "name": key,
                "value": value
            }));
        }

        // Add default env vars if not already present
        if !env_vars.iter().any(|e| {
            e.as_object()
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
                == Some("JOB_ID")
        }) {
            env_vars.push(json!({
                "name": "JOB_ID",
                "value": spec.id.to_string()
            }));
        }

        // Build resource requirements
        let mut requests = json!({});
        let mut limits = json!({});

        if let Some(cpu) = &spec.resources.cpu {
            requests["cpu"] = json!(cpu);
            limits["cpu"] = json!(cpu);
        }
        if let Some(memory) = &spec.resources.memory {
            requests["memory"] = json!(memory);
            limits["memory"] = json!(memory);
        }
        if let Some(ephemeral_storage) = &spec.resources.ephemeral_storage {
            requests["ephemeral-storage"] = json!(ephemeral_storage);
            limits["ephemeral-storage"] = json!(ephemeral_storage);
        }

        // Build labels
        let mut labels = json!({
            "hodei.job": "true",
            "hodei.job.id": spec.id.to_string(),
            "hodei.provider": "kubernetes"
        });

        // Add spec labels
        if let Some(labels_obj) = labels.as_object_mut() {
            for (key, value) in &spec.labels {
                labels_obj.insert(key.clone(), json!(value));
            }
        }

        let job_name = Self::create_job_name(&spec.id);

        let manifest = json!({
            "apiVersion": "batch/v1",
            "kind": "Job",
            "metadata": {
                "name": job_name,
                "namespace": namespace,
                "labels": labels
            },
            "spec": {
                "template": {
                    "metadata": {
                        "labels": labels
                    },
                    "spec": {
                        "restartPolicy": "Never",
                        "containers": [{
                            "name": "job",
                            "image": spec.image,
                            "command": spec.command,
                            "env": env_vars,
                            "resources": {
                                "requests": requests,
                                "limits": limits
                            }
                        }]
                    }
                },
                "backoffLimit": 0
            }
        });

        Ok(manifest)
    }

    async fn k8s_get(&self, path: &str) -> Result<Response, ProviderError> {
        let url = format!("{}{}", self.server_url, path);
        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| ProviderError::Provider(format!("K8s GET {} failed: {}", path, e)))
    }

    async fn k8s_post(&self, path: &str, body: &Value) -> Result<Response, ProviderError> {
        let url = format!("{}{}", self.server_url, path);
        self.client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| ProviderError::Provider(format!("K8s POST {} failed: {}", path, e)))
    }

    async fn k8s_delete(
        &self,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Response, ProviderError> {
        let url = format!("{}{}", self.server_url, path);
        let request = self.client.delete(&url);

        let request = if let Some(b) = body {
            request.json(b)
        } else {
            request
        };

        request
            .send()
            .await
            .map_err(|e| ProviderError::Provider(format!("K8s DELETE {} failed: {}", path, e)))
    }
}

#[async_trait]
impl ProviderWorker for KubernetesProvider {
    fn provider_id(&self) -> ProviderId {
        self.provider_id.clone()
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Kubernetes
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    fn can_execute(&self, spec: &JobSpec) -> bool {
        // Kubernetes can handle most workloads
        true
    }

    async fn submit_job(&self, spec: &JobSpec) -> Result<ExecutionContext, ProviderError> {
        let job_name = Self::create_job_name(&spec.id);
        let manifest = self
            .create_job_manifest(spec, &self.namespace)
            .map_err(|e| ProviderError::Provider(e.to_string()))?;

        // Create the job
        let path = format!("/apis/batch/v1/namespaces/{}/jobs", self.namespace);
        let response = self.k8s_post(&path, &manifest).await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::ExecutionFailed(format!(
                "Failed to create job '{}': {}",
                job_name, error_text
            )));
        }

        // Create execution context
        let context = ExecutionContext::new(spec.id.clone(), self.provider_id.clone(), job_name);

        info!(
            "Job {} submitted to Kubernetes provider {}",
            spec.id, self.provider_id
        );

        Ok(context)
    }

    async fn check_status(
        &self,
        context: &ExecutionContext,
    ) -> Result<ExecutionStatus, ProviderError> {
        let path = format!(
            "/apis/batch/v1/namespaces/{}/jobs/{}",
            self.namespace, context.provider_execution_id
        );

        let response = self.k8s_get(&path).await?;

        if response.status() == 404 {
            return Err(ProviderError::NotFound(format!(
                "Job '{}' not found",
                context.provider_execution_id
            )));
        }

        if !response.status().is_success() {
            return Err(ProviderError::Provider(format!(
                "Failed to get job '{}': HTTP {}",
                context.provider_execution_id,
                response.status()
            )));
        }

        let job: Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Provider(format!("Failed to parse job response: {}", e)))?;

        let status = if let Some(job_status) = job.get("status") {
            if job_status.get("succeeded").and_then(|s| s.as_u64()) == Some(1) {
                ExecutionStatus::Succeeded
            } else if job_status.get("failed").and_then(|s| s.as_u64()) == Some(1) {
                ExecutionStatus::Failed
            } else if job_status.get("active").and_then(|s| s.as_u64()) == Some(1) {
                ExecutionStatus::Running
            } else {
                ExecutionStatus::Queued
            }
        } else {
            ExecutionStatus::Queued
        };

        Ok(status)
    }

    async fn wait_for_completion(
        &self,
        context: &ExecutionContext,
        timeout: std::time::Duration,
    ) -> Result<JobResult, ProviderError> {
        let start = std::time::Instant::now();
        let check_interval = std::time::Duration::from_secs(2);

        loop {
            if start.elapsed() > timeout {
                // Cancel the job on timeout
                let _ = self.cancel(context).await;
                return Ok(JobResult::Timeout);
            }

            let status = self.check_status(context).await?;

            match status {
                ExecutionStatus::Succeeded => return Ok(JobResult::Success),
                ExecutionStatus::Failed => return Ok(JobResult::Failed { exit_code: 1 }),
                ExecutionStatus::Cancelled => return Ok(JobResult::Cancelled),
                _ => {} // Keep waiting
            }

            tokio::time::sleep(check_interval).await;
        }
    }

    async fn get_logs(&self, context: &ExecutionContext) -> Result<Vec<LogEntry>, ProviderError> {
        // Get the pods for this job
        let pods_path = format!(
            "/api/v1/namespaces/{}/pods?labelSelector=hodei.job.id%3D{}",
            self.namespace, context.job_id
        );

        let response = self.k8s_get(&pods_path).await?;

        if !response.status().is_success() {
            return Err(ProviderError::Provider(format!(
                "Failed to get pods for job: HTTP {}",
                response.status()
            )));
        }

        let pods: Value = response.json().await.map_err(|e| {
            ProviderError::Provider(format!("Failed to parse pods response: {}", e))
        })?;

        let mut log_entries = Vec::new();

        if let Some(items) = pods.get("items").and_then(|i| i.as_array()) {
            for pod in items {
                if let Some(pod_name) = pod
                    .get("metadata")
                    .and_then(|m| m.get("name"))
                    .and_then(|n| n.as_str())
                {
                    // Get logs from this pod
                    let logs_path = format!(
                        "/api/v1/namespaces/{}/pods/{}/log",
                        self.namespace, pod_name
                    );

                    let log_response = self.k8s_get(&logs_path).await?;

                    if log_response.status().is_success() {
                        let log_text = log_response.text().await.unwrap_or_default();

                        for line in log_text.lines() {
                            log_entries.push(LogEntry {
                                job_id: context.job_id.clone(),
                                timestamp: chrono::Utc::now(),
                                message: line.to_string(),
                                stream_type: LogStreamType::Stdout,
                            });
                        }
                    }
                }
            }
        }

        Ok(log_entries)
    }

    async fn cancel(&self, context: &ExecutionContext) -> Result<(), ProviderError> {
        let path = format!(
            "/apis/batch/v1/namespaces/{}/jobs/{}",
            self.namespace, context.provider_execution_id
        );

        let delete_body = Some(json!({
            "apiVersion": "batch/v1",
            "kind": "DeleteOptions",
            "gracePeriodSeconds": 0
        }));

        let response = self.k8s_delete(&path, delete_body.as_ref()).await?;

        if !response.status().is_success() && response.status() != 404 {
            return Err(ProviderError::Provider(format!(
                "Failed to cancel job '{}': HTTP {}",
                context.provider_execution_id,
                response.status()
            )));
        }

        Ok(())
    }

    async fn health_check(&self) -> Result<bool, ProviderError> {
        // Try to get the API server version as a health check
        let path = "/apis/metrics.k8s.io/v1beta1/nodes";

        match self.k8s_get(path).await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => {
                tracing::error!("Kubernetes health check failed: {}", e);
                Ok(false)
            }
        }
    }

    fn estimate_cost(&self, spec: &JobSpec) -> Option<f64> {
        // Kubernetes cost depends on cluster pricing
        // This is a simplified estimate
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

        let cpu_cores = if let Some(cpu) = &spec.resources.cpu {
            if cpu.ends_with("m") {
                cpu.trim_end_matches("m").parse::<f64>().unwrap_or(2000.0) / 1000.0
            } else {
                cpu.parse::<f64>().unwrap_or(1.0)
            }
        } else {
            1.0
        };

        // Estimate: $0.05 per CPU core-hour + $0.01 per GB-memory-hour
        Some(cpu_cores * 0.05 + memory_gb * 0.01)
    }
}
