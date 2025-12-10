//! Happy path scenario
//!
//! This module implements the happy path test scenario that validates
//! the complete workflow of the distributed job orchestration platform.

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::TestResult;
use crate::helpers::{HttpClient, TestDataGenerator};
use crate::infrastructure::{
    OrchestratorClient, SchedulerClient, TestEnvironment, WorkerManagerClient,
};
use crate::scenarios::{Scenario, ScenarioResult};

/// Happy path test scenario
pub struct HappyPathScenario {
    generator: TestDataGenerator,
}

impl HappyPathScenario {
    /// Create a new happy path scenario
    pub fn new() -> Self {
        Self {
            generator: TestDataGenerator::new(),
        }
    }
}

#[async_trait]
impl Scenario for HappyPathScenario {
    fn name(&self) -> &'static str {
        "happy_path"
    }

    fn description(&self) -> &'static str {
        "Test the complete workflow: create pipeline, schedule job, execute with worker"
    }

    async fn run(&self, env: &TestEnvironment) -> TestResult<ScenarioResult> {
        // Initialize HTTP clients
        let orchestrator_client = OrchestratorClient::new(env.config.orchestrator_url());
        let scheduler_client = SchedulerClient::new(env.config.scheduler_url());
        let worker_client = WorkerManagerClient::new(env.config.worker_manager_url());

        let mut metrics = std::collections::HashMap::new();
        let start_time = std::time::Instant::now();

        println!("  1️⃣  Creating a pipeline...");
        let pipeline = orchestrator_client
            .create_pipeline("test-pipeline", "Happy path test pipeline")
            .await?;
        let pipeline_id = pipeline["id"].as_str().unwrap().to_string();

        println!("  2️⃣  Listing pipelines to verify creation...");
        let pipelines = orchestrator_client.list_pipelines().await?;
        assert!(pipelines.as_array().unwrap().len() > 0);

        println!("  3️⃣  Creating a job for the pipeline...");
        let job = orchestrator_client.create_job(&pipeline_id).await?;
        let job_id = job["id"].as_str().unwrap().to_string();

        println!("  4️⃣  Listing jobs to verify creation...");
        let jobs = orchestrator_client.list_jobs().await?;
        assert!(jobs.as_array().unwrap().len() > 0);

        println!("  5️⃣  Scheduling the job...");
        let schedule = scheduler_client
            .schedule_job(&job_id, "2025-01-01T00:00:00Z")
            .await?;

        println!("  6️⃣  Listing scheduled jobs...");
        let scheduled_jobs = scheduler_client.list_scheduled_jobs().await?;

        println!("  7️⃣  Registering a worker...");
        let worker = scheduler_client.register_worker("rust").await?;
        let worker_id = worker["id"].as_str().unwrap().to_string();

        println!("  8️⃣  Starting a worker in the worker manager...");
        let started_worker = worker_client.start_worker("rust").await?;

        println!("  9️⃣  Listing workers...");
        let workers = worker_client.list_workers().await?;
        assert!(workers.as_array().unwrap().len() > 0);

        println!(" 🔟  Executing a job...");
        let execution = worker_client.execute_job(&job_id).await?;
        let execution_id = execution["id"].as_str().unwrap().to_string();

        println!(" 1️⃣1️⃣  Listing executions...");
        let executions = worker_client.list_executions().await?;
        assert!(executions.as_array().unwrap().len() > 0);

        // Collect metrics
        metrics.insert("pipeline_id".to_string(), pipeline_id);
        metrics.insert("job_id".to_string(), job_id);
        metrics.insert("execution_id".to_string(), execution_id);
        metrics.insert(
            "workers_count".to_string(),
            workers.as_array().unwrap().len().to_string(),
        );
        metrics.insert(
            "pipelines_count".to_string(),
            pipelines.as_array().unwrap().len().to_string(),
        );

        // Verify all operations completed successfully
        assert!(pipeline.get("id").is_some());
        assert!(job.get("id").is_some());
        assert!(schedule.get("id").is_some());
        assert!(worker.get("id").is_some());
        assert!(execution.get("id").is_some());

        let duration = start_time.elapsed().as_millis() as u64;

        Ok(ScenarioResult::success(
            self.name().to_string(),
            duration,
            "Complete workflow executed successfully".to_string(),
        )
        .with_metrics(metrics))
    }
}

impl Default for HappyPathScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_happy_path_scenario_creation() {
        let scenario = HappyPathScenario::new();
        assert_eq!(scenario.name(), "happy_path");
    }
}
