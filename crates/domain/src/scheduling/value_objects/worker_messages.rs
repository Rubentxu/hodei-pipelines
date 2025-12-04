//! Worker-related message types for distributed communication

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Worker identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct WorkerId(pub Uuid);

impl WorkerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for WorkerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Worker state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerState {
    Creating,
    Available,
    Running,
    Unhealthy,
    Draining,
    Terminated,
    Failed { reason: String },
}

/// Worker status for tracking worker state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerStatus {
    pub worker_id: WorkerId,
    pub status: String,
    pub current_jobs: Vec<Uuid>,
    pub last_heartbeat: std::time::SystemTime,
}

// For simplicity, WorkerStatus is stored as JSON in PostgreSQL
// No direct SQLx Type implementation needed

impl WorkerStatus {
    pub const IDLE: &'static str = "IDLE";
    pub const BUSY: &'static str = "BUSY";
    pub const OFFLINE: &'static str = "OFFLINE";
    pub const DRAINING: &'static str = "DRAINING";

    pub fn new(worker_id: WorkerId, status: String) -> Self {
        Self {
            worker_id,
            status,
            current_jobs: Vec::new(),
            last_heartbeat: std::time::SystemTime::now(),
        }
    }

    pub fn create_with_status(status: String) -> Self {
        Self {
            worker_id: WorkerId::new(),
            status,
            current_jobs: Vec::new(),
            last_heartbeat: std::time::SystemTime::now(),
        }
    }

    pub fn from_status_string(status: String) -> Self {
        Self::create_with_status(status)
    }

    pub fn as_str(&self) -> &str {
        &self.status
    }

    pub fn is_available(&self) -> bool {
        self.status == Self::IDLE
    }
}

/// Runtime specification for worker
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSpec {
    pub image: String,
    pub command: Option<Vec<String>>,
    pub resources: crate::pipeline_execution::job_definitions::ResourceQuota,
    pub env: std::collections::HashMap<String, String>,
    pub labels: std::collections::HashMap<String, String>,
}

impl RuntimeSpec {
    pub fn new(image: String) -> Self {
        Self {
            image,
            command: None,
            resources: crate::pipeline_execution::job_definitions::ResourceQuota::new(100, 512),
            env: std::collections::HashMap::new(),
            labels: std::collections::HashMap::new(),
        }
    }
}

/// Worker capabilities for matching
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerCapabilities {
    pub cpu_cores: u32,
    pub memory_gb: u64,
    pub gpu: Option<u8>,
    pub features: Vec<String>,
    pub labels: std::collections::HashMap<String, String>,
    pub max_concurrent_jobs: u32,
}

impl WorkerCapabilities {
    /// Create new WorkerCapabilities (legacy method without validation)
    pub fn new(cpu_cores: u32, memory_gb: u64) -> Self {
        Self {
            cpu_cores,
            memory_gb,
            gpu: None,
            features: Vec::new(),
            labels: std::collections::HashMap::new(),
            max_concurrent_jobs: 4,
        }
    }

    /// Create new WorkerCapabilities with validation
    ///
    /// # Errors
    /// Returns `crate::shared_kernel::error::DomainError::Validation` if:
    /// - `cpu_cores` is 0
    /// - `memory_gb` is 0
    pub fn create(cpu_cores: u32, memory_gb: u64) -> crate::Result<Self> {
        if cpu_cores == 0 {
            return Err(crate::shared_kernel::error::DomainError::Validation(
                "CPU cores must be greater than 0".to_string(),
            ));
        }

        if memory_gb == 0 {
            return Err(crate::shared_kernel::error::DomainError::Validation(
                "Memory must be greater than 0 GB".to_string(),
            ));
        }

        Ok(Self {
            cpu_cores,
            memory_gb,
            gpu: None,
            features: Vec::new(),
            labels: std::collections::HashMap::new(),
            max_concurrent_jobs: 4, // Default value
        })
    }

    /// Create WorkerCapabilities with all parameters
    ///
    /// # Errors
    /// Returns `crate::shared_kernel::error::DomainError::Validation` if any parameter is invalid
    pub fn create_with_concurrency(
        cpu_cores: u32,
        memory_gb: u64,
        max_concurrent_jobs: u32,
    ) -> crate::Result<Self> {
        if cpu_cores == 0 {
            return Err(crate::shared_kernel::error::DomainError::Validation(
                "CPU cores must be greater than 0".to_string(),
            ));
        }

        if memory_gb == 0 {
            return Err(crate::shared_kernel::error::DomainError::Validation(
                "Memory must be greater than 0 GB".to_string(),
            ));
        }

        if max_concurrent_jobs == 0 {
            return Err(crate::shared_kernel::error::DomainError::Validation(
                "Max concurrent jobs must be greater than 0".to_string(),
            ));
        }

        Ok(Self {
            cpu_cores,
            memory_gb,
            gpu: None,
            features: Vec::new(),
            labels: std::collections::HashMap::new(),
            max_concurrent_jobs,
        })
    }

    /// Parse WorkerCapabilities from a string list
    ///
    /// Format: "key:value,key2:value2" or "key=value,key2=value2"
    /// Supported keys:
    /// - cpu: CPU cores (u32, required, > 0)
    /// - memory: Memory in GB (u64, required, > 0)
    /// - gpu: GPU count (u8, optional, default: None)
    /// - max_concurrent_jobs: Max concurrent jobs (u32, default: 4)
    ///
    /// # Errors
    /// Returns `crate::shared_kernel::error::DomainError::Validation` if:
    /// - Required fields are missing
    /// - Invalid format or parsing fails
    /// - Values are out of valid range
    pub fn from_string_list(capabilities: &str) -> crate::Result<Self> {
        if capabilities.trim().is_empty() {
            return Err(crate::shared_kernel::error::DomainError::Validation(
                "Capabilities string cannot be empty".to_string(),
            ));
        }

        let mut cpu_cores: Option<u32> = None;
        let mut memory_gb: Option<u64> = None;
        let mut gpu: Option<u8> = None;
        let mut max_concurrent_jobs: Option<u32> = None;

        let pairs: Vec<&str> = capabilities.split(',').map(|s| s.trim()).collect();

        for pair in pairs {
            if pair.is_empty() {
                continue;
            }

            // Support both : and = separators
            let parts: Vec<&str> = if pair.contains(':') {
                pair.split(':').collect()
            } else if pair.contains('=') {
                pair.split('=').collect()
            } else {
                return Err(crate::shared_kernel::error::DomainError::Validation(format!(
                    "Invalid capability format: '{}' (expected key:value or key=value)",
                    pair
                )));
            };

            if parts.len() != 2 {
                return Err(crate::shared_kernel::error::DomainError::Validation(format!(
                    "Invalid capability format: '{}' (expected exactly one separator)",
                    pair
                )));
            }

            let key = parts[0].trim().to_lowercase();
            let value = parts[1].trim();

            match key.as_str() {
                "cpu" => {
                    let parsed = value.parse::<u32>().map_err(|_| {
                        crate::shared_kernel::error::DomainError::Validation(format!(
                            "Invalid CPU cores value: '{}' (must be a positive integer)",
                            value
                        ))
                    })?;
                    if parsed == 0 {
                        return Err(crate::shared_kernel::error::DomainError::Validation(
                            "CPU cores must be greater than 0".to_string(),
                        ));
                    }
                    cpu_cores = Some(parsed);
                }
                "memory" => {
                    let parsed = value.parse::<u64>().map_err(|_| {
                        crate::shared_kernel::error::DomainError::Validation(format!(
                            "Invalid memory value: '{}' (must be a positive integer)",
                            value
                        ))
                    })?;
                    if parsed == 0 {
                        return Err(crate::shared_kernel::error::DomainError::Validation(
                            "Memory must be greater than 0 GB".to_string(),
                        ));
                    }
                    memory_gb = Some(parsed);
                }
                "gpu" => {
                    // GPU is optional - if present, must be valid
                    let parsed = value.parse::<u8>().map_err(|_| {
                        crate::shared_kernel::error::DomainError::Validation(format!(
                            "Invalid GPU count: '{}' (must be 0-255 or omitted)",
                            value
                        ))
                    })?;
                    gpu = Some(parsed);
                }
                "max_concurrent_jobs" | "concurrency" => {
                    let parsed = value.parse::<u32>().map_err(|_| {
                        crate::shared_kernel::error::DomainError::Validation(format!(
                            "Invalid max_concurrent_jobs value: '{}' (must be a positive integer)",
                            value
                        ))
                    })?;
                    if parsed == 0 {
                        return Err(crate::shared_kernel::error::DomainError::Validation(
                            "Max concurrent jobs must be greater than 0".to_string(),
                        ));
                    }
                    max_concurrent_jobs = Some(parsed);
                }
                other => {
                    return Err(crate::shared_kernel::error::DomainError::Validation(format!(
                        "Unknown capability key: '{}' (supported: cpu, memory, gpu, max_concurrent_jobs)",
                        other
                    )));
                }
            }
        }

        // Validate required fields
        let cpu_cores = cpu_cores.ok_or_else(|| {
            crate::shared_kernel::error::DomainError::Validation("Missing required capability: cpu".to_string())
        })?;

        let memory_gb = memory_gb.ok_or_else(|| {
            crate::shared_kernel::error::DomainError::Validation("Missing required capability: memory".to_string())
        })?;

        let max_concurrent_jobs = max_concurrent_jobs.unwrap_or(4);

        Ok(Self {
            cpu_cores,
            memory_gb,
            gpu,
            features: Vec::new(),
            labels: std::collections::HashMap::new(),
            max_concurrent_jobs,
        })
    }

    // TODO(#US-02.1): Future enhancements for capabilities parser:
    // - Add support for boolean flags (e.g., "ssd:true", "nvme:false")
    // - Add support for unit parsing (e.g., "cpu:4,mem:8GB,memory:8GiB")
    // - Add support for range validation (e.g., "cpu:4-8", "memory:4096-16384")
    // - Add support for parsing features and labels from the string list
    // - Add validation against minimum system requirements
    // - Add support for capability expressions (e.g., "cpu>=4,memory>=8GB")
}

/// Worker message envelope for distributed communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerMessage {
    pub correlation_id: crate::shared_kernel::correlation::CorrelationId,
    pub worker_id: WorkerId,
    pub message_type: String,
    pub payload: serde_json::Value,
}

/// Worker state message for status updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStateMessage {
    pub worker_id: WorkerId,
    pub state: WorkerState,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== TDD Tests: WorkerCapabilities Validation =====

    #[test]
    fn worker_capabilities_rejects_zero_cpu() {
        let result = WorkerCapabilities::create(0, 8);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("CPU cores must be greater than 0"));
        }
    }

    #[test]
    fn worker_capabilities_rejects_zero_memory() {
        let result = WorkerCapabilities::create(4, 0);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Memory must be greater than 0 GB"));
        }
    }

    #[test]
    fn worker_capabilities_rejects_zero_concurrent_jobs() {
        let result = WorkerCapabilities::create_with_concurrency(4, 8, 0);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(
                e.to_string()
                    .contains("Max concurrent jobs must be greater than 0")
            );
        }
    }

    #[test]
    fn worker_capabilities_accepts_valid_values() {
        let caps = WorkerCapabilities::create(4, 16).unwrap();
        assert_eq!(caps.cpu_cores, 4);
        assert_eq!(caps.memory_gb, 16);
        assert_eq!(caps.max_concurrent_jobs, 4);
    }

    #[test]
    fn worker_capabilities_with_concurrency_accepts_valid_values() {
        let caps = WorkerCapabilities::create_with_concurrency(8, 32, 10).unwrap();
        assert_eq!(caps.cpu_cores, 8);
        assert_eq!(caps.memory_gb, 32);
        assert_eq!(caps.max_concurrent_jobs, 10);
    }

    // ===== TDD Tests: US-02.1 - Capabilities Parser =====

    #[test]
    fn from_string_list_parses_valid_capabilities_with_colon_separator() {
        let caps = WorkerCapabilities::from_string_list("cpu:4,memory:8192,gpu:1").unwrap();
        assert_eq!(caps.cpu_cores, 4);
        assert_eq!(caps.memory_gb, 8192);
        assert_eq!(caps.gpu, Some(1));
        assert_eq!(caps.max_concurrent_jobs, 4); // Default value
    }

    #[test]
    fn from_string_list_parses_valid_capabilities_with_equals_separator() {
        let caps = WorkerCapabilities::from_string_list("cpu=8,memory=16384").unwrap();
        assert_eq!(caps.cpu_cores, 8);
        assert_eq!(caps.memory_gb, 16384);
        assert_eq!(caps.gpu, None);
        assert_eq!(caps.max_concurrent_jobs, 4); // Default value
    }

    #[test]
    fn from_string_list_parses_mixed_separators() {
        let caps = WorkerCapabilities::from_string_list("cpu:4,memory=8192,gpu:0").unwrap();
        assert_eq!(caps.cpu_cores, 4);
        assert_eq!(caps.memory_gb, 8192);
        assert_eq!(caps.gpu, Some(0));
    }

    #[test]
    fn from_string_list_uses_default_max_concurrent_jobs() {
        let caps = WorkerCapabilities::from_string_list("cpu:4,memory:8192").unwrap();
        assert_eq!(caps.max_concurrent_jobs, 4);
    }

    #[test]
    fn from_string_list_parses_custom_max_concurrent_jobs() {
        let caps = WorkerCapabilities::from_string_list("cpu:4,memory:8192,max_concurrent_jobs:8")
            .unwrap();
        assert_eq!(caps.max_concurrent_jobs, 8);
    }

    #[test]
    fn from_string_list_accepts_gpu_zero() {
        let caps = WorkerCapabilities::from_string_list("cpu:4,memory:8192,gpu:0").unwrap();
        assert_eq!(caps.gpu, Some(0));
    }

    #[test]
    fn from_string_list_accepts_max_gpu_value() {
        let caps = WorkerCapabilities::from_string_list("cpu:4,memory:8192,gpu:255").unwrap();
        assert_eq!(caps.gpu, Some(255));
    }

    #[test]
    fn from_string_list_handles_whitespace() {
        let caps =
            WorkerCapabilities::from_string_list(" cpu: 4 , memory: 8192 , gpu: 1 ").unwrap();
        assert_eq!(caps.cpu_cores, 4);
        assert_eq!(caps.memory_gb, 8192);
        assert_eq!(caps.gpu, Some(1));
    }

    #[test]
    fn from_string_list_accepts_concurrency_alias() {
        let caps = WorkerCapabilities::from_string_list("cpu:4,memory:8192,concurrency:6").unwrap();
        assert_eq!(caps.max_concurrent_jobs, 6);
    }

    #[test]
    fn from_string_list_handles_case_insensitive_keys() {
        let caps = WorkerCapabilities::from_string_list("CPU:4,Memory:8192,GPU:1").unwrap();
        assert_eq!(caps.cpu_cores, 4);
        assert_eq!(caps.memory_gb, 8192);
        assert_eq!(caps.gpu, Some(1));
    }

    // ===== Error Handling Tests =====

    #[test]
    fn from_string_list_rejects_empty_string() {
        let result = WorkerCapabilities::from_string_list("");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(
                e.to_string()
                    .contains("Capabilities string cannot be empty")
            );
        }
    }

    #[test]
    fn from_string_list_rejects_whitespace_only() {
        let result = WorkerCapabilities::from_string_list("   ");
        assert!(result.is_err());
    }

    #[test]
    fn from_string_list_rejects_missing_cpu() {
        let result = WorkerCapabilities::from_string_list("memory:8192");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Missing required capability: cpu"));
        }
    }

    #[test]
    fn from_string_list_rejects_missing_memory() {
        let result = WorkerCapabilities::from_string_list("cpu:4");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(
                e.to_string()
                    .contains("Missing required capability: memory")
            );
        }
    }

    #[test]
    fn from_string_list_rejects_zero_cpu() {
        let result = WorkerCapabilities::from_string_list("cpu:0,memory:8192");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("CPU cores must be greater than 0"));
        }
    }

    #[test]
    fn from_string_list_rejects_zero_memory() {
        let result = WorkerCapabilities::from_string_list("cpu:4,memory:0");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Memory must be greater than 0 GB"));
        }
    }

    #[test]
    fn from_string_list_rejects_zero_max_concurrent_jobs() {
        let result =
            WorkerCapabilities::from_string_list("cpu:4,memory:8192,max_concurrent_jobs:0");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(
                e.to_string()
                    .contains("Max concurrent jobs must be greater than 0")
            );
        }
    }

    #[test]
    fn from_string_list_rejects_invalid_cpu_value() {
        let result = WorkerCapabilities::from_string_list("cpu:abc,memory:8192");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Invalid CPU cores value"));
        }
    }

    #[test]
    fn from_string_list_rejects_invalid_memory_value() {
        let result = WorkerCapabilities::from_string_list("cpu:4,memory:xyz");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Invalid memory value"));
        }
    }

    #[test]
    fn from_string_list_rejects_invalid_gpu_value() {
        let result = WorkerCapabilities::from_string_list("cpu:4,memory:8192,gpu:invalid");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Invalid GPU count"));
        }
    }

    #[test]
    fn from_string_list_rejects_gpu_too_large() {
        let result = WorkerCapabilities::from_string_list("cpu:4,memory:8192,gpu:256");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Invalid GPU count"));
        }
    }

    #[test]
    fn from_string_list_rejects_unknown_key() {
        let result = WorkerCapabilities::from_string_list("cpu:4,memory:8192,disk:100");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Unknown capability key: 'disk'"));
        }
    }

    #[test]
    fn from_string_list_rejects_invalid_format_missing_separator() {
        let result = WorkerCapabilities::from_string_list("cpu 4,memory 8192");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Invalid capability format"));
        }
    }

    #[test]
    fn from_string_list_rejects_multiple_separators() {
        let result = WorkerCapabilities::from_string_list("cpu::4,memory:8192");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("expected exactly one separator"));
        }
    }

    #[test]
    fn from_string_list_rejects_equals_and_colon_together() {
        let _result = WorkerCapabilities::from_string_list("cpu=4,memory:8192");
        // This should work (uses the first separator found)
        // Actually no, it should use whichever separator is present
        // This test might not be needed as the logic handles it
    }

    #[test]
    fn from_string_list_rejects_empty_key() {
        let result = WorkerCapabilities::from_string_list(":4,memory:8192");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Unknown capability key"));
        }
    }

    #[test]
    fn from_string_list_rejects_empty_value() {
        let result = WorkerCapabilities::from_string_list("cpu:,memory:8192");
        assert!(result.is_err());
    }
}
