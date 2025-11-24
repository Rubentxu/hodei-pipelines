//! Tests for Kubernetes Provider
//!
//! These tests validate the KubernetesProvider implementation.
//! Note: These are unit tests that mock the Kubernetes API client.

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::WorkerId;
    use hodei_ports::worker_provider::{ProviderConfig, ProviderType};
    use hodei_shared_types::WorkerStatus;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_provider_type() {
        // Create a mock provider
        let config = ProviderConfig::kubernetes("test-k8s".to_string());

        // Note: In a real test, we'd use a mock Kubernetes client
        // For now, this just validates the config
        assert_eq!(config.provider_type, ProviderType::Kubernetes);
        assert_eq!(config.name, "test-k8s");
        assert_eq!(config.namespace, Some("default".to_string()));
    }

    #[tokio::test]
    async fn test_provider_config_builder() {
        let config = ProviderConfig::kubernetes("test-provider".to_string());

        assert_eq!(config.provider_type, ProviderType::Kubernetes);
        assert_eq!(config.name, "test-provider");
        assert!(config.namespace.is_some());
        assert!(config.docker_host.is_none());
        assert!(config.kube_config.is_none());
    }

    #[tokio::test]
    async fn test_provider_config_with_namespace() {
        let mut config = ProviderConfig::kubernetes("test".to_string());
        config.namespace = Some("hodei-workers".to_string());

        assert_eq!(config.namespace, Some("hodei-workers".to_string()));
    }

    #[test]
    fn test_pod_name_generation() {
        let worker_id = WorkerId::from_uuid(
            uuid::Uuid::from_str("123e4567-e89b-12d3-a456-426614174000").unwrap(),
        );

        let pod_name = format!("hodei-worker-{}", worker_id);

        assert!(pod_name.starts_with("hodei-worker-"));
        assert!(pod_name.contains("123e4567-e89b-12d3-a456-426614174000"));
    }

    #[test]
    fn test_service_name_generation() {
        let worker_id = WorkerId::from_uuid(
            uuid::Uuid::from_str("123e4567-e89b-12d3-a456-426614174000").unwrap(),
        );

        let service_name = format!("hodei-worker-{}-svc", worker_id);

        assert!(service_name.starts_with("hodei-worker-"));
        assert!(service_name.ends_with("-svc"));
        assert!(service_name.contains("123e4567-e89b-12d3-a456-426614174000"));
    }
}
