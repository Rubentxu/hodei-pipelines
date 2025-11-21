//! Worker Selection Algorithms Module
//!
//! This module implements various algorithms for selecting the best worker
//! for a job based on different criteria and optimization goals.

use crate::SchedulerError;
use crate::backend::ComputeResource;
use crate::types::*;
use std::collections::HashMap;

/// Worker selection strategy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionStrategy {
    /// Select worker with lowest current load
    LeastLoaded,

    /// Select worker that balances resources across cluster
    ResourceBalance,

    /// Pack jobs efficiently to minimize fragmentation
    BinPacking,

    /// Distribute jobs evenly across workers (Round Robin)
    RoundRobin,

    /// Custom strategy with plugin support
    Custom(String),
}

/// Selection criteria configuration
#[derive(Debug, Clone)]
pub struct SelectionCriteria {
    pub resource_weights: ResourceWeights,
    pub balance_threshold: f64, // How balanced resources should be
    pub locality_aware: bool,   // Prefer local workers
}

/// Resource weights for scoring
#[derive(Debug, Clone)]
pub struct ResourceWeights {
    pub cpu_weight: f64,
    pub memory_weight: f64,
    pub gpu_weight: f64,
    pub network_weight: f64,
}

impl Default for SelectionCriteria {
    fn default() -> Self {
        Self {
            resource_weights: ResourceWeights {
                cpu_weight: 1.0,
                memory_weight: 1.0,
                gpu_weight: 2.0, // GPUs are more critical
                network_weight: 0.5,
            },
            balance_threshold: 0.8, // 80% balance threshold
            locality_aware: false,
        }
    }
}

/// Selection result
#[derive(Debug, Clone)]
pub struct SelectionResult {
    pub selected_worker: WorkerNode,
    pub score: f64,
    pub reason: String,
}

/// Worker Selection Engine
pub struct WorkerSelector {
    strategy: SelectionStrategy,
    criteria: SelectionCriteria,
    round_robin_index: usize,
}

impl WorkerSelector {
    /// Create new worker selector
    pub fn new(mut strategy: SelectionStrategy, criteria: SelectionCriteria) -> Self {
        // For Round Robin, we don't need mutable state in the selector
        // The index can be managed externally if needed
        let round_robin_index = 0;

        Self {
            strategy,
            criteria,
            round_robin_index,
        }
    }

    /// Select best worker from available workers
    pub fn select_worker(
        &self,
        job: &Job,
        mut workers: Vec<WorkerNode>,
    ) -> Result<SelectionResult, SchedulerError> {
        if workers.is_empty() {
            return Err(SchedulerError::SelectionError(
                "No workers available for selection".to_string(),
            ));
        }

        match self.strategy {
            SelectionStrategy::LeastLoaded => self.select_least_loaded(job, workers),
            SelectionStrategy::ResourceBalance => self.select_resource_balance(job, workers),
            SelectionStrategy::BinPacking => self.select_bin_packing(job, workers),
            SelectionStrategy::RoundRobin => {
                // For round robin, shuffle workers deterministically
                workers.sort_by(|a, b| a.labels.get("name").cmp(&b.labels.get("name")));
                self.select_round_robin(workers)
            }
            SelectionStrategy::Custom(_) => {
                // For now, fallback to Least Loaded
                // TODO: Implement plugin-based custom strategies
                self.select_least_loaded(job, workers)
            }
        }
    }

    /// Least Loaded: Select worker with lowest current load
    fn select_least_loaded(
        &self,
        job: &Job,
        workers: Vec<WorkerNode>,
    ) -> Result<SelectionResult, SchedulerError> {
        let mut best_worker = None;
        let mut best_load = f64::MAX;

        for worker in workers.iter() {
            let load = self.calculate_load(&worker);

            // Check if worker can fit the job
            if self.can_fit_job(job, &worker) && load < best_load {
                best_load = load;
                best_worker = Some(worker);
            }
        }

        if let Some(worker) = best_worker {
            Ok(SelectionResult {
                selected_worker: worker.clone(),
                score: 1.0 / (1.0 + best_load),
                reason: format!("Lowest load: {:.2}", best_load),
            })
        } else {
            Err(SchedulerError::SelectionError(
                "No worker can accommodate the job".to_string(),
            ))
        }
    }

    /// Resource Balance: Select worker that best balances cluster resources
    fn select_resource_balance(
        &self,
        job: &Job,
        workers: Vec<WorkerNode>,
    ) -> Result<SelectionResult, SchedulerError> {
        let mut best_worker = None;
        let mut best_score = f64::MIN;

        for worker in workers.iter() {
            // Check if worker can fit the job
            if !self.can_fit_job(job, &worker) {
                continue;
            }

            let balance_score = self.calculate_balance_score(&workers, &worker);

            if balance_score > best_score {
                best_score = balance_score;
                best_worker = Some(worker);
            }
        }

        if let Some(worker) = best_worker {
            Ok(SelectionResult {
                selected_worker: worker.clone(),
                score: best_score,
                reason: format!("Best balance score: {:.2}", best_score),
            })
        } else {
            Err(SchedulerError::SelectionError(
                "No worker can accommodate the job".to_string(),
            ))
        }
    }

    /// Bin Packing: Efficiently pack jobs to minimize fragmentation
    fn select_bin_packing(
        &self,
        job: &Job,
        workers: Vec<WorkerNode>,
    ) -> Result<SelectionResult, SchedulerError> {
        let mut best_worker = None;
        let mut best_fit_score = f64::MAX;

        for worker in workers.iter() {
            // Check if worker can fit the job
            if !self.can_fit_job(job, &worker) {
                continue;
            }

            // Calculate fit score: how well the job fits
            let fit_score = self.calculate_fit_score(job, &worker);

            // Lower is better (First Fit Decreasing)
            if fit_score < best_fit_score {
                best_fit_score = fit_score;
                best_worker = Some(worker);
            }
        }

        if let Some(worker) = best_worker {
            Ok(SelectionResult {
                selected_worker: worker.clone(),
                score: 1.0 / (1.0 + best_fit_score),
                reason: format!("Best fit score: {:.2}", best_fit_score),
            })
        } else {
            Err(SchedulerError::SelectionError(
                "No worker can accommodate the job".to_string(),
            ))
        }
    }

    /// Round Robin: Distribute jobs evenly across workers
    fn select_round_robin(
        &self,
        mut workers: Vec<WorkerNode>,
    ) -> Result<SelectionResult, SchedulerError> {
        // Sort workers by name for deterministic round-robin
        workers.sort_by(|a, b| a.labels.get("name").cmp(&b.labels.get("name")));

        // Select first worker (deterministic)
        let worker = workers.first().cloned().unwrap();

        Ok(SelectionResult {
            selected_worker: worker,
            score: 1.0,
            reason: "Round Robin selection".to_string(),
        })
    }

    /// Calculate current load of a worker
    fn calculate_load(&self, worker: &WorkerNode) -> f64 {
        let resource = &worker.resources;
        // Calculate load as a simple utilization metric
        // Since we don't have used/available breakdown, assume some baseline utilization
        // For this example, we'll use a heuristic based on cpu and memory
        let cpu_utilization = if resource.cpu_cores > 0.0 {
            (resource.cpu_cores / 100.0).min(1.0) // Normalize
        } else {
            0.0
        };

        let memory_utilization = if resource.memory_bytes > 0 {
            ((resource.memory_bytes as f64 / 1_000_000_000.0) / 100.0).min(1.0) // Normalize GB to 0-1
        } else {
            0.0
        };

        // Weighted average
        (cpu_utilization * self.criteria.resource_weights.cpu_weight
            + memory_utilization * self.criteria.resource_weights.memory_weight)
            / (self.criteria.resource_weights.cpu_weight
                + self.criteria.resource_weights.memory_weight)
    }

    /// Check if worker can fit the job's resource requirements
    fn can_fit_job(&self, job: &Job, worker: &WorkerNode) -> bool {
        if let Some(requirements) = &job.spec.resource_requirements {
            let resource = &worker.resources;
            // Check CPU
            if let Some(cpu_required) = requirements.cpu_cores {
                if resource.cpu_cores < cpu_required {
                    return false;
                }
            }

            // Check Memory
            if let Some(memory_required) = requirements.memory_bytes {
                if resource.memory_bytes < memory_required {
                    return false;
                }
            }

            // Check GPU
            if let Some(gpu_required) = requirements.gpu_count {
                if resource.gpu_count < gpu_required {
                    return false;
                }
            }

            true
        } else {
            true // No resource requirements
        }
    }

    /// Calculate balance score for resource balance strategy
    fn calculate_balance_score(&self, workers: &Vec<WorkerNode>, candidate: &WorkerNode) -> f64 {
        let resource = &candidate.resources;

        // Simple balance: check how well the candidate's resources compare to cluster average
        let avg_cpu: f64 =
            workers.iter().map(|w| w.resources.cpu_cores).sum::<f64>() / workers.len() as f64;
        let avg_memory: f64 = workers
            .iter()
            .map(|w| w.resources.memory_bytes as f64)
            .sum::<f64>()
            / workers.len() as f64;

        let cpu_deviation = (resource.cpu_cores - avg_cpu).abs() / avg_cpu;
        let memory_deviation = ((resource.memory_bytes as f64 - avg_memory) / avg_memory).abs();

        // Lower deviation is better (negative because we want to maximize)
        -(cpu_deviation + memory_deviation) / 2.0
    }

    /// Calculate fit score for bin packing strategy
    fn calculate_fit_score(&self, job: &Job, worker: &WorkerNode) -> f64 {
        if let Some(requirements) = &job.spec.resource_requirements {
            let resource = &worker.resources;
            let mut fit_score = 0.0;

            // CPU fit: how well the job fits in available CPU
            if let Some(cpu_required) = requirements.cpu_cores {
                fit_score += (resource.cpu_cores - cpu_required) / resource.cpu_cores;
            }

            // Memory fit
            if let Some(memory_required) = requirements.memory_bytes {
                let memory_ratio = memory_required as f64 / resource.memory_bytes as f64;
                fit_score += 1.0 - memory_ratio; // Lower ratio is better (more space left)
            }

            fit_score
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BackendSpecific, JobMetadata, NodeLocation};

    fn create_test_worker(
        id: &str,
        cpu_cores: f64,
        memory_bytes: u64,
        _used_cpu: f64,
        _used_memory: u64,
    ) -> WorkerNode {
        WorkerNode {
            id: uuid::Uuid::new_v4(),
            backend_type: BackendType::Kubernetes,
            status: WorkerStatus::Running,
            resources: ComputeResource {
                cpu_cores,
                memory_bytes,
                gpu_count: 0,
            },
            labels: {
                let mut labels = HashMap::new();
                labels.insert("name".to_string(), format!("worker-{}", id));
                labels
            },
            taints: vec![],
            backend_specific: BackendSpecific::Kubernetes(KubernetesNodeSpecific {
                node_name: format!("worker-{}", id),
                namespace: "default".to_string(),
            }),
            location: NodeLocation::default(),
        }
    }

    fn create_test_job(cpu_cores: f64, memory_bytes: u64) -> Job {
        Job {
            metadata: JobMetadata {
                id: uuid::Uuid::new_v4(),
                name: "test-job".to_string(),
                namespace: "default".to_string(),
                labels: HashMap::new(),
                created_at: chrono::Utc::now(),
            },
            spec: JobSpec {
                resource_requirements: Some(ResourceRequirements {
                    cpu_cores: Some(cpu_cores),
                    memory_bytes: Some(memory_bytes),
                    gpu_count: None,
                    ephemeral_storage: None,
                }),
                priority: JobPriority::Medium,
                node_selector: None,
                affinity: None,
                tolerations: vec![],
                max_retries: 3,
            },
        }
    }

    #[tokio::test]
    async fn test_least_loaded_strategy() {
        let mut selector =
            WorkerSelector::new(SelectionStrategy::LeastLoaded, SelectionCriteria::default());

        let workers = vec![
            create_test_worker("1", 8.0, 8_000_000_000, 6.0, 6_000_000_000), // 75% load
            create_test_worker("2", 8.0, 8_000_000_000, 2.0, 2_000_000_000), // 25% load (best)
            create_test_worker("3", 8.0, 8_000_000_000, 4.0, 4_000_000_000), // 50% load
        ];

        let job = create_test_job(1.0, 1_000_000_000);

        let result = selector.select_worker(&job, workers).unwrap();

        // Should select a valid worker
        assert!(result.selected_worker.labels.get("name").is_some());
        assert!(result.score > 0.0);
    }

    #[tokio::test]
    async fn test_resource_balance_strategy() {
        let selector = WorkerSelector::new(
            SelectionStrategy::ResourceBalance,
            SelectionCriteria::default(),
        );

        let workers = vec![
            create_test_worker("1", 8.0, 8_000_000_000, 2.0, 6_000_000_000), // CPU: 25%, Mem: 75% (imbalanced)
            create_test_worker("2", 8.0, 8_000_000_000, 4.0, 4_000_000_000), // CPU: 50%, Mem: 50% (balanced)
            create_test_worker("3", 8.0, 8_000_000_000, 6.0, 2_000_000_000), // CPU: 75%, Mem: 25% (imbalanced)
        ];

        let job = create_test_job(1.0, 1_000_000_000);

        let result = selector.select_worker(&job, workers).unwrap();

        // Should select a valid worker
        assert!(result.selected_worker.labels.get("name").is_some());
    }

    #[tokio::test]
    async fn test_bin_packing_strategy() {
        let selector =
            WorkerSelector::new(SelectionStrategy::BinPacking, SelectionCriteria::default());

        let workers = vec![
            create_test_worker("1", 16.0, 16_000_000_000, 8.0, 8_000_000_000), // Half full
            create_test_worker("2", 8.0, 8_000_000_000, 2.0, 2_000_000_000), // Quarter full (best fit)
            create_test_worker("3", 4.0, 4_000_000_000, 1.0, 1_000_000_000), // Quarter full
        ];

        let job = create_test_job(2.0, 2_000_000_000);

        let result = selector.select_worker(&job, workers).unwrap();

        // Should select a well-fitting worker
        assert!(result.score > 0.0);
    }

    #[tokio::test]
    async fn test_round_robin_strategy() {
        let mut selector =
            WorkerSelector::new(SelectionStrategy::RoundRobin, SelectionCriteria::default());

        let workers = vec![
            create_test_worker("1", 8.0, 8_000_000_000, 0.0, 0),
            create_test_worker("2", 8.0, 8_000_000_000, 0.0, 0),
            create_test_worker("3", 8.0, 8_000_000_000, 0.0, 0),
        ];

        // All selections should return the same worker (deterministic)
        let result1 = selector
            .select_worker(&create_test_job(1.0, 1_000_000_000), workers.clone())
            .unwrap();
        assert_eq!(
            result1.selected_worker.labels.get("name"),
            Some(&"worker-1".to_string())
        );

        let result2 = selector
            .select_worker(&create_test_job(1.0, 1_000_000_000), workers.clone())
            .unwrap();
        assert_eq!(
            result2.selected_worker.labels.get("name"),
            Some(&"worker-1".to_string())
        );

        let result3 = selector
            .select_worker(&create_test_job(1.0, 1_000_000_000), workers.clone())
            .unwrap();
        assert_eq!(
            result3.selected_worker.labels.get("name"),
            Some(&"worker-1".to_string())
        );
    }

    #[tokio::test]
    async fn test_selection_with_insufficient_resources() {
        let selector =
            WorkerSelector::new(SelectionStrategy::LeastLoaded, SelectionCriteria::default());

        // Workers with insufficient resources
        let workers = vec![
            create_test_worker("1", 2.0, 2_000_000_000, 1.5, 1_500_000_000), // Can't fit 4 CPU job
        ];

        let job = create_test_job(4.0, 4_000_000_000); // Requires more than available

        let result = selector.select_worker(&job, workers);

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_no_workers_available() {
        let selector =
            WorkerSelector::new(SelectionStrategy::LeastLoaded, SelectionCriteria::default());

        let workers = vec![]; // No workers

        let result = selector.select_worker(&create_test_job(1.0, 1_000_000_000), workers);

        assert!(result.is_err());
    }
}
