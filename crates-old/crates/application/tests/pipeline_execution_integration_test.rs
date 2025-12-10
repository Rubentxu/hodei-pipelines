//! Pipeline Execution Integration Test
//!
//! This test verifies the complete pipeline execution flow with ephemeral workers,
//! including WorkerProvisioner, PipelineStepExecutor, and ToolManager integration.

use async_trait::async_trait;
use hodei_pipelines_adapters::{
    InMemoryBus, PostgreSqlJobRepository, PostgreSqlPipelineExecutionRepository,
    PostgreSqlPipelineRepository,
};
use hodei_pipelines_application::{
    pipeline_execution::{
        ephemeral_worker_handle::ProvisionedWorkerHandleBuilder,
        integration_service::{
            PipelineExecutionIntegrationConfig, PipelineExecutionIntegrationService,
            PipelineExecutionService,
        },
        pipeline_step_executor::{
            EphemeralWorkerHandle, StepExecutionResult, ToolManager, ToolRequirement,
        },
        tool_manager::AsdfToolManager,
    },
    scheduling::worker_provisioner::WorkerProvisionerService,
};
use hodei_pipelines_domain::{
    DomainError, Pipeline, PipelineId, Worker, WorkerCapabilities, WorkerId,
    pipeline_execution::{
        entities::pipeline::PipelineStep as DomainPipelineStep,
        job_definitions::{JobSpec, ResourceQuota},
    },
    resource_governance::{
        ComputePool, ComputePoolBuilder, GlobalResourceController, PoolCapacity, PoolId,
        ProviderType,
    },
};
use hodei_pipelines_ports::{
    PipelineRepository,
    scheduling::{
        scheduler_port::SchedulerPort,
        worker_provider::{ProviderConfig, WorkerProvider},
        worker_provisioner::{ProvisionedWorker, WorkerProvisioner},
        worker_template::{
            ContainerSpec, ResourceRequirements as TemplateResourceRequirements, TemplateMetadata,
            WorkerTemplate,
        },
    },
};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use testcontainers::{ContainerAsync, GenericImage, ImageExt, runners::AsyncRunner};
use tokio::process::Command;
use tokio::sync::OnceCell;

// Singleton Test Context
struct TestContext {
    #[allow(dead_code)]
    container: ContainerAsync<GenericImage>,
    pool: sqlx::PgPool,
}

static TEST_CONTEXT: OnceCell<TestContext> = OnceCell::const_new();

async fn get_test_context() -> &'static TestContext {
    TEST_CONTEXT
        .get_or_init(|| async {
            let postgres_image = GenericImage::new("postgres", "15-alpine")
                .with_env_var("POSTGRES_DB", "hodei_test")
                .with_env_var("POSTGRES_USER", "hodei")
                .with_env_var("POSTGRES_PASSWORD", "hodei");

            let container = postgres_image
                .start()
                .await
                .expect("Failed to start Postgres container");

            let port = container
                .get_host_port_ipv4(5432)
                .await
                .expect("Failed to get Postgres port");

            let connection_string = format!("postgres://hodei:hodei@127.0.0.1:{}/hodei_test", port);

            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(&connection_string)
                .await
                .expect("Failed to connect to Postgres");

            TestContext { container, pool }
        })
        .await
}

// Mock WorkerProvider for testing
use hodei_pipelines_ports::scheduling::worker_provider::{ProviderCapabilities, ProviderError};

#[derive(Debug, Clone)]
struct MockWorkerProvider {
    capabilities: ProviderCapabilities,
}

#[async_trait]
impl WorkerProvider for MockWorkerProvider {
    fn provider_type(&self) -> hodei_pipelines_ports::scheduling::worker_provider::ProviderType {
        hodei_pipelines_ports::scheduling::worker_provider::ProviderType::Docker
    }

    fn name(&self) -> &str {
        "mock-provider"
    }

    async fn capabilities(&self) -> std::result::Result<ProviderCapabilities, ProviderError> {
        Ok(
            hodei_pipelines_ports::scheduling::worker_provider::ProviderCapabilities {
                supports_auto_scaling: true,
                supports_health_checks: true,
                supports_volumes: true,
                max_workers: Some(10),
                estimated_provision_time_ms: 1000,
            },
        )
    }

    async fn create_worker(
        &self,
        worker_id: WorkerId,
        _config: ProviderConfig,
    ) -> std::result::Result<Worker, ProviderError> {
        Ok(Worker::new(
            worker_id,
            "mock-worker".to_string(),
            WorkerCapabilities::new(4, 8192),
        ))
    }

    async fn get_worker_status(
        &self,
        _worker_id: &WorkerId,
    ) -> std::result::Result<hodei_pipelines_domain::WorkerStatus, ProviderError> {
        Ok(hodei_pipelines_domain::WorkerStatus::create_with_status(
            "IDLE".to_string(),
        ))
    }

    async fn stop_worker(
        &self,
        _worker_id: &WorkerId,
        _force: bool,
    ) -> std::result::Result<(), ProviderError> {
        Ok(())
    }

    async fn delete_worker(&self, _worker_id: &WorkerId) -> std::result::Result<(), ProviderError> {
        Ok(())
    }

    async fn list_workers(&self) -> std::result::Result<Vec<WorkerId>, ProviderError> {
        Ok(vec![])
    }

    async fn create_ephemeral_worker(
        &self,
        worker_id: WorkerId,
        _config: ProviderConfig,
        _timeout_secs: Option<u64>,
    ) -> std::result::Result<hodei_pipelines_domain::Worker, ProviderError> {
        let worker = hodei_pipelines_domain::Worker::new(
            worker_id,
            "mock-ephemeral-worker".to_string(),
            hodei_pipelines_domain::WorkerCapabilities::new(4, 8192),
        );
        Ok(worker)
    }
}

// Mock SchedulerPort for testing
#[derive(Debug, Clone)]
struct MockScheduler;

#[async_trait]
impl SchedulerPort for MockScheduler {
    async fn register_worker(
        &self,
        _worker: &Worker,
    ) -> std::result::Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        Ok(())
    }

    async fn unregister_worker(
        &self,
        _worker_id: &WorkerId,
    ) -> std::result::Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        Ok(())
    }

    async fn get_registered_workers(
        &self,
    ) -> std::result::Result<Vec<WorkerId>, hodei_pipelines_ports::scheduler_port::SchedulerError>
    {
        Ok(vec![])
    }

    async fn register_transmitter(
        &self,
        _worker_id: &WorkerId,
        _transmitter: tokio::sync::mpsc::UnboundedSender<
            std::result::Result<
                hodei_pipelines_proto::ServerMessage,
                hodei_pipelines_ports::scheduler_port::SchedulerError,
            >,
        >,
    ) -> std::result::Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        Ok(())
    }

    async fn unregister_transmitter(
        &self,
        _worker_id: &WorkerId,
    ) -> std::result::Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        Ok(())
    }

    async fn send_to_worker(
        &self,
        _worker_id: &WorkerId,
        _message: hodei_pipelines_proto::ServerMessage,
    ) -> std::result::Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        Ok(())
    }
}

// Mock EphemeralWorkerHandle for testing
#[derive(Debug, Clone)]
struct MockEphemeralWorkerHandle {
    worker_id: WorkerId,
}

#[async_trait]
impl EphemeralWorkerHandle for MockEphemeralWorkerHandle {
    async fn execute_command(
        &self,
        _command: &str,
        _args: &[String],
        working_dir: Option<&Path>,
        env_vars: Option<&[(String, String)]>,
    ) -> hodei_pipelines_domain::Result<StepExecutionResult> {
        // Simulate command execution
        let start_time = std::time::Instant::now();

        // Build command
        let mut cmd = Command::new("echo");
        cmd.arg("Mock execution");

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        if let Some(vars) = env_vars {
            for (key, value) in vars {
                cmd.env(key, value);
            }
        }

        let output = cmd.output().await.map_err(|e| {
            DomainError::Infrastructure(format!("Failed to execute mock command: {}", e))
        })?;

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let exit_code = output.status.code();
        let success = output.status.success();

        Ok(StepExecutionResult {
            step_id: self.worker_id.to_string(),
            success,
            exit_code,
            duration_ms,
            logs: vec!["Mock execution completed".to_string()],
        })
    }

    fn worker_id(&self) -> &WorkerId {
        &self.worker_id
    }

    async fn is_healthy(&self) -> hodei_pipelines_domain::Result<bool> {
        Ok(true)
    }
}

// Mock ToolManager for testing
#[derive(Debug, Clone)]
struct MockToolManager;

#[async_trait]
impl ToolManager for MockToolManager {
    async fn install_tools(
        &self,
        _workspace_path: &Path,
        _tools: &[ToolRequirement],
    ) -> hodei_pipelines_domain::Result<()> {
        Ok(())
    }

    async fn is_tool_installed(
        &self,
        _workspace_path: &Path,
        _tool: &ToolRequirement,
    ) -> hodei_pipelines_domain::Result<bool> {
        Ok(true)
    }
}

// Helper function to create test pipeline
fn create_test_pipeline() -> Pipeline {
    let pipeline_id = PipelineId::new();
    let steps = vec![
        DomainPipelineStep::new(
            "step-1".to_string(),
            JobSpec {
                name: "step-1".to_string(),
                image: "alpine".to_string(),
                command: vec!["echo".to_string(), "Hello from step 1".to_string()],
                resources: ResourceQuota::new(1000, 1024),
                timeout_ms: 30000,
                retries: 3,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            30000,
        ),
        DomainPipelineStep::new(
            "step-2".to_string(),
            JobSpec {
                name: "step-2".to_string(),
                image: "alpine".to_string(),
                command: vec!["echo".to_string(), "Hello from step 2".to_string()],
                resources: ResourceQuota::new(1000, 1024),
                timeout_ms: 30000,
                retries: 3,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            30000,
        ),
    ];

    Pipeline::new(pipeline_id, "test-pipeline".to_string(), steps).unwrap()
}

// Helper function to create test compute pool
fn create_test_compute_pool() -> ComputePool {
    ComputePoolBuilder::new()
        .id(PoolId::from("test-pool"))
        .name("Test Pool".to_string())
        .provider_type(ProviderType::Docker)
        .total_capacity(PoolCapacity {
            cpu_millicores: 10000,
            memory_mb: 8192,
            gpu_count: 0,
            max_workers: 10,
            active_workers: 0,
            storage_gb: None,
        })
        .build()
        .unwrap()
}

// TEMPORARILY DISABLED - Requires TestContainers and Docker
// #[tokio::test]
async fn _test_pipeline_execution_integration_disabled() {
    // Setup temporary directory for workspace
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().to_path_buf();

    // Get singleton test context
    let context = get_test_context().await;
    let pool = context.pool.clone();

    // Initialize Repositories
    let job_repo = Arc::new(PostgreSqlJobRepository::new(
        pool.clone(),
        None,
        "20241201_jobs.sql".to_string(),
    ));
    let pipeline_repo = Arc::new(PostgreSqlPipelineRepository::new(
        pool.clone(),
        None,
        "20241201_pipelines.sql".to_string(),
    ));
    let execution_repo = Arc::new(PostgreSqlPipelineExecutionRepository::new(
        pool.clone(),
        None,
        "20241201_pipeline_executions.sql".to_string(),
    ));

    // Initialize Schemas
    job_repo
        .init_schema()
        .await
        .expect("Failed to init job schema");
    pipeline_repo
        .init_schema()
        .await
        .expect("Failed to init pipeline schema");
    execution_repo
        .init_schema()
        .await
        .expect("Failed to init execution schema");

    // Create mock components
    let worker_provider = Arc::new(MockWorkerProvider {
        capabilities: ProviderCapabilities {
            supports_auto_scaling: true,
            supports_health_checks: true,
            supports_volumes: true,
            max_workers: Some(10),
            estimated_provision_time_ms: 1000,
        },
    });
    let scheduler = Arc::new(MockScheduler);
    let tool_manager = Arc::new(MockToolManager);

    // Create Global Resource Controller with test pool
    let grc = Arc::new(GlobalResourceController::new(
        hodei_pipelines_domain::resource_governance::GRCConfig::default(),
        Arc::new(
            hodei_pipelines_adapters::RedbResourcePoolRepository::new_with_path(
                temp_dir.path().join("resources.db").to_str().unwrap(),
            )
            .unwrap(),
        ),
    ));
    let _test_pool = create_test_compute_pool();

    // Note: In a real test, we would add the pool to GRC
    // For now, we'll skip this as GRC integration requires more setup

    // Create WorkerProvisionerService
    let worker_provisioner = Arc::new(WorkerProvisionerService::new(
        worker_provider,
        scheduler,
        grc,
        "default-namespace".to_string(),
    ));

    // Create in-memory event bus
    let event_bus = Arc::new(InMemoryBus::default());

    // Create pipeline execution integration service
    let config = PipelineExecutionIntegrationConfig {
        workspace_base_dir: workspace_dir.clone(),
        auto_install_tools: false, // Disable for test
        cleanup_workspace: false,
        max_concurrent_executions: 1,
    };

    let integration_service = PipelineExecutionIntegrationService::new(
        worker_provisioner,
        tool_manager,
        execution_repo.clone(),
        job_repo.clone(),
        pipeline_repo.clone(),
        event_bus.clone(),
        config,
    );

    // Create and save test pipeline
    let pipeline = create_test_pipeline();
    pipeline_repo
        .save_pipeline(&pipeline)
        .await
        .expect("Failed to save pipeline");

    // Test 1: Execute pipeline with ephemeral worker
    let execution_id = integration_service
        .execute_pipeline(pipeline.id.clone(), HashMap::new(), None, None)
        .await
        .expect("Failed to execute pipeline");

    println!("Pipeline execution started: {}", execution_id);

    // Test 2: Check execution status
    let execution_status = integration_service
        .get_execution(&execution_id)
        .await
        .expect("Failed to get execution status");

    assert!(execution_status.is_some());
    println!("Execution status retrieved");

    // Test 3: Verify ephemeral worker handle builder
    let mock_provisioned_worker = ProvisionedWorker {
        worker: Worker::new(
            WorkerId::new(),
            "test-worker".to_string(),
            WorkerCapabilities::new(4, 8192),
        ),
        worker_id: WorkerId::new(),
        template: WorkerTemplate {
            metadata: TemplateMetadata {
                name: "test-template".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                labels: HashMap::new(),
            },
            container: ContainerSpec {
                image: "test-image".to_string(),
                image_pull_policy: None,
                command: None,
                args: None,
                working_dir: None,
            },
            resources: TemplateResourceRequirements {
                cpu: Some("1000m".to_string()),
                memory: Some("1024Mi".to_string()),
                ephemeral_storage: None,
            },
            volumes: vec![],
            env: vec![],
            network: None,
            security: None,
            tools: vec![],
            labels: HashMap::new(),
            annotations: HashMap::new(),
            command: None,
            working_dir: Some("/workspace".to_string()),
            provider_config: HashMap::new(),
        },
        pool_id: Some("test-pool".to_string()),
        allocation_metadata:
            hodei_pipelines_ports::scheduling::worker_provisioner::AllocationMetadata {
                allocated_at: chrono::Utc::now(),
                estimated_cost_per_hour: None,
                auto_cleanup_seconds: Some(3600),
                pool_constraints: vec!["docker".to_string()],
            },
    };

    let handle_builder = ProvisionedWorkerHandleBuilder::new(Arc::new(mock_provisioned_worker));
    let _handle = handle_builder.build();
    println!("Ephemeral worker handle created successfully");

    // Test 4: Verify tool manager integration
    let _asdf_tool_manager = AsdfToolManager::new(workspace_dir.clone());
    println!("Tool manager created successfully");

    // Note: The actual execution would happen asynchronously
    // In a real test, we would wait for completion and verify results
    // For now, we verify that the components can be created and integrated

    println!("Pipeline execution integration test completed successfully");
}

#[tokio::test]
async fn test_worker_provisioner_calculation() {
    let pipeline = create_test_pipeline();

    // Create mock components
    let worker_provider = Arc::new(MockWorkerProvider {
        capabilities: ProviderCapabilities {
            supports_auto_scaling: true,
            supports_health_checks: true,
            supports_volumes: true,
            max_workers: Some(10),
            estimated_provision_time_ms: 1000,
        },
    });
    let scheduler = Arc::new(MockScheduler);
    let temp_dir = TempDir::new().unwrap();
    let grc = Arc::new(GlobalResourceController::new(
        hodei_pipelines_domain::resource_governance::GRCConfig::default(),
        Arc::new(
            hodei_pipelines_adapters::RedbResourcePoolRepository::new_with_path(
                temp_dir.path().join("resources.db").to_str().unwrap(),
            )
            .unwrap(),
        ),
    ));

    let worker_provisioner = WorkerProvisionerService::new(
        worker_provider,
        scheduler,
        grc,
        "default-namespace".to_string(),
    );

    // Test resource requirements calculation
    let requirements = worker_provisioner
        .calculate_resource_requirements(&pipeline)
        .await
        .expect("Failed to calculate resource requirements");

    // Verify calculations
    assert!(requirements.cpu_millicores > 0);
    assert!(requirements.memory_mb > 0);
    assert!(requirements.estimated_duration_secs > 0);

    println!(
        "Resource requirements calculated: CPU={}m, Memory={}MB, Duration={}s",
        requirements.cpu_millicores, requirements.memory_mb, requirements.estimated_duration_secs
    );

    // Test worker template creation
    let template = worker_provisioner
        .create_worker_template(&pipeline, &requirements)
        .await
        .expect("Failed to create worker template");

    assert_eq!(
        template.metadata.name,
        format!("pipeline-{}-template", pipeline.id)
    );
    assert_eq!(template.container.image, "hwp-agent:latest");
    assert_eq!(template.working_dir, Some("/workspace".to_string()));

    println!(
        "Worker template created successfully: {}",
        template.metadata.name
    );
}

#[tokio::test]
async fn test_mock_ephemeral_worker_handle() {
    let worker_id = WorkerId::new();
    let handle = MockEphemeralWorkerHandle {
        worker_id: worker_id.clone(),
    };

    // Test command execution
    let result = handle
        .execute_command(
            "echo",
            &["test".to_string()],
            Some(Path::new("/tmp")),
            Some(&[("TEST_VAR".to_string(), "test_value".to_string())]),
        )
        .await
        .expect("Failed to execute command");

    assert!(result.success);
    assert_eq!(result.step_id, worker_id.to_string());
    assert!(result.duration_ms > 0);

    // Test health check
    let is_healthy = handle.is_healthy().await.expect("Failed to check health");

    assert!(is_healthy);

    println!("Mock ephemeral worker handle test completed successfully");
}
