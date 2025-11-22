/// Rust SDK Client for Hodei CI/CD platform
use hodei_sdk_core::{
    ClientConfig, HttpClient, Job, JobStatus, Pipeline, PipelineConfig, SdkResult, Worker,
};
use std::time::Duration;
use tracing::{debug, info};

/// Main client for interacting with Hodei CI/CD platform
#[derive(Clone)]
pub struct CicdClient {
    http_client: HttpClient,
}

impl CicdClient {
    /// Create a new CICD client
    ///
    /// # Arguments
    /// * `base_url` - Base URL of the API (e.g., "https://api.hodei.example.com")
    /// * `api_token` - Authentication token
    ///
    /// # Example
    /// ```no_run
    /// use hodei_rust_sdk::CicdClient;
    ///
    /// let client = CicdClient::new("https://api.hodei.example.com", "your-token")
    ///     .expect("Failed to create client");
    /// ```
    pub fn new(base_url: impl Into<String>, api_token: impl Into<String>) -> SdkResult<Self> {
        let config = ClientConfig::new(base_url, api_token);
        let http_client = HttpClient::new(config)?;

        Ok(Self { http_client })
    }

    /// Create a new CICD client with custom configuration
    ///
    /// # Example
    /// ```no_run
    /// use hodei_rust_sdk::CicdClient;
    /// use hodei_sdk_core::ClientConfig;
    /// use std::time::Duration;
    ///
    /// let config = ClientConfig::new("https://api.hodei.example.com", "your-token")
    ///     .with_timeout(Duration::from_secs(60))
    ///     .with_max_retries(5);
    ///
    /// let client = CicdClient::with_config(config).expect("Failed to create client");
    /// ```
    pub fn with_config(config: ClientConfig) -> SdkResult<Self> {
        let http_client = HttpClient::new(config)?;
        Ok(Self { http_client })
    }

    /// Create a new pipeline
    ///
    /// # Example
    /// ```no_run
    /// # use hodei_rust_sdk::CicdClient;
    /// # use hodei_sdk_core::{PipelineConfig, StageConfig};
    /// # use std::collections::HashMap;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CicdClient::new("https://api.hodei.example.com", "token")?;
    ///
    /// let pipeline_config = PipelineConfig {
    ///     name: "my-pipeline".to_string(),
    ///     description: Some("Test pipeline".to_string()),
    ///     stages: vec![
    ///         StageConfig {
    ///             name: "build".to_string(),
    ///             image: "rust:1.70".to_string(),
    ///             commands: vec!["cargo build".to_string()],
    ///             dependencies: vec![],
    ///             environment: HashMap::new(),
    ///             resources: None,
    ///         }
    ///     ],
    ///     environment: HashMap::new(),
    ///     triggers: vec![],
    /// };
    ///
    /// let pipeline = client.create_pipeline(pipeline_config).await?;
    /// println!("Created pipeline: {}", pipeline.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_pipeline(&self, config: PipelineConfig) -> SdkResult<Pipeline> {
        info!("Creating pipeline: {}", config.name);
        self.http_client.post("/api/v1/pipelines", &config).await
    }

    /// Get a pipeline by ID
    pub async fn get_pipeline(&self, pipeline_id: &str) -> SdkResult<Pipeline> {
        debug!("Getting pipeline: {}", pipeline_id);
        self.http_client
            .get(&format!("/api/v1/pipelines/{}", pipeline_id))
            .await
    }

    /// List all pipelines
    pub async fn list_pipelines(&self) -> SdkResult<Vec<Pipeline>> {
        debug!("Listing all pipelines");
        self.http_client.get("/api/v1/pipelines").await
    }

    /// Execute a pipeline
    ///
    /// # Example
    /// ```no_run
    /// # use hodei_rust_sdk::CicdClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CicdClient::new("https://api.hodei.example.com", "token")?;
    ///
    /// let job = client.execute_pipeline("pipeline-123").await?;
    /// println!("Started job: {}", job.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_pipeline(&self, pipeline_id: &str) -> SdkResult<Job> {
        info!("Executing pipeline: {}", pipeline_id);
        self.http_client
            .post(&format!("/api/v1/pipelines/{}/execute", pipeline_id), &())
            .await
    }

    /// Get job status
    pub async fn get_job_status(&self, job_id: &str) -> SdkResult<Job> {
        debug!("Getting job status: {}", job_id);
        self.http_client
            .get(&format!("/api/v1/jobs/{}", job_id))
            .await
    }

    /// Get job logs
    pub async fn get_job_logs(&self, job_id: &str) -> SdkResult<String> {
        debug!("Getting job logs: {}", job_id);
        self.http_client
            .get(&format!("/api/v1/jobs/{}/logs", job_id))
            .await
    }

    /// Wait for job completion
    ///
    /// # Arguments
    /// * `job_id` - Job identifier
    /// * `poll_interval` - Interval between status checks
    ///
    /// # Example
    /// ```no_run
    /// # use hodei_rust_sdk::CicdClient;
    /// # use std::time::Duration;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CicdClient::new("https://api.hodei.example.com", "token")?;
    ///
    /// let job = client.execute_pipeline("pipeline-123").await?;
    /// let completed_job = client.wait_for_completion(&job.id, Duration::from_secs(5)).await?;
    ///
    /// println!("Job completed with status: {:?}", completed_job.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_completion(
        &self,
        job_id: &str,
        poll_interval: Duration,
    ) -> SdkResult<Job> {
        info!("Waiting for job completion: {}", job_id);

        loop {
            let job = self.get_job_status(job_id).await?;

            match job.status {
                JobStatus::Success | JobStatus::Failed | JobStatus::Cancelled => {
                    info!("Job {} completed with status: {:?}", job_id, job.status);
                    return Ok(job);
                }
                JobStatus::Queued | JobStatus::Running => {
                    debug!("Job {} still running, waiting...", job_id);
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }
    }

    /// Delete a pipeline
    pub async fn delete_pipeline(&self, pipeline_id: &str) -> SdkResult<()> {
        info!("Deleting pipeline: {}", pipeline_id);
        self.http_client
            .delete(&format!("/api/v1/pipelines/{}", pipeline_id))
            .await
    }

    /// List all workers
    pub async fn list_workers(&self) -> SdkResult<Vec<Worker>> {
        debug!("Listing all workers");
        self.http_client.get("/api/v1/workers").await
    }

    /// Get worker by ID
    pub async fn get_worker(&self, worker_id: &str) -> SdkResult<Worker> {
        debug!("Getting worker: {}", worker_id);
        self.http_client
            .get(&format!("/api/v1/workers/{}", worker_id))
            .await
    }

    /// Scale workers in a worker group
    pub async fn scale_workers(&self, worker_group: &str, target_count: u32) -> SdkResult<()> {
        info!(
            "Scaling worker group {} to {} instances",
            worker_group, target_count
        );

        #[derive(serde::Serialize)]
        struct ScaleRequest {
            target_count: u32,
        }

        self.http_client
            .post(
                &format!("/api/v1/workers/groups/{}/scale", worker_group),
                &ScaleRequest { target_count },
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_sdk_core::{PipelineStatus, WorkerStatus};
    use mockito::Server;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_create_pipeline() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/pipelines")
            .match_header("authorization", "Bearer test-token")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "id": "pipeline-123",
                "name": "test-pipeline",
                "status": "PENDING",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": null,
                "config": {
                    "name": "test-pipeline",
                    "description": null,
                    "stages": [],
                    "environment": {},
                    "triggers": []
                }
            }"#,
            )
            .create_async()
            .await;

        let client = CicdClient::new(server.url(), "test-token").unwrap();

        let config = PipelineConfig {
            name: "test-pipeline".to_string(),
            description: None,
            stages: vec![],
            environment: HashMap::new(),
            triggers: vec![],
        };

        let pipeline = client.create_pipeline(config).await.unwrap();

        assert_eq!(pipeline.id, "pipeline-123");
        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.status, PipelineStatus::Pending);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_execute_pipeline() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/pipelines/pipeline-123/execute")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "id": "job-456",
                "pipeline_id": "pipeline-123",
                "name": "test-job",
                "status": "QUEUED",
                "started_at": "2024-01-01T00:00:00Z",
                "finished_at": null,
                "error_message": null
            }"#,
            )
            .create_async()
            .await;

        let client = CicdClient::new(server.url(), "test-token").unwrap();
        let job = client.execute_pipeline("pipeline-123").await.unwrap();

        assert_eq!(job.id, "job-456");
        assert_eq!(job.pipeline_id, "pipeline-123");
        assert_eq!(job.status, JobStatus::Queued);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_list_workers() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("GET", "/api/v1/workers")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{
                "id": "worker-1",
                "name": "worker-1",
                "status": "ACTIVE",
                "provider": "kubernetes",
                "cpu_usage": 0.5,
                "memory_usage": 0.6,
                "jobs_processed": 10
            }]"#,
            )
            .create_async()
            .await;

        let client = CicdClient::new(server.url(), "test-token").unwrap();
        let workers = client.list_workers().await.unwrap();

        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].id, "worker-1");
        assert_eq!(workers[0].status, WorkerStatus::Active);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_error_handling() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("GET", "/api/v1/pipelines/non-existent")
            .with_status(404)
            .with_body("Pipeline not found")
            .create_async()
            .await;

        let client = CicdClient::new(server.url(), "test-token").unwrap();
        let result = client.get_pipeline("non-existent").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().is_not_found());

        mock.assert_async().await;
    }
}
