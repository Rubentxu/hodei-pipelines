//! Test data generators
//!
//! This module provides utilities for generating test data.

use rand::prelude::*;
use serde_json::{json, Value};
use uuid::Uuid;

/// Generator for test data
pub struct TestDataGenerator {
    rng: rand::ThreadRng,
}

impl TestDataGenerator {
    /// Create a new test data generator
    pub fn new() -> Self {
        Self {
            rng: rand::thread_rng(),
        }
    }

    /// Generate a unique test ID
    pub fn test_id(&mut self) -> String {
        Uuid::new_v4().to_string()
    }

    /// Generate a test pipeline
    pub fn create_pipeline(&mut self) -> Value {
        let id = self.test_id();

        json!({
            "id": id,
            "name": format!("test-pipeline-{}", self.rng.gen::<u32>()),
            "description": "Test pipeline for E2E testing",
            "steps": [
                {
                    "id": Uuid::new_v4().to_string(),
                    "name": "build",
                    "type": "build",
                    "config": {
                        "image": "rust:1.70",
                        "command": "cargo build"
                    }
                },
                {
                    "id": Uuid::new_v4().to_string(),
                    "name": "test",
                    "type": "test",
                    "config": {
                        "image": "rust:1.70",
                        "command": "cargo test"
                    },
                    "depends_on": ["build"]
                }
            ],
            "triggers": ["manual"],
            "timeout": 300,
            "created_at": chrono::Utc::now()
        })
    }

    /// Generate a test job
    pub fn create_job(&mut self, pipeline_id: Option<String>) -> Value {
        json!({
            "id": self.test_id(),
            "pipeline_id": pipeline_id.unwrap_or_else(|| self.test_id()),
            "name": format!("test-job-{}", self.rng.gen::<u32>()),
            "priority": "normal",
            "status": "pending",
            "config": {
                "retry_count": 3,
                "timeout": 300
            },
            "created_at": chrono::Utc::now()
        })
    }

    /// Generate a test worker configuration
    pub fn create_worker(&mut self, worker_type: &str) -> Value {
        json!({
            "id": self.test_id(),
            "type": worker_type,
            "name": format!("{}-worker-{}", worker_type, self.rng.gen::<u32>()),
            "config": {
                "image": format!("{}-worker:latest", worker_type),
                "resources": {
                    "cpu": "500m",
                    "memory": "512Mi"
                }
            },
            "created_at": chrono::Utc::now()
        })
    }

    /// Generate a test schedule
    pub fn create_schedule(&mut self, job_id: Option<String>) -> Value {
        json!({
            "id": self.test_id(),
            "job_id": job_id.unwrap_or_else(|| self.test_id()),
            "cron": "0 0 * * *",
            "timezone": "UTC",
            "enabled": true,
            "created_at": chrono::Utc::now()
        })
    }

    /// Generate random pipeline names
    pub fn random_pipeline_name(&mut self) -> String {
        let prefixes = ["build", "deploy", "test", "release"];
        let suffix = self.rng.gen::<u32>();

        format!(
            "{}-pipeline-{}",
            self.rng.choose(&prefixes).unwrap(),
            suffix
        )
    }

    /// Generate random job names
    pub fn random_job_name(&mut self) -> String {
        let prefixes = ["build", "run", "execute", "process"];
        let suffix = self.rng.gen::<u32>();

        format!("{}-job-{}", self.rng.choose(&prefixes).unwrap(), suffix)
    }

    /// Generate a complete test scenario
    pub fn create_scenario(&mut self) -> Value {
        let pipeline = self.create_pipeline();
        let pipeline_id = pipeline["id"].as_str().unwrap().to_string();

        json!({
            "pipeline": pipeline,
            "jobs": vec![
                self.create_job(Some(pipeline_id.clone())),
                self.create_job(Some(pipeline_id.clone()))
            ],
            "workers": vec![
                self.create_worker("rust"),
                self.create_worker("python")
            ],
            "schedules": vec![
                self.create_schedule(None)
            ]
        })
    }
}

impl Default for TestDataGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pipeline() {
        let mut gen = TestDataGenerator::new();
        let pipeline = gen.create_pipeline();

        assert!(pipeline.get("id").is_some());
        assert!(pipeline.get("name").is_some());
        assert!(pipeline.get("steps").is_some());
    }

    #[test]
    fn test_generate_job() {
        let mut gen = TestDataGenerator::new();
        let job = gen.create_job(None);

        assert!(job.get("id").is_some());
        assert!(job.get("pipeline_id").is_some());
    }

    #[test]
    fn test_generate_worker() {
        let mut gen = TestDataGenerator::new();
        let worker = gen.create_worker("rust");

        assert!(worker.get("id").is_some());
        assert_eq!(worker["type"], "rust");
    }
}
