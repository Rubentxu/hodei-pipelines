/// Shared types for the SDK
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipelineConfig {
    /// Pipeline name
    pub name: String,

    /// Pipeline description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// List of stages in the pipeline
    pub stages: Vec<StageConfig>,

    /// Environment variables
    #[serde(default)]
    pub environment: HashMap<String, String>,

    /// Pipeline triggers
    #[serde(default)]
    pub triggers: Vec<TriggerConfig>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            stages: Vec::new(),
            environment: HashMap::new(),
            triggers: Vec::new(),
        }
    }
}

/// Stage configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StageConfig {
    /// Stage name
    pub name: String,

    /// Docker image to use
    pub image: String,

    /// Commands to execute
    pub commands: Vec<String>,

    /// Stage dependencies
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Stage-specific environment variables
    #[serde(default)]
    pub environment: HashMap<String, String>,

    /// Resource requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceRequirements>,
}

/// Resource requirements for a stage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceRequirements {
    /// CPU cores
    pub cpu: f32,

    /// Memory in MB
    pub memory: u32,

    /// Disk space in MB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk: Option<u32>,
}

/// Trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerConfig {
    /// Git push trigger
    GitPush { branch: String, repository: String },

    /// Scheduled trigger (cron)
    Schedule { cron: String },

    /// Manual trigger
    Manual,

    /// Webhook trigger
    Webhook { url: String },
}

/// Pipeline entity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pipeline {
    /// Pipeline unique identifier
    pub id: String,

    /// Pipeline name
    pub name: String,

    /// Pipeline status
    pub status: PipelineStatus,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Pipeline configuration
    pub config: PipelineConfig,
}

/// Pipeline status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PipelineStatus {
    /// Pipeline is pending execution
    Pending,

    /// Pipeline is currently running
    Running,

    /// Pipeline completed successfully
    Success,

    /// Pipeline failed
    Failed,

    /// Pipeline was cancelled
    Cancelled,
}

/// Job execution result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Job {
    /// Job unique identifier
    pub id: String,

    /// Pipeline ID this job belongs to
    pub pipeline_id: String,

    /// Job name
    pub name: String,

    /// Job status
    pub status: JobStatus,

    /// Start time
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// End time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Job status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobStatus {
    /// Job is queued
    Queued,

    /// Job is running
    Running,

    /// Job completed successfully
    Success,

    /// Job failed
    Failed,

    /// Job was cancelled
    Cancelled,
}

/// Worker information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Worker {
    /// Worker unique identifier
    pub id: String,

    /// Worker name
    pub name: String,

    /// Worker status
    pub status: WorkerStatus,

    /// Provider type
    pub provider: String,

    /// CPU usage (0.0 to 1.0)
    pub cpu_usage: f32,

    /// Memory usage (0.0 to 1.0)
    pub memory_usage: f32,

    /// Number of jobs processed
    pub jobs_processed: u64,
}

/// Worker status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkerStatus {
    /// Worker is active and ready
    Active,

    /// Worker is busy executing a job
    Busy,

    /// Worker is inactive
    Inactive,

    /// Worker has failed
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_config_serialization() {
        let config = PipelineConfig {
            name: "test-pipeline".to_string(),
            description: Some("Test pipeline".to_string()),
            stages: vec![StageConfig {
                name: "build".to_string(),
                image: "rust:1.70".to_string(),
                commands: vec!["cargo build".to_string()],
                dependencies: vec![],
                environment: HashMap::new(),
                resources: Some(ResourceRequirements {
                    cpu: 2.0,
                    memory: 2048,
                    disk: None,
                }),
            }],
            environment: HashMap::new(),
            triggers: vec![TriggerConfig::Manual],
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: PipelineConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_trigger_config_variants() {
        let trigger = TriggerConfig::GitPush {
            branch: "main".to_string(),
            repository: "https://github.com/test/repo".to_string(),
        };

        let json = serde_json::to_string(&trigger).unwrap();
        assert!(json.contains("git_push"));

        let deserialized: TriggerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(trigger, deserialized);
    }

    #[test]
    fn test_pipeline_status_serialization() {
        let status = PipelineStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"RUNNING\"");

        let deserialized: PipelineStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);
    }
}
