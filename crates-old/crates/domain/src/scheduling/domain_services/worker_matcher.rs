//! WorkerMatcher Domain Service
//! Filters and matches workers based on job requirements and resource availability

use crate::{Job, Worker};

/// Domain Service for matching workers to jobs
/// Handles filtering logic based on resource availability and capability matching
pub struct WorkerMatcher {
    // Configuration for matching criteria
    enable_strict_matching: bool,
}

impl WorkerMatcher {
    /// Create a new WorkerMatcher with default settings
    pub fn new() -> Self {
        Self {
            enable_strict_matching: false,
        }
    }

    /// Create with strict matching mode
    /// In strict mode, workers must exactly match all requirements
    pub fn with_strict_matching(enable: bool) -> Self {
        Self {
            enable_strict_matching: enable,
        }
    }

    /// Find all workers eligible for a given job
    /// Filters workers based on:
    /// 1. Availability status
    /// 2. CPU capacity
    /// 3. Memory capacity
    /// 4. Current load (optional)
    pub fn find_eligible_workers(&self, workers: &[Worker], job: &Job) -> Vec<Worker> {
        workers
            .iter()
            .filter(|worker| self.is_worker_eligible(worker, job))
            .cloned()
            .collect()
    }

    /// Check if a single worker is eligible for a job
    pub fn is_worker_eligible(&self, worker: &Worker, job: &Job) -> bool {
        // 1. Check availability
        if !worker.is_available() {
            return false;
        }

        // 2. Check CPU capacity
        let worker_cpu_m = u64::from(worker.capabilities.cpu_cores) * 1000;
        if worker_cpu_m < job.spec.resources.cpu_m {
            return false;
        }

        // 3. Check memory capacity
        let worker_memory_mb = worker.capabilities.memory_gb * 1024;
        if worker_memory_mb < job.spec.resources.memory_mb {
            return false;
        }

        // 4. Check GPU requirement if specified
        if let Some(required_gpu) = job.spec.resources.gpu {
            let worker_gpu = worker.capabilities.gpu.unwrap_or(0);
            if worker_gpu < required_gpu {
                return false;
            }
        }

        // 5. Check current load against max concurrent jobs
        let current_load = worker.current_jobs.len() as u32;
        if current_load >= worker.capabilities.max_concurrent_jobs {
            return false;
        }

        // 6. Check capability labels (optional matching)
        if !self.capability_labels_match(worker, job) {
            return false;
        }

        true
    }

    /// Check if worker's capability labels match job requirements
    /// Returns true if all job environment labels have matching worker labels
    fn capability_labels_match(&self, worker: &Worker, job: &Job) -> bool {
        if job.spec.env.is_empty() {
            return true; // No label requirements
        }

        if worker.capabilities.labels.is_empty() {
            // No worker labels, accept only in lenient mode
            return !self.enable_strict_matching;
        }

        // In strict mode, all job env keys must exist in worker labels
        if self.enable_strict_matching {
            return job
                .spec
                .env
                .keys()
                .all(|key| worker.capabilities.labels.contains_key(key));
        }

        // In lenient mode: if both job and worker have labels (regardless of overlap), accept
        // Lenient mode is very permissive - presence of labels on both sides is enough
        !job.spec.env.is_empty() && !worker.capabilities.labels.is_empty()
    }

    /// Get workers sorted by suitability score
    /// Returns workers sorted by a composite score of capacity and availability
    pub fn find_best_workers(&self, workers: &[Worker], job: &Job, limit: usize) -> Vec<Worker> {
        let eligible_workers = self.find_eligible_workers(workers, job);

        // Sort by composite score: (available_capacity / required_resources)
        let mut scored_workers: Vec<(Worker, f64)> = eligible_workers
            .into_iter()
            .map(|worker| {
                let score = self.calculate_suitability_score(&worker, job);
                (worker, score)
            })
            .collect();

        // Sort by score (descending)
        scored_workers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top N workers
        scored_workers
            .into_iter()
            .take(limit)
            .map(|(worker, _)| worker)
            .collect()
    }

    /// Calculate suitability score for a worker-job pair
    /// Higher score = better fit
    fn calculate_suitability_score(&self, worker: &Worker, job: &Job) -> f64 {
        let worker_cpu_m = u64::from(worker.capabilities.cpu_cores) * 1000;
        let worker_memory_mb = worker.capabilities.memory_gb * 1024;

        // Calculate spare capacity ratio
        let cpu_ratio = worker_cpu_m as f64 / job.spec.resources.cpu_m as f64;
        let memory_ratio = worker_memory_mb as f64 / job.spec.resources.memory_mb as f64;

        // Score based on how close to 1.0 (exact fit) the ratio is
        // Penalize both under-provisioning and over-provisioning
        let cpu_score = if cpu_ratio < 1.0 {
            cpu_ratio * 0.5 // Under-provisioned - low score
        } else if cpu_ratio <= 1.5 {
            1.5 - (cpu_ratio - 1.0) * 0.4 // Close to 1.0 - high score
        } else {
            1.5 - (cpu_ratio - 1.5) * 0.2 // Over-provisioned - penalty
        };

        let memory_score = if memory_ratio < 1.0 {
            memory_ratio * 0.5
        } else if memory_ratio <= 1.5 {
            1.5 - (memory_ratio - 1.0) * 0.4
        } else {
            1.5 - (memory_ratio - 1.5) * 0.2
        };

        // Current load factor (prefer less loaded workers)
        let current_load = worker.current_jobs.len() as f64;
        let max_load = worker.capabilities.max_concurrent_jobs as f64;
        let load_factor = 1.0 - (current_load / max_load).min(1.0);

        // Combined score
        (cpu_score * 0.5 + memory_score * 0.3 + load_factor * 0.2) * 100.0
    }

    /// Check if any worker in the list is eligible
    pub fn has_eligible_worker(&self, workers: &[Worker], job: &Job) -> bool {
        workers
            .iter()
            .any(|worker| self.is_worker_eligible(worker, job))
    }

    /// Count eligible workers
    pub fn count_eligible_workers(&self, workers: &[Worker], job: &Job) -> usize {
        workers
            .iter()
            .filter(|worker| self.is_worker_eligible(worker, job))
            .count()
    }

    /// Get the best single worker for a job
    pub fn find_best_worker(&self, workers: &[Worker], job: &Job) -> Option<Worker> {
        self.find_best_workers(workers, job, 1).first().cloned()
    }
}

impl Default for WorkerMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JobId, JobSpec, ResourceQuota, WorkerCapabilities, WorkerId};
    use std::collections::HashMap;

    fn create_test_worker(id: u8, cpu_cores: u32, memory_gb: u64, max_jobs: u32) -> Worker {
        let capabilities =
            WorkerCapabilities::create_with_concurrency(cpu_cores, memory_gb, max_jobs).unwrap();

        Worker::new(
            WorkerId::from_uuid(uuid::Uuid::from_bytes([
                id, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ])),
            format!("worker-{}", id),
            capabilities,
        )
    }

    fn create_test_job(cpu_m: u64, memory_mb: u64, gpu: Option<u8>) -> Job {
        let resources = if let Some(gpu_count) = gpu {
            ResourceQuota::create_with_gpu(cpu_m, memory_mb, gpu_count).unwrap()
        } else {
            ResourceQuota::create(cpu_m, memory_mb).unwrap()
        };

        let spec = JobSpec {
            name: "test-job".to_string(),
            image: "test:latest".to_string(),
            command: vec!["echo".to_string(), "test".to_string()],
            resources,
            timeout_ms: 300000,
            retries: 0,
            env: HashMap::new(),
            secret_refs: Vec::new(),
        };

        Job::create(JobId::new(), spec, None::<String>, None::<String>).unwrap()
    }

    // ===== TDD Tests: Worker Matching =====

    #[test]
    fn test_find_eligible_workers_basic() {
        let matcher = WorkerMatcher::new();

        let workers = vec![
            create_test_worker(1, 4, 8, 10),  // Eligible: 4 cores, 8GB
            create_test_worker(2, 2, 4, 10),  // Ineligible: 2 cores < 4 required
            create_test_worker(3, 8, 16, 10), // Eligible: 8 cores, 16GB
        ];

        let job = create_test_job(4000, 8192, None); // Requires 4 cores, 8GB

        let eligible = matcher.find_eligible_workers(&workers, &job);

        assert_eq!(eligible.len(), 2);
        assert!(eligible.iter().any(|w| w.name == "worker-1"));
        assert!(eligible.iter().any(|w| w.name == "worker-3"));
        assert!(!eligible.iter().any(|w| w.name == "worker-2"));
    }

    #[test]
    fn test_worker_not_available() {
        let matcher = WorkerMatcher::new();

        let mut worker = create_test_worker(1, 4, 8, 10);
        // Worker with current jobs at capacity
        worker.current_jobs = vec![uuid::Uuid::new_v4(); 10];

        let workers = vec![worker];
        let job = create_test_job(2000, 4096, None);

        let eligible = matcher.find_eligible_workers(&workers, &job);

        assert_eq!(eligible.len(), 0);
    }

    #[test]
    fn test_gpu_requirement_matching() {
        let matcher = WorkerMatcher::new();

        let mut worker1 = create_test_worker(1, 4, 8, 10);
        worker1.capabilities.gpu = Some(1);

        let mut worker2 = create_test_worker(2, 4, 8, 10);
        worker2.capabilities.gpu = Some(0);

        let workers = vec![worker1, worker2];
        let job = create_test_job(2000, 4096, Some(1)); // Requires 1 GPU

        let eligible = matcher.find_eligible_workers(&workers, &job);

        assert_eq!(eligible.len(), 1);
        assert!(eligible.iter().any(|w| w.name == "worker-1"));
    }

    #[test]
    fn test_strict_label_matching() {
        let matcher = WorkerMatcher::with_strict_matching(true);

        let mut worker = create_test_worker(1, 4, 8, 10);
        worker
            .capabilities
            .labels
            .insert("env".to_string(), "production".to_string());

        let workers = vec![worker];

        // Job with env label
        let job = {
            let mut spec = JobSpec {
                name: "test-job".to_string(),
                image: "test:latest".to_string(),
                command: vec!["echo".to_string(), "test".to_string()],
                resources: ResourceQuota::create(2000, 4096).unwrap(),
                timeout_ms: 300000,
                retries: 0,
                env: HashMap::new(),
                secret_refs: Vec::new(),
            };
            spec.env.insert("env".to_string(), "production".to_string());
            Job::create(JobId::new(), spec, None::<String>, None::<String>).unwrap()
        };

        let eligible = matcher.find_eligible_workers(&workers, &job);

        assert_eq!(eligible.len(), 1);

        // Test with missing label
        let job_no_match = {
            let mut spec = JobSpec {
                name: "test-job".to_string(),
                image: "test:latest".to_string(),
                command: vec!["echo".to_string(), "test".to_string()],
                resources: ResourceQuota::create(2000, 4096).unwrap(),
                timeout_ms: 300000,
                retries: 0,
                env: HashMap::new(),
                secret_refs: Vec::new(),
            };
            spec.env.insert("missing".to_string(), "label".to_string());
            Job::create(JobId::new(), spec, None::<String>, None::<String>).unwrap()
        };

        let eligible_no_match = matcher.find_eligible_workers(&workers, &job_no_match);
        assert_eq!(eligible_no_match.len(), 0);
    }

    #[test]
    fn test_lenient_label_matching() {
        let matcher = WorkerMatcher::new(); // Lenient mode

        let mut worker = create_test_worker(1, 4, 8, 10);
        worker
            .capabilities
            .labels
            .insert("env".to_string(), "production".to_string());

        let workers = vec![worker];

        // Job with different label - should still match in lenient mode
        let job = {
            let mut spec = JobSpec {
                name: "test-job".to_string(),
                image: "test:latest".to_string(),
                command: vec!["echo".to_string(), "test".to_string()],
                resources: ResourceQuota::create(2000, 4096).unwrap(),
                timeout_ms: 300000,
                retries: 0,
                env: HashMap::new(),
                secret_refs: Vec::new(),
            };
            spec.env
                .insert("different".to_string(), "label".to_string());
            Job::create(JobId::new(), spec, None::<String>, None::<String>).unwrap()
        };

        let eligible = matcher.find_eligible_workers(&workers, &job);

        // In lenient mode, any overlap (or no requirements) is enough
        assert_eq!(eligible.len(), 1);
    }

    #[test]
    fn test_find_best_worker() {
        let matcher = WorkerMatcher::new();

        // Worker 1: Over-provisioned (4 cores, 8GB)
        let worker1 = create_test_worker(1, 4, 8, 10);

        // Worker 2: Over-provisioned (16 cores, 32GB)
        let worker2 = create_test_worker(2, 16, 32, 10);

        // Worker 3: Exact fit (2 cores, 4GB) - should score highest
        let worker3 = create_test_worker(3, 2, 4, 10);

        let workers = vec![worker1, worker2, worker3];
        let job = create_test_job(2000, 4096, None);

        let best_worker = matcher.find_best_worker(&workers, &job);

        assert!(best_worker.is_some());
        // Worker 3 should be preferred (exact fit scores highest)
        assert_eq!(best_worker.unwrap().name, "worker-3");
    }

    #[test]
    fn test_has_eligible_worker() {
        let matcher = WorkerMatcher::new();

        let workers = vec![
            create_test_worker(1, 2, 4, 10),  // Insufficient resources
            create_test_worker(2, 8, 16, 10), // Eligible
        ];

        let job = create_test_job(4000, 8192, None);

        assert!(matcher.has_eligible_worker(&workers, &job));
        assert_eq!(matcher.count_eligible_workers(&workers, &job), 1);
    }

    #[test]
    fn test_no_eligible_workers() {
        let matcher = WorkerMatcher::new();

        let workers = vec![
            create_test_worker(1, 2, 4, 10),
            create_test_worker(2, 1, 2, 10),
        ];

        let job = create_test_job(4000, 8192, None);

        let eligible = matcher.find_eligible_workers(&workers, &job);

        assert!(eligible.is_empty());
        assert!(!matcher.has_eligible_worker(&workers, &job));
        assert_eq!(matcher.count_eligible_workers(&workers, &job), 0);
    }

    #[test]
    fn test_worker_at_capacity() {
        let matcher = WorkerMatcher::new();

        let mut worker = create_test_worker(1, 8, 16, 5);
        // Worker at full capacity
        worker.current_jobs = vec![
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
        ];

        let workers = vec![worker];
        let job = create_test_job(2000, 4096, None);

        let eligible = matcher.find_eligible_workers(&workers, &job);

        assert_eq!(eligible.len(), 0);
    }

    #[test]
    fn test_find_best_workers_with_limit() {
        let matcher = WorkerMatcher::new();

        let workers = vec![
            create_test_worker(1, 4, 8, 10),
            create_test_worker(2, 8, 16, 10),
            create_test_worker(3, 16, 32, 10),
            create_test_worker(4, 2, 4, 10), // Exact fit
        ];

        let job = create_test_job(2000, 4096, None);

        let best_three = matcher.find_best_workers(&workers, &job, 3);

        assert_eq!(best_three.len(), 3);
        // Worker 4 (exact fit) should be in top 3 due to best fit score
        assert!(best_three.iter().any(|w| w.name == "worker-4"));
    }
}
