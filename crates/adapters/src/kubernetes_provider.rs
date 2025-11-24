//! Kubernetes Provider Adapter
//!
//! This module provides a concrete implementation of the WorkerProvider port
//! using kubectl CLI commands for Kubernetes operations.

use async_trait::async_trait;
use hodei_core::{Worker, WorkerId};
use hodei_ports::worker_provider::{
    ProviderCapabilities, ProviderConfig, ProviderError, ProviderType, WorkerProvider,
};
use hodei_shared_types::WorkerStatus;
use std::process::Command;

/// Kubernetes worker provider implementation using kubectl
#[derive(Debug, Clone)]
pub struct KubernetesProvider {
    namespace: String,
    name: String,
}

impl KubernetesProvider {
    pub async fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        if config.provider_type != ProviderType::Kubernetes {
            return Err(ProviderError::InvalidConfiguration(
                "Expected Kubernetes provider configuration".to_string(),
            ));
        }

        let namespace = config.namespace.unwrap_or_else(|| "default".to_string());

        Ok(Self {
            namespace,
            name: config.name,
        })
    }

    fn create_pod_name(worker_id: &WorkerId) -> String {
        format!("hodei-worker-{}", worker_id)
    }

    fn create_pod_yaml(worker_id: &WorkerId, namespace: &str) -> String {
        let pod_name = Self::create_pod_name(worker_id);
        let grpc_url = std::env::var("HODEI_SERVER_GRPC_URL")
            .unwrap_or_else(|_| "http://hodei-server:50051".to_string());

        format!(
            r#"apiVersion: v1
kind: Pod
metadata:
  name: {pod_name}
  namespace: {namespace}
  labels:
    app: hodei-worker
    worker.id: {worker_id}
    managed-by: hodei
spec:
  containers:
  - name: {pod_name}
    image: hodei-worker:latest
    env:
    - name: WORKER_ID
      value: "{worker_id}"
    - name: HODEI_SERVER_GRPC_URL
      value: "{grpc_url}"
    resources:
      requests:
        cpu: 100m
        memory: 128Mi
      limits:
        cpu: 1000m
        memory: 2Gi
    ports:
    - containerPort: 50051
      protocol: TCP
  restartPolicy: Never
"#
        )
    }

    fn run_kubectl(args: &[&str]) -> Result<std::process::Output, ProviderError> {
        let output = Command::new("kubectl")
            .args(args)
            .output()
            .map_err(|e| ProviderError::Provider(format!("Failed to execute kubectl: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ProviderError::Provider(format!(
                "kubectl command failed: {}",
                stderr
            )));
        }

        Ok(output)
    }
}

#[async_trait]
impl WorkerProvider for KubernetesProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Kubernetes
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn capabilities(&self) -> Result<ProviderCapabilities, ProviderError> {
        Ok(ProviderCapabilities {
            supports_auto_scaling: true,
            supports_health_checks: true,
            supports_volumes: true,
            max_workers: Some(500),
            estimated_provision_time_ms: 10000,
        })
    }

    async fn create_worker(
        &self,
        worker_id: WorkerId,
        _config: ProviderConfig,
    ) -> Result<Worker, ProviderError> {
        let pod_name = Self::create_pod_name(&worker_id);
        let yaml = KubernetesProvider::create_pod_yaml(&worker_id, &self.namespace);

        // Apply the pod using kubectl (write YAML to file and apply)
        let temp_file = format!("/tmp/pod-{}.yaml", worker_id.clone());
        std::fs::write(&temp_file, yaml)
            .map_err(|e| ProviderError::Provider(format!("Failed to write YAML file: {}", e)))?;

        let args = &[
            "apply",
            "-f",
            &temp_file,
            "-n",
            &self.namespace,
            "--validate=false",
        ];
        let output = Self::run_kubectl(args)?;

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);

        // Wait for pod to be running
        let args = &["get", "pod", &pod_name, "-n", &self.namespace];
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_secs(60) {
            let output = Self::run_kubectl(args)?;

            if String::from_utf8_lossy(&output.stdout).contains("Running") {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // Create Worker domain entity
        let worker = Worker::new(
            worker_id.clone(),
            format!("worker-{}", worker_id),
            hodei_shared_types::WorkerCapabilities::new(1, 2048),
        );

        Ok(worker)
    }

    async fn get_worker_status(&self, worker_id: &WorkerId) -> Result<WorkerStatus, ProviderError> {
        let pod_name = Self::create_pod_name(worker_id);
        let args = &[
            "get",
            "pod",
            &pod_name,
            "-n",
            &self.namespace,
            "-o",
            "jsonpath={.status.phase}",
        ];

        let output = Self::run_kubectl(args)?;

        let phase = String::from_utf8_lossy(&output.stdout).trim().to_string();

        let status = match phase.as_str() {
            "Running" => WorkerStatus::new(worker_id.clone(), WorkerStatus::IDLE.to_string()),
            "Pending" | "ContainerCreating" => {
                WorkerStatus::new(worker_id.clone(), "PROVISIONING".to_string())
            }
            "Succeeded" | "Failed" | "Unknown" | "" => {
                WorkerStatus::new(worker_id.clone(), WorkerStatus::OFFLINE.to_string())
            }
            _ => WorkerStatus::new(worker_id.clone(), "UNKNOWN".to_string()),
        };

        Ok(status)
    }

    async fn stop_worker(&self, worker_id: &WorkerId, graceful: bool) -> Result<(), ProviderError> {
        let pod_name = Self::create_pod_name(worker_id);
        let args = &["delete", "pod", &pod_name, "-n", &self.namespace];

        Self::run_kubectl(args)?;
        Ok(())
    }

    async fn delete_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        self.stop_worker(worker_id, true).await
    }

    async fn list_workers(&self) -> Result<Vec<WorkerId>, ProviderError> {
        let args = &[
            "get",
            "pods",
            "-n",
            &self.namespace,
            "-l",
            "app=hodei-worker",
            "-o",
            "jsonpath={.items[*].metadata.labels.worker\\.id}",
        ];

        let output = Self::run_kubectl(args)?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut worker_ids = Vec::new();
        for worker_id_str in stdout.split_whitespace() {
            if let Ok(uuid) = uuid::Uuid::parse_str(worker_id_str) {
                worker_ids.push(WorkerId::from_uuid(uuid));
            }
        }

        Ok(worker_ids)
    }
}
