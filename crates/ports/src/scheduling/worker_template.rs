//! Worker template abstraction for provider-agnostic worker configuration.
//!
//! This module defines the WorkerTemplate system that allows generating
//! provider-specific configurations (Kubernetes Pods, Docker containers, etc.)
//! from a single abstract template definition.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during template operations.
#[derive(Error, Debug)]
pub enum TemplateError {
    #[error("Template validation failed: {message}")]
    Validation { message: String },

    #[error("Provider not supported: {provider}")]
    UnsupportedProvider { provider: String },

    #[error("Template generation failed: {source}")]
    GenerationFailed {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Invalid resource specification: {message}")]
    InvalidResource { message: String },
}

/// Worker resources (CPU, memory, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU cores (e.g., "2", "2000m")
    pub cpu: Option<String>,
    /// Memory (e.g., "4Gi", "512Mi")
    pub memory: Option<String>,
    /// Ephemeral storage (e.g., "10Gi")
    pub ephemeral_storage: Option<String>,
}

/// Tool specification for programmatic installation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Tool name (e.g., "nodejs", "rust", "python")
    pub name: String,
    /// Version (e.g., "18.17.0", "1.72", "3.11")
    pub version: String,
    /// Plugin for asdf-vm
    pub plugin: Option<String>,
}

/// Volume mount configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Volume name
    pub name: String,
    /// Mount path in the container
    pub mount_path: String,
    /// Sub-path within the volume
    pub sub_path: Option<String>,
    /// Read-only flag
    pub read_only: bool,
}

/// Environment variable
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvVar {
    /// Environment variable name
    pub name: String,
    /// Environment variable value
    pub value: String,
}

/// Network configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Port mappings for container/worker access
    pub ports: Vec<PortMapping>,
    /// Network mode (bridge, host, etc.)
    pub network_mode: Option<String>,
}

/// Port mapping definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortMapping {
    /// Port number
    pub port: u32,
    /// Protocol (TCP/UDP)
    pub protocol: String,
    /// Host port (if applicable)
    pub host_port: Option<u32>,
}

/// Security context and privileges
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Run as user ID
    pub run_as_user: Option<u32>,
    /// Run as group ID
    pub run_as_group: Option<u32>,
    /// SELinux options
    pub selinux_options: Option<HashMap<String, String>>,
    /// Privileged mode
    pub privileged: Option<bool>,
    /// Capabilities to add
    pub capabilities_add: Option<Vec<String>>,
    /// Capabilities to drop
    pub capabilities_drop: Option<Vec<String>>,
}

/// WorkerTemplate defines the abstract configuration for creating workers
/// across different providers (Kubernetes, Docker, VMs, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerTemplate {
    /// Template metadata
    pub metadata: TemplateMetadata,
    /// Container/image configuration
    pub container: ContainerSpec,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Volume mounts for workspace and tools
    pub volumes: Vec<VolumeMount>,
    /// Environment variables
    pub env: Vec<EnvVar>,
    /// Network configuration
    pub network: Option<NetworkConfig>,
    /// Security context
    pub security: Option<SecurityContext>,
    /// Tools to be installed via asdf-vm
    pub tools: Vec<ToolSpec>,
    /// Labels for filtering and selection
    pub labels: HashMap<String, String>,
    /// Annotations for metadata
    pub annotations: HashMap<String, String>,
    /// Startup command/args override
    pub command: Option<Vec<String>>,
    /// Working directory
    pub working_dir: Option<String>,
    /// Provider-specific configuration
    pub provider_config: HashMap<String, serde_json::Value>,
}

/// Template metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateMetadata {
    /// Template name
    pub name: String,
    /// Template version
    pub version: String,
    /// Description
    pub description: Option<String>,
    /// Labels for categorization
    pub labels: HashMap<String, String>,
}

/// Container specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSpec {
    /// Container image
    pub image: String,
    /// Image pull policy
    pub image_pull_policy: Option<String>,
    /// Container command
    pub command: Option<Vec<String>>,
    /// Container args
    pub args: Option<Vec<String>>,
    /// Working directory
    pub working_dir: Option<String>,
}

/// Provider-specific configuration generator
pub trait WorkerTemplateGenerator {
    /// Generate provider-specific configuration from template
    fn generate(
        &self,
        template: &WorkerTemplate,
        name: &str,
    ) -> Result<serde_json::Value, TemplateError>;

    /// Validate template for this provider
    fn validate(&self, template: &WorkerTemplate) -> Result<(), TemplateError>;
}

/// Kubernetes template generator
pub struct KubernetesTemplateGenerator {
    pub namespace: Option<String>,
    pub service_account: Option<String>,
    pub node_selector: Option<HashMap<String, String>>,
    pub affinity: Option<serde_json::Value>,
    pub tolerations: Option<Vec<serde_json::Value>>,
}

impl KubernetesTemplateGenerator {
    pub fn new() -> Self {
        Self {
            namespace: None,
            service_account: None,
            node_selector: None,
            affinity: None,
            tolerations: None,
        }
    }

    /// Generate Kubernetes Pod specification
    pub fn generate_pod_spec(
        &self,
        template: &WorkerTemplate,
        _name: &str,
    ) -> Result<serde_json::Value, TemplateError> {
        let _pod_spec = serde_json::Map::new();

        // Container spec
        let mut container = serde_json::Map::new();
        container.insert(
            "name".to_string(),
            serde_json::Value::String(_name.to_string()),
        );
        container.insert(
            "image".to_string(),
            serde_json::Value::String(template.container.image.clone()),
        );

        if let Some(command) = &template.command {
            container.insert(
                "command".to_string(),
                serde_json::to_value(command).unwrap(),
            );
        }

        if let Some(args) = &template.container.args {
            container.insert("args".to_string(), serde_json::to_value(args).unwrap());
        }

        if let Some(working_dir) = &template.working_dir {
            container.insert(
                "workingDir".to_string(),
                serde_json::Value::String(working_dir.clone()),
            );
        }

        // Environment variables
        let mut env_vars = Vec::new();
        for env in &template.env {
            let mut env_var = serde_json::Map::new();
            env_var.insert(
                "name".to_string(),
                serde_json::Value::String(env.name.clone()),
            );
            env_var.insert(
                "value".to_string(),
                serde_json::Value::String(env.value.clone()),
            );
            env_vars.push(serde_json::Value::Object(env_var));
        }
        if !env_vars.is_empty() {
            container.insert("env".to_string(), serde_json::Value::Array(env_vars));
        }

        // Volume mounts
        let mut volume_mounts = Vec::new();
        for vol in &template.volumes {
            let mut vm = serde_json::Map::new();
            vm.insert(
                "name".to_string(),
                serde_json::Value::String(vol.name.clone()),
            );
            vm.insert(
                "mountPath".to_string(),
                serde_json::Value::String(vol.mount_path.clone()),
            );
            if let Some(sub_path) = &vol.sub_path {
                vm.insert(
                    "subPath".to_string(),
                    serde_json::Value::String(sub_path.clone()),
                );
            }
            vm.insert(
                "readOnly".to_string(),
                serde_json::Value::Bool(vol.read_only),
            );
            volume_mounts.push(serde_json::Value::Object(vm));
        }
        if !volume_mounts.is_empty() {
            container.insert(
                "volumeMounts".to_string(),
                serde_json::Value::Array(volume_mounts),
            );
        }

        // Resources
        let mut resources = serde_json::Map::new();
        if let Some(cpu) = &template.resources.cpu {
            let mut requests = serde_json::Map::new();
            requests.insert("cpu".to_string(), serde_json::Value::String(cpu.clone()));
            resources.insert("requests".to_string(), serde_json::Value::Object(requests));
        }
        if let Some(memory) = &template.resources.memory {
            let requests = resources
                .get("requests")
                .and_then(|v| v.as_object())
                .map(|m| {
                    let mut requests = m.clone();
                    requests.insert(
                        "memory".to_string(),
                        serde_json::Value::String(memory.clone()),
                    );
                    requests
                })
                .unwrap_or_else(|| {
                    let mut requests = serde_json::Map::new();
                    requests.insert(
                        "memory".to_string(),
                        serde_json::Value::String(memory.clone()),
                    );
                    requests
                });
            resources.insert("requests".to_string(), serde_json::Value::Object(requests));
        }
        if !resources.is_empty() {
            container.insert(
                "resources".to_string(),
                serde_json::Value::Object(resources),
            );
        }

        // Container ports
        if let Some(network) = &template.network {
            let mut ports = Vec::new();
            for port_map in &network.ports {
                let mut port = serde_json::Map::new();
                port.insert(
                    "containerPort".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(port_map.port)),
                );
                port.insert(
                    "protocol".to_string(),
                    serde_json::Value::String(port_map.protocol.clone()),
                );
                ports.push(serde_json::Value::Object(port));
            }
            if !ports.is_empty() {
                container.insert("ports".to_string(), serde_json::Value::Array(ports));
            }
        }

        // Pod spec
        let mut spec = serde_json::Map::new();
        spec.insert(
            "containers".to_string(),
            serde_json::Value::Array(vec![serde_json::Value::Object(container)]),
        );

        // Volumes
        let mut volumes = Vec::new();
        for vol in &template.volumes {
            let mut volume = serde_json::Map::new();
            volume.insert(
                "name".to_string(),
                serde_json::Value::String(vol.name.clone()),
            );
            // For now, use emptyDir - can be extended for PVCs
            let mut empty_dir = serde_json::Map::new();
            empty_dir.insert(
                "sizeLimit".to_string(),
                serde_json::Value::String("10Gi".to_string()),
            );
            volume.insert("emptyDir".to_string(), serde_json::Value::Object(empty_dir));
            volumes.push(serde_json::Value::Object(volume));
        }
        if !volumes.is_empty() {
            spec.insert("volumes".to_string(), serde_json::Value::Array(volumes));
        }

        // Node selector
        if let Some(node_selector) = &self.node_selector {
            spec.insert(
                "nodeSelector".to_string(),
                serde_json::to_value(node_selector).unwrap(),
            );
        }

        // Affinity
        if let Some(affinity) = &self.affinity {
            spec.insert("affinity".to_string(), affinity.clone());
        }

        // Tolerations
        if let Some(tolerations) = &self.tolerations {
            spec.insert(
                "tolerations".to_string(),
                serde_json::to_value(tolerations).unwrap(),
            );
        }

        // Service account
        if let Some(sa) = &self.service_account {
            spec.insert(
                "serviceAccountName".to_string(),
                serde_json::Value::String(sa.clone()),
            );
        }

        // Security context
        if let Some(security) = &template.security {
            let mut pod_security = serde_json::Map::new();
            if let Some(run_as_user) = security.run_as_user {
                pod_security.insert(
                    "runAsUser".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(run_as_user)),
                );
            }
            if let Some(run_as_group) = security.run_as_group {
                pod_security.insert(
                    "runAsGroup".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(run_as_group)),
                );
            }
            if let Some(fs_group) = security.run_as_group {
                pod_security.insert(
                    "fsGroup".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(fs_group)),
                );
            }
            if !pod_security.is_empty() {
                spec.insert(
                    "securityContext".to_string(),
                    serde_json::Value::Object(pod_security),
                );
            }
        }

        Ok(serde_json::Value::Object(spec))
    }
}

impl Default for KubernetesTemplateGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerTemplateGenerator for KubernetesTemplateGenerator {
    fn generate(
        &self,
        template: &WorkerTemplate,
        name: &str,
    ) -> Result<serde_json::Value, TemplateError> {
        self.generate_pod_spec(template, name)
            .map_err(|e| TemplateError::GenerationFailed {
                source: Box::new(std::io::Error::other(
                    e.to_string(),
                )),
            })
    }

    fn validate(&self, template: &WorkerTemplate) -> Result<(), TemplateError> {
        if template.container.image.is_empty() {
            return Err(TemplateError::Validation {
                message: "Container image is required".to_string(),
            });
        }

        // Add more validations as needed
        Ok(())
    }
}

/// Docker template generator
pub struct DockerTemplateGenerator {
    pub network: Option<String>,
    pub shm_size: Option<String>,
    pub ulimits: Option<HashMap<String, String>>,
}

impl DockerTemplateGenerator {
    pub fn new() -> Self {
        Self {
            network: None,
            shm_size: None,
            ulimits: None,
        }
    }

    /// Generate Docker Compose-style configuration
    pub fn generate_docker_config(
        &self,
        template: &WorkerTemplate,
        name: &str,
    ) -> Result<serde_json::Value, TemplateError> {
        let mut service = serde_json::Map::new();

        service.insert(
            "image".to_string(),
            serde_json::Value::String(template.container.image.clone()),
        );

        if let Some(command) = &template.command {
            service.insert(
                "command".to_string(),
                serde_json::to_value(command).unwrap(),
            );
        }

        // Environment variables
        let mut env_vars = std::collections::BTreeMap::new();
        for env in &template.env {
            env_vars.insert(env.name.clone(), env.value.clone());
        }
        service.insert(
            "environment".to_string(),
            serde_json::to_value(env_vars).unwrap(),
        );

        // Volume mounts
        let mut volumes = Vec::new();
        for vol in &template.volumes {
            let mount_spec = format!("{}:{}", vol.name, vol.mount_path);
            volumes.push(serde_json::Value::String(mount_spec));
        }
        if !volumes.is_empty() {
            service.insert("volumes".to_string(), serde_json::Value::Array(volumes));
        }

        // Network
        if let Some(network) = &template.network
            && let Some(mode) = &network.network_mode {
                service.insert(
                    "network_mode".to_string(),
                    serde_json::Value::String(mode.clone()),
                );
            }

        // Resources (memory, cpu)
        let mut deploy = serde_json::Map::new();
        let mut resources = serde_json::Map::new();

        if let Some(cpu) = &template.resources.cpu {
            let mut limits = serde_json::Map::new();
            limits.insert("cpus".to_string(), serde_json::Value::String(cpu.clone()));
            resources.insert("limits".to_string(), serde_json::Value::Object(limits));
        }

        if let Some(memory) = &template.resources.memory {
            let limits = resources
                .get("limits")
                .and_then(|v| v.as_object())
                .map(|m| {
                    let mut limits = m.clone();
                    limits.insert(
                        "memory".to_string(),
                        serde_json::Value::String(memory.clone()),
                    );
                    limits
                })
                .unwrap_or_else(|| {
                    let mut limits = serde_json::Map::new();
                    limits.insert(
                        "memory".to_string(),
                        serde_json::Value::String(memory.clone()),
                    );
                    limits
                });
            resources.insert("limits".to_string(), serde_json::Value::Object(limits));
        }

        if !resources.is_empty() {
            deploy.insert(
                "resources".to_string(),
                serde_json::Value::Object(resources),
            );
            service.insert("deploy".to_string(), serde_json::Value::Object(deploy));
        }

        Ok(serde_json::Value::Object(service))
    }
}

impl Default for DockerTemplateGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerTemplateGenerator for DockerTemplateGenerator {
    fn generate(
        &self,
        template: &WorkerTemplate,
        name: &str,
    ) -> Result<serde_json::Value, TemplateError> {
        self.generate_docker_config(template, name)
            .map_err(|e| TemplateError::GenerationFailed {
                source: Box::new(std::io::Error::other(
                    e.to_string(),
                )),
            })
    }

    fn validate(&self, template: &WorkerTemplate) -> Result<(), TemplateError> {
        if template.container.image.is_empty() {
            return Err(TemplateError::Validation {
                message: "Container image is required".to_string(),
            });
        }
        Ok(())
    }
}

impl WorkerTemplate {
    /// Create a new worker template
    pub fn new(name: &str, version: &str, image: &str) -> Self {
        Self {
            metadata: TemplateMetadata {
                name: name.to_string(),
                version: version.to_string(),
                description: None,
                labels: HashMap::new(),
            },
            container: ContainerSpec {
                image: image.to_string(),
                image_pull_policy: None,
                command: None,
                args: None,
                working_dir: None,
            },
            resources: ResourceRequirements {
                cpu: None,
                memory: None,
                ephemeral_storage: None,
            },
            volumes: Vec::new(),
            env: Vec::new(),
            network: None,
            security: None,
            tools: Vec::new(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
            command: None,
            working_dir: None,
            provider_config: HashMap::new(),
        }
    }

    /// Set container image
    pub fn with_image(mut self, image: &str) -> Self {
        self.container.image = image.to_string();
        self
    }

    /// Add CPU resource requirement
    pub fn with_cpu(mut self, cpu: &str) -> Self {
        self.resources.cpu = Some(cpu.to_string());
        self
    }

    /// Add memory resource requirement
    pub fn with_memory(mut self, memory: &str) -> Self {
        self.resources.memory = Some(memory.to_string());
        self
    }

    /// Add environment variable
    pub fn with_env(mut self, name: &str, value: &str) -> Self {
        self.env.push(EnvVar {
            name: name.to_string(),
            value: value.to_string(),
        });
        self
    }

    /// Add volume mount
    pub fn with_volume(mut self, name: &str, mount_path: &str) -> Self {
        self.volumes.push(VolumeMount {
            name: name.to_string(),
            mount_path: mount_path.to_string(),
            sub_path: None,
            read_only: false,
        });
        self
    }

    /// Add tool to be installed
    pub fn with_tool(mut self, name: &str, version: &str, plugin: Option<&str>) -> Self {
        self.tools.push(ToolSpec {
            name: name.to_string(),
            version: version.to_string(),
            plugin: plugin.map(|p| p.to_string()),
        });
        self
    }

    /// Add label
    pub fn with_label(mut self, key: &str, value: &str) -> Self {
        self.labels.insert(key.to_string(), value.to_string());
        self
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: &str) -> Self {
        self.working_dir = Some(dir.to_string());
        self
    }

    /// Set custom command
    pub fn with_command(mut self, command: Vec<String>) -> Self {
        self.command = Some(command);
        self
    }

    /// Validate the template
    pub fn validate(&self) -> Result<(), TemplateError> {
        if self.container.image.is_empty() {
            return Err(TemplateError::Validation {
                message: "Container image is required".to_string(),
            });
        }

        if self.metadata.name.is_empty() {
            return Err(TemplateError::Validation {
                message: "Template name is required".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_template_creation() {
        let template = WorkerTemplate::new("test-template", "1.0.0", "ubuntu:22.04")
            .with_cpu("2")
            .with_memory("4Gi")
            .with_env("ENV", "production");

        assert_eq!(template.metadata.name, "test-template");
        assert_eq!(template.container.image, "ubuntu:22.04");
        assert_eq!(template.resources.cpu, Some("2".to_string()));
        assert_eq!(template.resources.memory, Some("4Gi".to_string()));
        assert_eq!(template.env.len(), 1);
    }

    #[test]
    fn test_kubernetes_generator() {
        let template = WorkerTemplate::new("test", "1.0.0", "ubuntu:22.04")
            .with_cpu("2")
            .with_memory("4Gi")
            .with_volume("workspace", "/workspace");

        let generator = KubernetesTemplateGenerator::new();
        let pod_spec = generator
            .generate_pod_spec(&template, "test-worker")
            .unwrap();

        let pod_spec_obj = pod_spec.as_object().unwrap();
        assert!(pod_spec_obj.contains_key("containers"));
    }

    #[test]
    fn test_docker_generator() {
        let template = WorkerTemplate::new("test", "1.0.0", "ubuntu:22.04")
            .with_cpu("2")
            .with_memory("4Gi");

        let generator = DockerTemplateGenerator::new();
        let docker_config = generator
            .generate_docker_config(&template, "test-worker")
            .unwrap();

        let service = docker_config.as_object().unwrap();
        assert!(service.contains_key("image"));
        assert!(service.contains_key("environment"));
    }

    #[test]
    fn test_template_validation() {
        let template = WorkerTemplate::new("test", "1.0.0", "ubuntu:22.04");
        assert!(template.validate().is_ok());

        let invalid_template = WorkerTemplate::new("", "1.0.0", "");
        assert!(invalid_template.validate().is_err());
    }
}
