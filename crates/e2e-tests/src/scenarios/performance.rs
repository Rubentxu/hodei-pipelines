//! Performance scenario
//!
//! This module implements performance test scenarios that validate
//! the platform's behavior under load and measure response times.

use async_trait::async_trait;
use futures::future;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::helpers::TestDataGenerator;
use crate::infrastructure::{
    OrchestratorClient, SchedulerClient, TestEnvironment, WorkerManagerClient,
};
use crate::{Scenario, ScenarioResult, TestResult};

/// Performance test scenario
pub struct PerformanceScenario {
    generator: TestDataGenerator,
}

impl PerformanceScenario {
    /// Create a new performance scenario
    pub fn new() -> Self {
        Self {
            generator: TestDataGenerator::new(),
        }
    }
}

#[async_trait]
impl Scenario for PerformanceScenario {
    fn name(&self) -> &'static str {
        "performance"
    }

    fn description(&self) -> &'static str {
        "Test system under load: concurrent operations, response times, throughput"
    }

    async fn run(&self, env: &TestEnvironment) -> TestResult<ScenarioResult> {
        let mut metrics = std::collections::HashMap::new();
        let start_time = std::time::Instant::now();

        // Initialize HTTP clients
        let orchestrator_client = Arc::new(OrchestratorClient::new(env.config.orchestrator_url()));
        let scheduler_client = Arc::new(SchedulerClient::new(env.config.scheduler_url()));
        let worker_client = Arc::new(WorkerManagerClient::new(env.config.worker_manager_url()));

        // Test 1: Concurrent pipeline creation
        println!("  1️⃣  Creating multiple pipelines concurrently...");
        let pipeline_count = 10;
        let mut pipeline_handles = Vec::new();

        for i in 0..pipeline_count {
            let client = Arc::clone(&orchestrator_client);
            let name = format!("concurrent-pipeline-{}", i);
            let handle = tokio::spawn(async move {
                client
                    .create_pipeline(&name, "Performance test pipeline")
                    .await
            });
            pipeline_handles.push(handle);
        }

        let pipelines = future::join_all(pipeline_handles).await;
        let mut created_pipelines = 0;
        for result in pipelines {
            if result.is_ok() {
                created_pipelines += 1;
            }
        }
        assert_eq!(created_pipelines, pipeline_count);

        println!("     ✅ Created {} pipelines", created_pipelines);

        // Test 2: Concurrent job creation
        println!("  2️⃣  Creating multiple jobs concurrently...");
        let job_count = 20;
        let mut job_handles = Vec::new();

        for i in 0..job_count {
            let client = Arc::clone(&orchestrator_client);
            let pipeline_id = format!("pipeline-{}", i % pipeline_count);
            let handle = tokio::spawn(async move { client.create_job(&pipeline_id).await });
            job_handles.push(handle);
        }

        let jobs = future::join_all(job_handles).await;
        let mut created_jobs = 0;
        for result in jobs {
            if result.is_ok() {
                created_jobs += 1;
            }
        }
        assert!(created_jobs > 0);

        println!("     ✅ Created {} jobs", created_jobs);

        // Test 3: Concurrent worker registration
        println!("  3️⃣  Registering multiple workers concurrently...");
        let worker_count = 5;
        let mut worker_handles = Vec::new();

        for i in 0..worker_count {
            let client = Arc::clone(&scheduler_client);
            let worker_type = if i % 2 == 0 { "rust" } else { "python" };
            let handle = tokio::spawn(async move { client.register_worker(worker_type).await });
            worker_handles.push(handle);
        }

        let workers = future::join_all(worker_handles).await;
        let mut registered_workers = 0;
        for result in workers {
            if result.is_ok() {
                registered_workers += 1;
            }
        }
        assert_eq!(registered_workers, worker_count);

        println!("     ✅ Registered {} workers", registered_workers);

        // Test 4: Concurrent job execution with semaphore (limit concurrency)
        println!("  4️⃣  Executing jobs with controlled concurrency...");
        let semaphore = Arc::new(Semaphore::new(3)); // Allow max 3 concurrent executions
        let execution_count = 10;
        let mut execution_handles = Vec::new();

        for i in 0..execution_count {
            let client = Arc::clone(&worker_client);
            let semaphore = Arc::clone(&semaphore);
            let job_id = format!("job-{}", i);
            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                client.execute_job(&job_id).await
            });
            execution_handles.push(handle);
        }

        let executions = future::join_all(execution_handles).await;
        let mut executed_jobs = 0;
        for result in executions {
            if result.is_ok() {
                executed_jobs += 1;
            }
        }
        assert!(executed_jobs > 0);

        println!("     ✅ Executed {} jobs", executed_jobs);

        // Test 5: Measure response times for listing operations
        println!("  5️⃣  Measuring response times for list operations...");

        let list_start = std::time::Instant::now();
        let pipelines = orchestrator_client.list_pipelines().await?;
        let pipelines_time = list_start.elapsed();

        let list_start = std::time::Instant::now();
        let jobs = orchestrator_client.list_jobs().await?;
        let jobs_time = list_start.elapsed();

        let list_start = std::time::Instant::now();
        let workers = scheduler_client.list_workers().await?;
        let workers_time = list_start.elapsed();

        let list_start = std::time::Instant::now();
        let executions = worker_client.list_executions().await?;
        let executions_time = list_start.elapsed();

        println!("     📊 Response times:");
        println!("        - List pipelines: {}ms", pipelines_time.as_millis());
        println!("        - List jobs: {}ms", jobs_time.as_millis());
        println!("        - List workers: {}ms", workers_time.as_millis());
        println!(
            "        - List executions: {}ms",
            executions_time.as_millis()
        );

        // Collect performance metrics
        metrics.insert(
            "pipelines_created".to_string(),
            created_pipelines.to_string(),
        );
        metrics.insert("jobs_created".to_string(), created_jobs.to_string());
        metrics.insert(
            "workers_registered".to_string(),
            registered_workers.to_string(),
        );
        metrics.insert("jobs_executed".to_string(), executed_jobs.to_string());
        metrics.insert(
            "pipelines_list_time_ms".to_string(),
            pipelines_time.as_millis().to_string(),
        );
        metrics.insert(
            "jobs_list_time_ms".to_string(),
            jobs_time.as_millis().to_string(),
        );
        metrics.insert(
            "workers_list_time_ms".to_string(),
            workers_time.as_millis().to_string(),
        );
        metrics.insert(
            "executions_list_time_ms".to_string(),
            executions_time.as_millis().to_string(),
        );
        metrics.insert(
            "pipelines_count".to_string(),
            pipelines.as_array().unwrap_or(&vec![]).len().to_string(),
        );
        metrics.insert(
            "jobs_count".to_string(),
            jobs.as_array().unwrap_or(&vec![]).len().to_string(),
        );
        metrics.insert(
            "workers_count".to_string(),
            workers.as_array().unwrap_or(&vec![]).len().to_string(),
        );
        metrics.insert(
            "executions_count".to_string(),
            executions.as_array().unwrap_or(&vec![]).len().to_string(),
        );

        // Verify all operations completed
        assert!(pipelines.as_array().unwrap_or(&vec![]).len() > 0);
        assert!(jobs.as_array().unwrap_or(&vec![]).len() > 0);
        assert!(workers.as_array().unwrap_or(&vec![]).len() > 0);
        assert!(executions.as_array().unwrap_or(&vec![]).len() > 0);

        let duration = start_time.elapsed().as_millis() as u64;

        Ok(ScenarioResult::success(
            self.name().to_string(),
            duration,
            format!(
                "Performance test completed: {} pipelines, {} jobs, {} workers, {} executions",
                created_pipelines, created_jobs, registered_workers, executed_jobs
            ),
        )
        .with_metrics(metrics))
    }
}

impl Default for PerformanceScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_scenario_creation() {
        let scenario = PerformanceScenario::new();
        assert_eq!(scenario.name(), "performance");
    }
}
