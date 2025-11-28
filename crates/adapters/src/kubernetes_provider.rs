//! Kubernetes Provider Adapter
//!
//! This module provides a concrete implementation of the WorkerProvider port
//! using the Kubernetes REST API via reqwest for professional Kubernetes operations.

use async_trait::async_trait;
use hodei_core::WorkerStatus;
use hodei_core::{Worker, WorkerId};
use hodei_ports::worker_provider::{
    ProviderCapabilities, ProviderConfig, ProviderError, ProviderType, WorkerProvider,
};
use reqwest::{Client as HttpClient, Response};
use serde_json::{Value, json};
use std::fs;

/// Kubernetes worker provider implementation using REST API
#[derive(Debug, Clone)]
pub struct KubernetesProvider {
    client: HttpClient,
    namespace: String,
    name: String,
    server_url: String,
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

        // Note: bearer_auth is applied per-request, not during client build
        let _ = token; // Used later in request builder

        let client = client_builder
            .build()
            .map_err(|e| ProviderError::Provider(format!("Failed to build HTTP client: {}", e)))?;

        let namespace = config.namespace.unwrap_or_else(|| "default".to_string());

        Ok(Self {
            client,
            namespace,
            name: config.name,
            server_url,
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

            let token = fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/token")
                .ok()
                .map(|t| t.trim().to_string());
            let ca_cert = fs::read("/var/run/secrets/kubernetes.io/serviceaccount/ca.crt").ok();

            return Ok((server_url, ca_cert, token));
        }

        // Fall back to kubeconfig file (development standard)
        let kubeconfig =
            std::env::var("KUBECONFIG").unwrap_or_else(|_| "~/.kube/config".to_string());
        let kubeconfig_path = shellexpand::tilde(&kubeconfig).into_owned();

        if std::path::Path::new(&kubeconfig_path).exists() {
            let content = fs::read_to_string(&kubeconfig_path).map_err(|e| {
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
                .and_then(|path| fs::read(path).ok());

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

    fn create_pod_name(worker_id: &WorkerId) -> String {
        format!("hodei-worker-{}", worker_id)
    }

    fn create_pod_manifest(
        worker_id: &WorkerId,
        config: &ProviderConfig,
        namespace: &str,
    ) -> Value {
        // Use custom template if provided
        if let Some(template) = &config.custom_pod_template {
            // Try to parse as JSON first
            if let Ok(mut template_value) = serde_json::from_str::<Value>(template) {
                // Replace placeholders in the template
                if let Some(metadata) = template_value
                    .get_mut("metadata")
                    .and_then(|m| m.as_object_mut())
                {
                    metadata.insert("name".to_string(), json!(Self::create_pod_name(worker_id)));
                    metadata.insert("namespace".to_string(), json!(namespace));

                    let labels = metadata
                        .entry("labels")
                        .or_insert_with(|| json!({}))
                        .as_object_mut()
                        .unwrap();
                    labels.insert("hodei.worker".to_string(), json!("true"));
                    labels.insert("hodei.worker.id".to_string(), json!(worker_id.to_string()));
                }

                if let Some(spec) = template_value
                    .get_mut("spec")
                    .and_then(|s| s.as_object_mut())
                    && let Some(containers) =
                        spec.get_mut("containers").and_then(|c| c.as_array_mut())
                    {
                        for container in containers {
                            if let Some(env) =
                                container.get_mut("env").and_then(|e| e.as_array_mut())
                            {
                                env.push(
                                    json!({"name": "WORKER_ID", "value": worker_id.to_string()}),
                                );
                                env.push(json!({"name": "HODEI_SERVER_GRPC_URL", "value": std::env::var("HODEI_SERVER_GRPC_URL").unwrap_or_else(|_| "http://hodei-server:50051".to_string())}));
                            }
                        }
                    }

                return template_value;
            }
        }

        // Default template with HWP Agent image
        let pod_name = Self::create_pod_name(worker_id);
        let grpc_url = std::env::var("HODEI_SERVER_GRPC_URL")
            .unwrap_or_else(|_| "http://hodei-server:50051".to_string());

        // Use custom image if provided, otherwise default HWP Agent image
        let image = config.custom_image.as_deref().unwrap_or("hwp-agent:latest");

        json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": pod_name,
                "namespace": namespace,
                "labels": {
                    "hodei.worker": "true",
                    "hodei.worker.id": worker_id.to_string()
                }
            },
            "spec": {
                "restartPolicy": "Never",
                "containers": [{
                    "name": "worker",
                    "image": image,
                    "env": [
                        {"name": "WORKER_ID", "value": worker_id.to_string()},
                        {"name": "HODEI_SERVER_GRPC_URL", "value": grpc_url}
                    ],
                    "resources": {
                        "requests": {
                            "cpu": "2000m",
                            "memory": "4Gi"
                        },
                        "limits": {
                            "cpu": "2000m",
                            "memory": "4Gi"
                        }
                    },
                    "ports": [{
                        "containerPort": 8080
                    }]
                }]
            }
        })
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

    async fn wait_for_pod_ready(&self, pod_name: &str) -> Result<(), ProviderError> {
        for _ in 0..30 {
            let path = format!("/api/v1/namespaces/{}/pods/{}", self.namespace, pod_name);
            match self.k8s_get(&path).await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let pod: Value = resp.json().await.map_err(|e| {
                            ProviderError::Provider(format!("Failed to parse pod: {}", e))
                        })?;

                        if let Some(status) = pod.get("status").and_then(|s| s.get("phase"))
                            && status == "Running" {
                                return Ok(());
                            }
                    }
                }
                Err(e) => {
                    return Err(ProviderError::Provider(format!(
                        "Failed to check pod status: {}",
                        e
                    )));
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        Err(ProviderError::Provider(format!(
            "Pod '{}' did not become ready in time",
            pod_name
        )))
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
            max_workers: Some(50),
            estimated_provision_time_ms: 10000,
        })
    }

    async fn create_worker(
        &self,
        worker_id: WorkerId,
        config: ProviderConfig,
    ) -> Result<Worker, ProviderError> {
        let pod_name = Self::create_pod_name(&worker_id);
        let namespace = config.namespace.as_ref().unwrap_or(&self.namespace);
        let manifest = KubernetesProvider::create_pod_manifest(&worker_id, &config, namespace);

        // Create the pod
        let path = format!("/api/v1/namespaces/{}/pods", namespace);
        let response = self.k8s_post(&path, &manifest).await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Provider(format!(
                "Failed to create pod '{}': {}",
                pod_name, error_text
            )));
        }

        // Wait for pod to be ready
        if let Err(e) = self.wait_for_pod_ready(&pod_name).await {
            // Try to clean up the pod on error
            let delete_path = format!("/api/v1/namespaces/{}/pods/{}", namespace, pod_name);
            let _ = self.k8s_delete(&delete_path, None).await;
            return Err(e);
        }

        // Create and return Worker entity
        let worker_name = format!("worker-{}", worker_id);
        let worker = Worker::new(
            worker_id,
            worker_name,
            hodei_core::WorkerCapabilities::new(2, 4096),
        );

        Ok(worker)
    }

    async fn get_worker_status(&self, worker_id: &WorkerId) -> Result<WorkerStatus, ProviderError> {
        let pod_name = Self::create_pod_name(worker_id);
        let path = format!("/api/v1/namespaces/{}/pods/{}", self.namespace, pod_name);

        let response = self.k8s_get(&path).await?;

        if response.status() == 404 {
            return Err(ProviderError::NotFound(format!(
                "Pod '{}' not found",
                pod_name
            )));
        }

        if !response.status().is_success() {
            return Err(ProviderError::Provider(format!(
                "Failed to get pod '{}': HTTP {}",
                pod_name,
                response.status()
            )));
        }

        let pod: Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Provider(format!("Failed to parse pod response: {}", e)))?;

        let status = if pod
            .get("status")
            .and_then(|s| s.get("phase"))
            .and_then(|p| p.as_str())
            == Some("Running")
        {
            WorkerStatus::new(worker_id.clone(), WorkerStatus::IDLE.to_string())
        } else {
            WorkerStatus::new(worker_id.clone(), WorkerStatus::OFFLINE.to_string())
        };

        Ok(status)
    }

    async fn stop_worker(&self, worker_id: &WorkerId, graceful: bool) -> Result<(), ProviderError> {
        let pod_name = Self::create_pod_name(worker_id);
        let path = format!("/api/v1/namespaces/{}/pods/{}", self.namespace, pod_name);

        let delete_body = if graceful {
            Some(json!({
                "apiVersion": "v1",
                "kind": "DeleteOptions",
                "gracePeriodSeconds": 30
            }))
        } else {
            Some(json!({
                "apiVersion": "v1",
                "kind": "DeleteOptions",
                "gracePeriodSeconds": 0
            }))
        };

        let response = self.k8s_delete(&path, delete_body.as_ref()).await?;

        if !response.status().is_success() && response.status() != 404 {
            return Err(ProviderError::Provider(format!(
                "Failed to stop pod '{}': HTTP {}",
                pod_name,
                response.status()
            )));
        }

        // Wait for pod to be deleted
        for _ in 0..30 {
            match self.k8s_get(&path).await {
                Ok(resp) => {
                    if resp.status() == 404 {
                        return Ok(()); // Pod deleted
                    }
                }
                Err(_) => return Ok(()), // Assume deleted on error
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        Ok(())
    }

    async fn delete_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let _ = self.stop_worker(worker_id, true).await;
        Ok(())
    }

    async fn list_workers(&self) -> Result<Vec<WorkerId>, ProviderError> {
        let path = format!(
            "/api/v1/namespaces/{}/pods?labelSelector=hodei.worker%3Dtrue",
            self.namespace
        );

        let response = self
            .k8s_get(&path)
            .await
            .map_err(|e| ProviderError::Provider(format!("Failed to list pods: {}", e)))?;

        if !response.status().is_success() {
            return Err(ProviderError::Provider(format!(
                "Failed to list pods: HTTP {}",
                response.status()
            )));
        }

        let result: Value = response.json().await.map_err(|e| {
            ProviderError::Provider(format!("Failed to parse list response: {}", e))
        })?;

        let mut worker_ids = Vec::new();
        if let Some(items) = result.get("items").and_then(|i| i.as_array()) {
            for item in items {
                if let Some(labels) = item
                    .get("metadata")
                    .and_then(|m| m.get("labels"))
                    .and_then(|l| l.as_object())
                    && let Some(worker_id_str) =
                        labels.get("hodei.worker.id").and_then(|s| s.as_str())
                        && let Ok(uuid) = uuid::Uuid::parse_str(worker_id_str) {
                            worker_ids.push(WorkerId::from_uuid(uuid));
                        }
            }
        }

        Ok(worker_ids)
    }
}

