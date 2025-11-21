//! Multiple Schedulers Support Module
//!
//! This module implements support for multiple scheduler instances that can run
//! simultaneously with different configurations, scopes, and specializations.

use crate::SchedulerError;
use crate::backend::SchedulerBackend;
use crate::pipeline::SchedulingPipeline;
use crate::queue::{PriorityQueue, QueueConfig, QueueStrategy};
use crate::selection::{SelectionStrategy, WorkerSelector};
use crate::types::*;
use std::collections::HashMap;
use std::sync::Arc;

/// Multiple scheduler configuration
#[derive(Debug, Clone)]
pub struct MultiSchedulerConfig {
    /// Default scheduler name
    pub default_scheduler: String,
    /// Whether to enable automatic fallback
    pub enable_fallback: bool,
    /// Maximum number of schedulers allowed
    pub max_schedulers: usize,
    /// Scheduler health check interval
    pub health_check_interval_ms: u64,
}

/// Scheduler instance definition
#[derive(Debug, Clone)]
pub struct SchedulerInstance {
    /// Unique scheduler name
    pub name: String,
    /// Scheduler configuration
    pub config: SchedulerInstanceConfig,
    /// Backend to use
    pub backend: Arc<dyn SchedulerBackend>,
    /// Pipeline for job processing
    pub pipeline: SchedulingPipeline,
    /// Selection strategy
    pub selector: WorkerSelector,
    /// Queue for jobs
    pub queue: PriorityQueue,
    /// Whether this scheduler is enabled
    pub enabled: bool,
    /// Current status
    pub status: SchedulerInstanceStatus,
}

/// Scheduler instance configuration
#[derive(Debug, Clone)]
pub struct SchedulerInstanceConfig {
    /// Queue strategy
    pub queue_strategy: QueueStrategy,
    /// Selection strategy
    pub selection_strategy: SelectionStrategy,
    /// Scheduler scope (cluster-wide, namespace-specific, etc.)
    pub scope: SchedulerScope,
    /// Specialization tags (e.g., "gpu", "high-priority", "batch")
    pub specializations: Vec<String>,
    /// Priority threshold (only handle jobs above this priority)
    pub priority_threshold: Option<JobPriority>,
    /// Namespace filter (if namespace-scoped)
    pub namespace_filter: Option<String>,
}

/// Scheduler scope defines where this scheduler operates
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerScope {
    /// Cluster-wide scheduler
    ClusterWide,
    /// Namespace-specific scheduler
    Namespace(String),
    /// Custom scope
    Custom(String),
}

/// Scheduler instance status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerInstanceStatus {
    /// Scheduler is initializing
    Initializing,
    /// Scheduler is running normally
    Running,
    /// Scheduler is in degraded mode
    Degraded,
    /// Scheduler is offline
    Offline,
    /// Scheduler has failed
    Failed(String),
}

/// Job routing result
#[derive(Debug, Clone)]
pub struct RoutingResult {
    /// Selected scheduler name
    pub selected_scheduler: String,
    /// Whether routing was successful
    pub success: bool,
    /// Reason for routing decision
    pub reason: String,
}

/// Scheduler registry for managing multiple instances
pub struct SchedulerRegistry {
    /// Registry configuration
    config: MultiSchedulerConfig,
    /// Active scheduler instances
    schedulers: HashMap<String, SchedulerInstance>,
    /// Default scheduler name
    default_scheduler: String,
}

impl SchedulerRegistry {
    /// Create new scheduler registry
    pub fn new(config: MultiSchedulerConfig) -> Self {
        Self {
            schedulers: HashMap::new(),
            default_scheduler: config.default_scheduler.clone(),
            config,
        }
    }

    /// Register a new scheduler instance
    pub fn register_scheduler(
        &mut self,
        instance: SchedulerInstance,
    ) -> Result<(), SchedulerError> {
        if self.schedulers.len() >= self.config.max_schedulers {
            return Err(SchedulerError::ConfigurationError(
                "Maximum number of schedulers reached".to_string(),
            ));
        }

        if self.schedulers.contains_key(&instance.name) {
            return Err(SchedulerError::ConfigurationError(format!(
                "Scheduler '{}' already registered",
                instance.name
            )));
        }

        self.schedulers.insert(instance.name.clone(), instance);

        Ok(())
    }

    /// Unregister a scheduler instance
    pub fn unregister_scheduler(&mut self, name: &str) -> Result<(), SchedulerError> {
        if !self.schedulers.contains_key(name) {
            return Err(SchedulerError::ConfigurationError(format!(
                "Scheduler '{}' not found",
                name
            )));
        }

        self.schedulers.remove(name);

        // Update default if necessary
        if name == &self.default_scheduler && !self.schedulers.is_empty() {
            if let Some(first_key) = self.schedulers.keys().next() {
                self.default_scheduler = first_key.clone();
            }
        }

        Ok(())
    }

    /// Route a job to an appropriate scheduler
    pub fn route_job(&self, job: &Job) -> RoutingResult {
        // First, try to find a specialized scheduler
        if let Some(specialized) = self.find_specialized_scheduler(job) {
            return RoutingResult {
                selected_scheduler: specialized,
                success: true,
                reason: "Specialized scheduler found".to_string(),
            };
        }

        // If namespace-scoped scheduler exists, use it
        if let Some(namespace_scheduler) = self.find_namespace_scheduler(job) {
            return RoutingResult {
                selected_scheduler: namespace_scheduler,
                success: true,
                reason: "Namespace-scoped scheduler found".to_string(),
            };
        }

        // Use default scheduler
        if self.schedulers.contains_key(&self.default_scheduler) {
            RoutingResult {
                selected_scheduler: self.default_scheduler.clone(),
                success: true,
                reason: "Using default scheduler".to_string(),
            }
        } else {
            RoutingResult {
                selected_scheduler: "".to_string(),
                success: false,
                reason: "No schedulers available".to_string(),
            }
        }
    }

    /// Find a specialized scheduler for the job
    fn find_specialized_scheduler(&self, job: &Job) -> Option<String> {
        // Check if any scheduler specializes in the job's requirements
        for (name, scheduler) in &self.schedulers {
            if !scheduler.enabled || scheduler.status != SchedulerInstanceStatus::Running {
                continue;
            }

            // Check specialization tags
            let has_matching_specialization =
                scheduler
                    .config
                    .specializations
                    .iter()
                    .any(|tag| match tag.as_str() {
                        "gpu" => job
                            .spec
                            .resource_requirements
                            .as_ref()
                            .and_then(|r| r.gpu_count)
                            .map(|gpu| gpu > 0)
                            .unwrap_or(false),
                        "high-priority" => {
                            if let Some(threshold) = &scheduler.config.priority_threshold {
                                job.spec.priority >= *threshold
                            } else {
                                matches!(
                                    job.spec.priority,
                                    JobPriority::High | JobPriority::Critical
                                )
                            }
                        }
                        _ => false,
                    });

            if has_matching_specialization {
                return Some(name.clone());
            }
        }

        None
    }

    /// Find a namespace-scoped scheduler for the job
    fn find_namespace_scheduler(&self, job: &Job) -> Option<String> {
        for (name, scheduler) in &self.schedulers {
            if !scheduler.enabled || scheduler.status != SchedulerInstanceStatus::Running {
                continue;
            }

            if let SchedulerScope::Namespace(ref namespace) = scheduler.config.scope {
                if namespace == &job.metadata.namespace {
                    return Some(name.clone());
                }
            }
        }

        None
    }

    /// Get a scheduler instance by name
    pub fn get_scheduler(&self, name: &str) -> Option<&SchedulerInstance> {
        self.schedulers.get(name)
    }

    /// Get all scheduler names
    pub fn list_schedulers(&self) -> Vec<String> {
        self.schedulers.keys().cloned().collect()
    }

    /// Check scheduler health
    pub fn check_health(&self, name: &str) -> Result<SchedulerInstanceStatus, SchedulerError> {
        self.schedulers
            .get(name)
            .map(|s| s.status.clone())
            .ok_or_else(|| {
                SchedulerError::ConfigurationError(format!("Scheduler '{}' not found", name))
            })
    }

    /// Update scheduler status
    pub fn update_status(
        &mut self,
        name: &str,
        status: SchedulerInstanceStatus,
    ) -> Result<(), SchedulerError> {
        if let Some(scheduler) = self.schedulers.get_mut(name) {
            scheduler.status = status;
            Ok(())
        } else {
            Err(SchedulerError::ConfigurationError(format!(
                "Scheduler '{}' not found",
                name
            )))
        }
    }

    /// Get the default scheduler name
    pub fn default_scheduler(&self) -> &str {
        &self.default_scheduler
    }

    /// Enable or disable a scheduler
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> Result<(), SchedulerError> {
        if let Some(scheduler) = self.schedulers.get_mut(name) {
            scheduler.enabled = enabled;
            Ok(())
        } else {
            Err(SchedulerError::ConfigurationError(format!(
                "Scheduler '{}' not found",
                name
            )))
        }
    }
}

/// Helper to create a default scheduler instance
pub fn create_default_scheduler(
    name: String,
    backend: Arc<dyn SchedulerBackend>,
) -> SchedulerInstance {
    let config = SchedulerInstanceConfig {
        queue_strategy: QueueStrategy::Priority {
            with_preemption: true,
            max_queue_time: None,
        },
        selection_strategy: SelectionStrategy::ResourceBalance,
        scope: SchedulerScope::ClusterWide,
        specializations: vec![],
        priority_threshold: None,
        namespace_filter: None,
    };

    SchedulerInstance {
        name,
        config,
        backend,
        pipeline: SchedulingPipeline::new(PipelineConfig::default()),
        selector: WorkerSelector::new(
            SelectionStrategy::ResourceBalance,
            SelectionCriteria::default(),
        ),
        queue: PriorityQueue::new(QueueConfig::default()),
        enabled: true,
        status: SchedulerInstanceStatus::Running,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MockBackend;

    fn create_test_job(priority: JobPriority, namespace: String) -> Job {
        Job {
            metadata: JobMetadata {
                id: uuid::Uuid::new_v4(),
                name: "test-job".to_string(),
                namespace,
                labels: HashMap::new(),
                created_at: chrono::Utc::now(),
            },
            spec: JobSpec {
                resource_requirements: Some(ResourceRequirements {
                    cpu_cores: Some(2.0),
                    memory_bytes: Some(2_000_000_000),
                    gpu_count: if priority == JobPriority::Critical {
                        Some(1)
                    } else {
                        None
                    },
                    ephemeral_storage: None,
                }),
                priority,
                node_selector: None,
                affinity: None,
                tolerations: vec![],
                max_retries: 3,
            },
        }
    }

    #[tokio::test]
    async fn test_scheduler_registration() {
        let config = MultiSchedulerConfig {
            default_scheduler: "default".to_string(),
            enable_fallback: true,
            max_schedulers: 10,
            health_check_interval_ms: 1000,
        };

        let mut registry = SchedulerRegistry::new(config);

        let backend = Arc::new(MockBackend::new());
        let scheduler = create_default_scheduler("default".to_string(), backend);

        registry.register_scheduler(scheduler).unwrap();

        assert_eq!(registry.list_schedulers(), vec!["default"]);
    }

    #[tokio::test]
    async fn test_job_routing_default() {
        let config = MultiSchedulerConfig {
            default_scheduler: "default".to_string(),
            enable_fallback: true,
            max_schedulers: 10,
            health_check_interval_ms: 1000,
        };

        let mut registry = SchedulerRegistry::new(config);

        let backend = Arc::new(MockBackend::new());
        let scheduler = create_default_scheduler("default".to_string(), backend);

        registry.register_scheduler(scheduler).unwrap();

        let job = create_test_job(JobPriority::Medium, "default".to_string());
        let result = registry.route_job(&job);

        assert!(result.success);
        assert_eq!(result.selected_scheduler, "default");
    }

    #[tokio::test]
    async fn test_job_routing_specialized() {
        let config = MultiSchedulerConfig {
            default_scheduler: "default".to_string(),
            enable_fallback: true,
            max_schedulers: 10,
            health_check_interval_ms: 1000,
        };

        let mut registry = SchedulerRegistry::new(config);

        // Register default scheduler
        let backend1 = Arc::new(MockBackend::new());
        let mut default_scheduler = create_default_scheduler("default".to_string(), backend1);
        default_scheduler.config.specializations = vec!["general".to_string()];
        registry.register_scheduler(default_scheduler).unwrap();

        // Register GPU scheduler
        let backend2 = Arc::new(MockBackend::new());
        let mut gpu_scheduler = create_default_scheduler("gpu".to_string(), backend2);
        gpu_scheduler.config.specializations = vec!["gpu".to_string()];
        registry.register_scheduler(gpu_scheduler).unwrap();

        // Route GPU job
        let gpu_job = create_test_job(JobPriority::High, "default".to_string());
        let result = registry.route_job(&gpu_job);

        assert!(result.success);
        assert_eq!(result.selected_scheduler, "gpu");
    }

    #[tokio::test]
    async fn test_namespace_routing() {
        let config = MultiSchedulerConfig {
            default_scheduler: "default".to_string(),
            enable_fallback: true,
            max_schedulers: 10,
            health_check_interval_ms: 1000,
        };

        let mut registry = SchedulerRegistry::new(config);

        // Register namespace-scoped scheduler
        let backend = Arc::new(MockBackend::new());
        let mut namespace_scheduler =
            create_default_scheduler("team-a-scheduler".to_string(), backend);
        namespace_scheduler.config.scope = SchedulerScope::Namespace("team-a".to_string());
        registry.register_scheduler(namespace_scheduler).unwrap();

        // Route job from team-a namespace
        let job = create_test_job(JobPriority::Medium, "team-a".to_string());
        let result = registry.route_job(&job);

        assert!(result.success);
        assert_eq!(result.selected_scheduler, "team-a-scheduler");
    }

    #[tokio::test]
    async fn test_scheduler_not_found_error() {
        let config = MultiSchedulerConfig {
            default_scheduler: "default".to_string(),
            enable_fallback: true,
            max_schedulers: 10,
            health_check_interval_ms: 1000,
        };

        let registry = SchedulerRegistry::new(config);

        let status = registry.check_health("nonexistent");

        assert!(status.is_err());
    }

    #[tokio::test]
    async fn test_duplicate_scheduler_error() {
        let config = MultiSchedulerConfig {
            default_scheduler: "default".to_string(),
            enable_fallback: true,
            max_schedulers: 10,
            health_check_interval_ms: 1000,
        };

        let mut registry = SchedulerRegistry::new(config);

        let backend = Arc::new(MockBackend::new());
        let scheduler = create_default_scheduler("default".to_string(), backend);

        registry.register_scheduler(scheduler.clone()).unwrap();
        let result = registry.register_scheduler(scheduler);

        assert!(result.is_err());
    }
}
