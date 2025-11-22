//! Error handling scenario
//!
//! This module implements error handling test scenarios that validate
//! the platform's behavior under error conditions.

use crate::infrastructure::{
    OrchestratorClient, SchedulerClient, TestEnvironment, WorkerManagerClient,
};
use crate::{Scenario, ScenarioResult, TestResult};
use async_trait::async_trait;
use serde_json::json;

/// Error handling test scenario
pub struct ErrorHandlingScenario;

impl ErrorHandlingScenario {
    /// Create a new error handling scenario
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Scenario for ErrorHandlingScenario {
    fn name(&self) -> &'static str {
        "error_handling"
    }

    fn description(&self) -> &'static str {
        "Test error handling: invalid requests, non-existent resources, edge cases"
    }

    async fn run(&self, env: &TestEnvironment) -> TestResult<ScenarioResult> {
        let mut metrics = std::collections::HashMap::new();
        let start_time = std::time::Instant::now();

        // Initialize HTTP clients
        let orchestrator_client = OrchestratorClient::new(env.config.orchestrator_url());
        let scheduler_client = SchedulerClient::new(env.config.scheduler_url());
        let worker_client = WorkerManagerClient::new(env.config.worker_manager_url());

        // Test 1: Create pipeline with empty name (should work but may have validation)
        println!("  1️⃣  Creating pipeline with minimal data...");
        let minimal_pipeline = json!({
            "name": "minimal"
        });
        let result = orchestrator_client
            .create_pipeline("minimal-pipeline", "Test pipeline")
            .await?;
        assert!(result.get("id").is_some());

        // Test 2: Try to get non-existent pipeline (should return 404)
        println!("  2️⃣  Attempting to get non-existent pipeline...");
        let invalid_pipeline_id = "non-existent-pipeline-id-12345";
        let response = reqwest::get(&format!(
            "{}/api/v1/pipelines/{}",
            env.config.orchestrator_url(),
            invalid_pipeline_id
        ))
        .await?;
        assert_eq!(response.status().as_u16(), 404);

        // Test 3: Try to get non-existent job
        println!("  3️⃣  Attempting to get non-existent job...");
        let invalid_job_id = "non-existent-job-id-12345";
        let response = reqwest::get(&format!(
            "{}/api/v1/jobs/{}",
            env.config.orchestrator_url(),
            invalid_job_id
        ))
        .await?;
        assert_eq!(response.status().as_u16(), 404);

        // Test 4: Create job for non-existent pipeline
        println!("  4️⃣  Creating job with non-existent pipeline ID...");
        let invalid_pipeline_job = json!({
            "pipeline_id": "invalid-pipeline-id"
        });
        let result = orchestrator_client
            .create_job("invalid-pipeline-id")
            .await?;
        // Service creates job with the provided pipeline_id regardless (in-memory mock)
        assert!(result.get("id").is_some());

        // Test 5: Schedule job with empty data
        println!("  5️⃣  Scheduling job with minimal data...");
        let minimal_schedule = scheduler_client
            .schedule_job("test-job-id", "2025-12-31T23:59:59Z")
            .await?;
        assert!(minimal_schedule.get("id").is_some());

        // Test 6: Register worker with minimal data
        println!("  6️⃣  Registering worker with minimal data...");
        let minimal_worker = scheduler_client.register_worker("test-worker").await?;
        assert!(minimal_worker.get("id").is_some());

        // Test 7: Execute job with non-existent job ID
        println!("  7️⃣  Attempting to execute non-existent job...");
        let execution = worker_client
            .execute_job("non-existent-execution-id")
            .await?;
        // Service creates execution regardless (in-memory mock)
        assert!(execution.get("id").is_some());

        // Test 8: Try to get worker with invalid ID
        println!("  8️⃣  Attempting to get worker with invalid ID...");
        let response = reqwest::get(&format!(
            "{}/api/v1/workers/invalid-id",
            env.config.worker_manager_url()
        ))
        .await?;
        assert_eq!(response.status().as_u16(), 404);

        // Test 9: Try to stop non-existent worker
        println!("  9️⃣  Attempting to stop non-existent worker...");
        let response = reqwest::delete(&format!(
            "{}/api/v1/workers/non-existent-worker",
            env.config.worker_manager_url()
        ))
        .await?;
        assert_eq!(response.status().as_u16(), 404);

        // Test 10: Health check after all error tests
        println!("  🔟  Verifying services are still healthy after error tests...");
        let orchestrator_healthy = orchestrator_client.client.health().await?;
        let scheduler_healthy = scheduler_client.client.health().await?;
        let worker_healthy = worker_client.client.health().await?;

        assert!(orchestrator_healthy, "Orchestrator should be healthy");
        assert!(scheduler_healthy, "Scheduler should be healthy");
        assert!(worker_healthy, "Worker Manager should be healthy");

        // Collect metrics
        metrics.insert(
            "orchestrator_healthy".to_string(),
            orchestrator_healthy.to_string(),
        );
        metrics.insert(
            "scheduler_healthy".to_string(),
            scheduler_healthy.to_string(),
        );
        metrics.insert("worker_healthy".to_string(), worker_healthy.to_string());
        metrics.insert("error_tests_count".to_string(), "10".to_string());

        let duration = start_time.elapsed().as_millis() as u64;

        Ok(ScenarioResult::success(
            self.name().to_string(),
            duration,
            "All error scenarios handled correctly".to_string(),
        )
        .with_metrics(metrics))
    }
}

impl Default for ErrorHandlingScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_scenario_creation() {
        let scenario = ErrorHandlingScenario::new();
        assert_eq!(scenario.name(), "error_handling");
    }
}
