//! Functional Tests for Pipeline Execution
//!
//! These are REAL functional tests using PostgreSQL implementations with Testcontainers.
//! Tests are auto-gestionables - they start and stop their own PostgreSQL containers.
//!
//! Run with:
//! cargo test --features integration --lib postgres::functional_tests
//!

#[cfg(test)]
#[cfg(feature = "integration")]
mod tests {
    use hodei_adapters::{
        PostgreSqlJobRepository, PostgreSqlPipelineExecutionRepository,
        PostgreSqlPipelineRepository, PostgreSqlWorkerRepository,
    };
    use hodei_pipelines_core::{
        JobSpec, JobState, Pipeline, PipelineId, PipelineState, PipelineStep, PipelineStepId,
        ResourceQuota,
    };
    use hodei_modules::pipeline_execution_orchestrator::{
        PipelineExecutionConfig, PipelineExecutionOrchestrator,
    };
    use hodei_pipelines_ports::{
        EventPublisher, JobRepository, PipelineExecutionRepository, PipelineRepository,
    };
    use sqlx::{PgPool, postgres::PgPoolOptions};
    use std::collections::HashMap;
    use std::sync::Arc;
    use testcontainers::clients::Cli;
    use testcontainers::core::WaitFor;
    use testcontainers::modules::postgres::Postgres;
    use tokio::sync::Mutex;
    use tracing::info;

    /// Mock Event Publisher for testing
    #[derive(Debug, Default)]
    pub struct MockEventPublisher {
        published_events: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl EventPublisher for MockEventPublisher {
        async fn publish(&self, event: hodei_pipelines_ports::SystemEvent) -> hodei_pipelines_core::Result<()> {
            let mut events = self.published_events.lock().await;
            events.push(format!("{:?}", event));
            Ok(())
        }

        async fn publish_batch(
            &self,
            events: Vec<hodei_pipelines_ports::SystemEvent>,
        ) -> hodei_pipelines_core::Result<()> {
            let mut events_lock = self.published_events.lock().await;
            for event in events {
                events_lock.push(format!("{:?}", event));
            }
            Ok(())
        }
    }

    /// Setup a test database pool using Testcontainers (AUTO-GESTIONABLE)
    async fn setup_test_db() -> (PgPool, Postgres) {
        info!("🚀 Starting PostgreSQL container for tests");

        // Start PostgreSQL container automatically
        let postgres = Postgres::builder()
            .with_wait_for(WaitFor::LogMessage {
                message: "database system is ready to accept connections".to_string(),
                r#type: "stdout".to_string(),
            })
            .with_username("postgres")
            .with_password("postgres")
            .with_database("postgres")
            .build();

        let node = Cli::default().start(postgres);

        let connection_string = format!(
            "postgresql://postgres:postgres@{}:{}/postgres",
            node.get_host().await.unwrap(),
            node.get_host_port_ipv4(5432).await.unwrap()
        );

        info!("📡 Connecting to test database");

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await
            .expect("Failed to connect to test database");

        info!("✅ Test database ready");

        // Clean up any existing test data
        sqlx::query("DROP TABLE IF EXISTS pipeline_executions CASCADE")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DROP TABLE IF EXISTS pipeline_steps CASCADE")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DROP TABLE IF EXISTS pipelines CASCADE")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DROP TABLE IF EXISTS jobs CASCADE")
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DROP TABLE IF EXISTS workers CASCADE")
            .execute(&pool)
            .await
            .ok();

        (pool, node)
    }

    /// Test 1: Pipeline Creation and Persistence (TDD Red → Green)
    #[tokio::test]
    async fn test_pipeline_creation_and_persistence() {
        let (pool, _postgres) = setup_test_db().await;

        let pipeline_repo = PostgreSqlPipelineRepository::new(pool.clone());
        pipeline_repo.init_schema().await.unwrap();

        // Create a pipeline with steps
        let pipeline_id = PipelineId::new();
        let step1_id = PipelineStepId::new();
        let step2_id = PipelineStepId::new();

        let pipeline = Pipeline {
            id: pipeline_id.clone(),
            name: "test-pipeline".to_string(),
            description: Some("Test pipeline".to_string()),
            spec: hodei_pipelines_core::PipelineSpec {
                steps: vec![
                    PipelineStep {
                        id: step1_id.clone(),
                        name: "step1".to_string(),
                        job_spec: JobSpec {
                            name: "job1".to_string(),
                            image: "ubuntu".to_string(),
                            command: vec!["echo".to_string(), "hello".to_string()],
                            resources: ResourceQuota::default(),
                            timeout_ms: 30000,
                            retries: 3,
                            env: HashMap::new(),
                            secret_refs: vec![],
                        },
                        depends_on: vec![],
                        timeout_ms: 30000,
                    },
                    PipelineStep {
                        id: step2_id.clone(),
                        name: "step2".to_string(),
                        job_spec: JobSpec {
                            name: "job2".to_string(),
                            image: "ubuntu".to_string(),
                            command: vec!["echo".to_string(), "world".to_string()],
                            resources: ResourceQuota::default(),
                            timeout_ms: 30000,
                            retries: 3,
                            env: HashMap::new(),
                            secret_refs: vec![],
                        },
                        depends_on: vec![step1_id.clone()],
                        timeout_ms: 30000,
                    },
                ],
            },
            status: PipelineState::CREATED,
            variables: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: Some("test-tenant".to_string()),
            workflow_definition: serde_json::Value::Null,
        };

        // Save pipeline
        pipeline_repo.save_pipeline(&pipeline).await.unwrap();

        // Retrieve pipeline
        let retrieved = pipeline_repo
            .get_pipeline(&pipeline_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.id, pipeline_id);
        assert_eq!(retrieved.name, "test-pipeline");
        assert_eq!(retrieved.spec.steps.len(), 2);
        assert_eq!(retrieved.spec.steps[1].depends_on.len(), 1);
        assert_eq!(retrieved.spec.steps[1].depends_on[0], step1_id);
    }

    /// Test 2: Pipeline Execution Orchestration (TDD Red → Green)
    #[tokio::test]
    async fn test_pipeline_execution_orchestrator() {
        let (pool, _postgres) = setup_test_db().await;

        let job_repo = PostgreSqlJobRepository::new(pool.clone());
        let pipeline_repo = PostgreSqlPipelineRepository::new(pool.clone());
        let exec_repo = PostgreSqlPipelineExecutionRepository::new(pool.clone());
        let event_bus = MockEventPublisher::default();

        // Initialize schemas
        job_repo.init_schema().await.unwrap();
        pipeline_repo.init_schema().await.unwrap();
        exec_repo.init_schema().await.unwrap();

        // Create and save a pipeline
        let pipeline_id = PipelineId::new();
        let step_id = PipelineStepId::new();

        let pipeline = Pipeline {
            id: pipeline_id.clone(),
            name: "execution-test-pipeline".to_string(),
            description: None,
            spec: hodei_pipelines_core::PipelineSpec {
                steps: vec![PipelineStep {
                    id: step_id.clone(),
                    name: "test-step".to_string(),
                    job_spec: JobSpec {
                        name: "test-job".to_string(),
                        image: "ubuntu".to_string(),
                        command: vec!["sleep".to_string(), "1".to_string()],
                        resources: ResourceQuota::default(),
                        timeout_ms: 5000,
                        retries: 1,
                        env: HashMap::new(),
                        secret_refs: vec![],
                    },
                    depends_on: vec![],
                    timeout_ms: 5000,
                }],
            },
            status: PipelineState::CREATED,
            variables: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            workflow_definition: serde_json::Value::Null,
        };

        pipeline_repo.save_pipeline(&pipeline).await.unwrap();

        // Create orchestrator
        let config = PipelineExecutionConfig {
            max_concurrent_steps: 5,
            max_retry_attempts: 3,
            step_timeout_secs: 30,
            cleanup_interval_secs: 60,
        };

        let orchestrator = PipelineExecutionOrchestrator::new(
            Arc::new(exec_repo),
            Arc::new(job_repo),
            Arc::new(pipeline_repo),
            Arc::new(event_bus),
            config,
        );

        // Execute pipeline
        let execution_id = orchestrator
            .execute_pipeline(
                pipeline_id,
                HashMap::new(),
                None,
                Some("test-correlation-id".to_string()),
            )
            .await
            .unwrap();

        // Verify execution was created
        let execution = orchestrator
            .get_execution(&execution_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(execution.id, execution_id);
        assert_eq!(execution.steps.len(), 1);
    }

    /// Test 3: Pipeline with Multiple Steps and Dependencies (TDD Red → Green)
    #[tokio::test]
    async fn test_pipeline_with_dependencies() {
        let (pool, _postgres) = setup_test_db().await;
        let pipeline_repo = PostgreSqlPipelineRepository::new(pool);

        pipeline_repo.init_schema().await.unwrap();

        let pipeline_id = PipelineId::new();
        let step1 = PipelineStepId::new();
        let step2 = PipelineStepId::new();
        let step3 = PipelineStepId::new();

        // Create a pipeline with complex dependencies:
        // step1 -> step2 -> step3
        // step1 also directly to step3
        let pipeline = Pipeline {
            id: pipeline_id.clone(),
            name: "complex-dependencies-pipeline".to_string(),
            description: None,
            spec: hodei_pipelines_core::PipelineSpec {
                steps: vec![
                    PipelineStep {
                        id: step1.clone(),
                        name: "step1".to_string(),
                        job_spec: JobSpec {
                            name: "job1".to_string(),
                            image: "ubuntu".to_string(),
                            command: vec!["echo".to_string(), "step1".to_string()],
                            resources: ResourceQuota::default(),
                            timeout_ms: 30000,
                            retries: 3,
                            env: HashMap::new(),
                            secret_refs: vec![],
                        },
                        depends_on: vec![],
                        timeout_ms: 30000,
                    },
                    PipelineStep {
                        id: step2.clone(),
                        name: "step2".to_string(),
                        job_spec: JobSpec {
                            name: "job2".to_string(),
                            image: "ubuntu".to_string(),
                            command: vec!["echo".to_string(), "step2".to_string()],
                            resources: ResourceQuota::default(),
                            timeout_ms: 30000,
                            retries: 3,
                            env: HashMap::new(),
                            secret_refs: vec![],
                        },
                        depends_on: vec![step1.clone()],
                        timeout_ms: 30000,
                    },
                    PipelineStep {
                        id: step3.clone(),
                        name: "step3".to_string(),
                        job_spec: JobSpec {
                            name: "job3".to_string(),
                            image: "ubuntu".to_string(),
                            command: vec!["echo".to_string(), "step3".to_string()],
                            resources: ResourceQuota::default(),
                            timeout_ms: 30000,
                            retries: 3,
                            env: HashMap::new(),
                            secret_refs: vec![],
                        },
                        depends_on: vec![step2.clone(), step1.clone()],
                        timeout_ms: 30000,
                    },
                ],
            },
            status: PipelineState::CREATED,
            variables: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            workflow_definition: serde_json::Value::Null,
        };

        pipeline_repo.save_pipeline(&pipeline).await.unwrap();

        // Retrieve and verify dependencies
        let retrieved = pipeline_repo
            .get_pipeline(&pipeline_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.spec.steps.len(), 3);
        assert!(retrieved.spec.steps.iter().any(|s| s.id == step1));
        assert!(
            retrieved
                .spec
                .steps
                .iter()
                .any(|s| s.id == step2 && s.depends_on == vec![step1])
        );
        assert!(
            retrieved
                .spec
                .steps
                .iter()
                .any(|s| s.id == step3 && s.depends_on.len() == 2)
        );
    }

    /// Test 4: Pipeline Variables and Environment (TDD Red → Green)
    #[tokio::test]
    async fn test_pipeline_variables() {
        let (pool, _postgres) = setup_test_db().await;
        let pipeline_repo = PostgreSqlPipelineRepository::new(pool);

        pipeline_repo.init_schema().await.unwrap();

        let pipeline_id = PipelineId::new();
        let step_id = PipelineStepId::new();

        let mut variables = HashMap::new();
        variables.insert("ENV".to_string(), "production".to_string());
        variables.insert("DEBUG".to_string(), "false".to_string());

        let pipeline = Pipeline {
            id: pipeline_id.clone(),
            name: "variables-test-pipeline".to_string(),
            description: None,
            spec: hodei_pipelines_core::PipelineSpec {
                steps: vec![PipelineStep {
                    id: step_id.clone(),
                    name: "step-with-vars".to_string(),
                    job_spec: JobSpec {
                        name: "job-with-vars".to_string(),
                        image: "ubuntu".to_string(),
                        command: vec!["env".to_string()],
                        resources: ResourceQuota::default(),
                        timeout_ms: 30000,
                        retries: 3,
                        env: variables.clone(),
                        secret_refs: vec![],
                    },
                    depends_on: vec![],
                    timeout_ms: 30000,
                }],
            },
            status: PipelineState::CREATED,
            variables: variables.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: Some("multi-tenant".to_string()),
            workflow_definition: serde_json::Value::Null,
        };

        pipeline_repo.save_pipeline(&pipeline).await.unwrap();

        let retrieved = pipeline_repo
            .get_pipeline(&pipeline_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.variables, variables);
        assert_eq!(retrieved.tenant_id, Some("multi-tenant".to_string()));

        // Verify job spec also has the variables
        let step = &retrieved.spec.steps[0];
        assert_eq!(step.job_spec.env, variables);
    }
}
