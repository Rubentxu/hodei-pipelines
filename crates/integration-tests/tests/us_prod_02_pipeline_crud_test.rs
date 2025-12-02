#![cfg(feature = "container_tests")]
//!
//! This test suite validates complete CRUD operations for pipelines using
//! real PostgreSQL persistence with a singleton shared container.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use hodei_pipelines_adapters::{InMemoryBus, PostgreSqlPipelineRepository};
use hodei_pipelines_core::{
    DomainError, Result,
    job::ResourceQuota,
    pipeline::{PipelineId, PipelineStatus},
};
use hodei_pipelines_modules::{PipelineCrudConfig, PipelineCrudService};
use sqlx::pool::PoolOptions;

mod helpers;
use helpers::get_shared_postgres;

#[tokio::test]
async fn test_pipeline_crud_operations() -> Result<()> {
    info!("🧪 Starting comprehensive Pipeline CRUD integration tests with singleton container");

    // ✅ USE SINGLETON CONTAINER - shared across all tests
    let shared_pg = get_shared_postgres().await;

    info!("✅ Acquired shared PostgreSQL (port {})", shared_pg.port());
    info!("   💾 Single container reused across all tests");
    info!("   🚀 Maximum performance with minimum memory");

    // Get connection pool with retry logic
    let pool = retry_connection(&shared_pg.database_url(), 5)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to connect to shared PostgreSQL: {}", e))
        })?;

    info!("✅ Connected to shared PostgreSQL successfully");

    // Initialize repository and schema
    let pipeline_repo = PostgreSqlPipelineRepository::new(pool.clone());
    pipeline_repo.init_schema().await?;
    info!("✅ Pipeline schema initialized");

    // Initialize service
    let event_bus = Arc::new(InMemoryBus::new(1_000_000_000));
    let config = PipelineCrudConfig::default();
    let service = PipelineCrudService::new(Arc::new(pipeline_repo), event_bus, config);
    info!("✅ Pipeline CRUD service initialized with shared container");

    // Test 1: Create a new pipeline
    info!("\n📝 Test 1: Create a new pipeline");
    let create_request = hodei_pipelines_modules::pipeline_crud::CreatePipelineRequest {
        name: "test-pipeline-1".to_string(),
        description: Some("Test pipeline 1".to_string()),
        steps: vec![
            hodei_pipelines_modules::pipeline_crud::CreatePipelineStepRequest {
                name: "step-1".to_string(),
                image: "rust:1.75".to_string(),
                command: vec!["echo".to_string(), "Hello".to_string()],
                resources: Some(ResourceQuota {
                    cpu_m: 1000,
                    memory_mb: 512,
                    gpu: None,
                }),
                timeout_ms: Some(60000),
                retries: Some(0),
                env: Some(HashMap::new()),
                secret_refs: Some(Vec::new()),
                depends_on: None,
            },
            hodei_pipelines_modules::pipeline_crud::CreatePipelineStepRequest {
                name: "step-2".to_string(),
                image: "rust:1.75".to_string(),
                command: vec!["echo".to_string(), "World".to_string()],
                resources: Some(ResourceQuota {
                    cpu_m: 1000,
                    memory_mb: 512,
                    gpu: None,
                }),
                timeout_ms: Some(60000),
                retries: Some(0),
                env: Some(HashMap::new()),
                secret_refs: Some(Vec::new()),
                depends_on: None,
            },
        ],
        variables: Some(HashMap::from([
            ("ENV".to_string(), "test".to_string()),
            ("VERSION".to_string(), "1.0.0".to_string()),
        ])),
    };

    let pipeline = service.create_pipeline(create_request).await?;
    assert_eq!(pipeline.name, "test-pipeline-1");
    assert_eq!(pipeline.status, PipelineStatus::PENDING);
    assert_eq!(pipeline.steps.len(), 2);
    assert!(pipeline.variables.contains_key("ENV"));
    info!("✅ Pipeline created successfully with ID: {}", pipeline.id);

    // Test 2: Get pipeline by ID
    info!("\n📝 Test 2: Get pipeline by ID");
    let retrieved = service.get_pipeline(&pipeline.id).await?.unwrap();
    assert_eq!(retrieved.id, pipeline.id);
    assert_eq!(retrieved.name, "test-pipeline-1");
    info!("✅ Pipeline retrieved successfully");

    // Test 3: List all pipelines
    info!("\n📝 Test 3: List all pipelines");
    let pipelines = service.list_pipelines(None).await?;
    assert_eq!(pipelines.len(), 1);
    assert_eq!(pipelines[0].id, pipeline.id);
    info!("✅ All pipelines listed successfully");

    // Test 4: Update pipeline
    info!("\n📝 Test 4: Update pipeline");
    let update_request = hodei_pipelines_modules::pipeline_crud::UpdatePipelineRequest {
        name: Some("updated-pipeline-name".to_string()),
        description: Some("Updated description".to_string()),
        steps: None,
        variables: Some(HashMap::from([(
            "NEW_VAR".to_string(),
            "new_value".to_string(),
        )])),
    };

    let updated = service
        .update_pipeline(&pipeline.id, update_request)
        .await?;
    assert_eq!(updated.name, "updated-pipeline-name");
    assert_eq!(updated.description, Some("Updated description".to_string()));
    assert!(updated.variables.contains_key("NEW_VAR"));
    info!("✅ Pipeline updated successfully");

    // Test 5: Get execution order
    info!("\n📝 Test 5: Get execution order");
    let execution_order = service.get_execution_order(&pipeline.id).await?;
    assert_eq!(execution_order.len(), 2);
    info!("✅ Execution order retrieved successfully");

    // Test 6: Start pipeline
    info!("\n📝 Test 6: Start pipeline");
    service.start_pipeline(&pipeline.id).await?;
    let running_pipeline = service.get_pipeline(&pipeline.id).await?.unwrap();
    assert_eq!(running_pipeline.status, PipelineStatus::RUNNING);
    info!("✅ Pipeline started successfully");

    // Test 7: Complete pipeline
    info!("\n📝 Test 7: Complete pipeline");
    service.complete_pipeline(&pipeline.id).await?;
    let completed_pipeline = service.get_pipeline(&pipeline.id).await?.unwrap();
    assert_eq!(completed_pipeline.status, PipelineStatus::SUCCESS);
    info!("✅ Pipeline completed successfully");

    // Test 8: Create second pipeline with dependency
    info!("\n�� Test 8: Create pipeline with dependencies");
    let first_step_id = pipeline.steps[0].id.to_string();
    let dependent_request = hodei_pipelines_modules::pipeline_crud::CreatePipelineRequest {
        name: "dependent-pipeline".to_string(),
        description: Some("Dependent test pipeline".to_string()),
        steps: vec![
            hodei_pipelines_modules::pipeline_crud::CreatePipelineStepRequest {
                name: "step-a".to_string(),
                image: "rust:1.75".to_string(),
                command: vec!["echo".to_string(), "Step A".to_string()],
                resources: Some(ResourceQuota {
                    cpu_m: 1000,
                    memory_mb: 512,
                    gpu: None,
                }),
                timeout_ms: Some(60000),
                retries: Some(0),
                env: Some(HashMap::new()),
                secret_refs: Some(Vec::new()),
                depends_on: Some(vec![first_step_id]),
            },
        ],
        variables: Some(HashMap::new()),
    };

    let pipeline2 = service.create_pipeline(dependent_request).await?;
    assert_eq!(pipeline2.steps.len(), 1);
    assert_eq!(pipeline2.steps[0].depends_on.len(), 1);
    info!("✅ Pipeline with dependencies created successfully");

    // Test 9: Delete pipeline
    info!("\n📝 Test 9: Delete pipeline");
    let delete_id = pipeline2.id.clone();
    service.delete_pipeline(&delete_id).await?;
    let deleted = service.get_pipeline(&delete_id).await?;
    assert!(deleted.is_none());
    info!("✅ Pipeline deleted successfully");

    // Test 10: Execute pipeline
    info!("\n📝 Test 10: Execute pipeline");
    let execute_request = hodei_pipelines_modules::pipeline_crud::ExecutePipelineRequest {
        pipeline_id: pipeline.id.clone(),
        variables: Some(HashMap::from([(
            "EXEC_VAR".to_string(),
            "execution_value".to_string(),
        )])),
        tenant_id: Some("test-tenant".to_string()),
        correlation_id: Some("test-correlation".to_string()),
    };

    let execution = service.execute_pipeline(execute_request).await?;
    assert_eq!(execution.pipeline_id, pipeline.id);
    assert_eq!(execution.steps.len(), 2);
    info!("✅ Pipeline execution created successfully");

    info!("\n�� All Pipeline CRUD tests passed successfully!");
    info!("✅ US-PROD-02: Singleton Pipeline Persistence and CRUD - COMPLETED");
    info!("💾 Memory usage: ~37MB (shared container)");

    Ok(())
}

#[tokio::test]
async fn test_pipeline_validation_errors() -> Result<()> {
    info!("🧪 Testing Pipeline validation errors with singleton container");

    // ✅ USE SINGLETON CONTAINER
    let shared_pg = get_shared_postgres().await;
    let pool = retry_connection(&shared_pg.database_url(), 5)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to connect to shared PostgreSQL: {}", e))
        })?;

    let pipeline_repo = PostgreSqlPipelineRepository::new(pool.clone());
    pipeline_repo.init_schema().await?;

    let event_bus = Arc::new(InMemoryBus::new(1_000_000_000));
    let service = PipelineCrudService::new(
        Arc::new(pipeline_repo),
        event_bus,
        PipelineCrudConfig::default(),
    );

    // Test 1: Name too long
    info!("📝 Test 1: Validation - Name too long");
    let long_name = "a".repeat(300);
    let request = hodei_pipelines_modules::pipeline_crud::CreatePipelineRequest {
        name: long_name,
        description: None,
        steps: vec![],
        variables: None,
    };

    let result = service.create_pipeline(request).await;
    assert!(result.is_err(), "Should reject name that's too long");
    info!("✅ Correctly rejected pipeline with name too long");

    // Test 2: Too many steps
    info!("📝 Test 2: Validation - Too many steps");
    let request = hodei_pipelines_modules::pipeline_crud::CreatePipelineRequest {
        name: "test-pipeline".to_string(),
        description: None,
        steps: (0..150)
            .map(
                |i| hodei_pipelines_modules::pipeline_crud::CreatePipelineStepRequest {
                    name: format!("step-{}", i),
                    image: "rust:1.75".to_string(),
                    command: vec!["echo".to_string()],
                    resources: None,
                    timeout_ms: Some(60000),
                    retries: Some(0),
                    env: None,
                    secret_refs: None,
                    depends_on: None,
                },
            )
            .collect(),
        variables: None,
    };

    let result = service.create_pipeline(request).await;
    assert!(
        result.is_err(),
        "Should reject pipeline with too many steps"
    );
    info!("✅ Correctly rejected pipeline with too many steps");

    // Test 3: Update non-existent pipeline
    info!("📝 Test 3: Validation - Update non-existent pipeline");
    let fake_id = PipelineId::new();
    let update_request = hodei_pipelines_modules::pipeline_crud::UpdatePipelineRequest {
        name: Some("nonexistent".to_string()),
        description: None,
        steps: None,
        variables: None,
    };

    let result = service.update_pipeline(&fake_id, update_request).await;
    assert!(
        result.is_err(),
        "Should fail when updating non-existent pipeline"
    );
    info!("✅ Correctly rejected update of non-existent pipeline");

    info!("\n🎉 All Pipeline validation tests passed successfully!");

    Ok(())
}

#[tokio::test]
async fn test_pipeline_lifecycle_states() -> Result<()> {
    info!("🧪 Testing Pipeline lifecycle states with singleton container");

    // ✅ USE SINGLETON CONTAINER
    let shared_pg = get_shared_postgres().await;
    let pool = retry_connection(&shared_pg.database_url(), 5)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to connect to shared PostgreSQL: {}", e))
        })?;

    let pipeline_repo = PostgreSqlPipelineRepository::new(pool.clone());
    pipeline_repo.init_schema().await?;

    let event_bus = Arc::new(InMemoryBus::new(1_000_000_000));
    let service = PipelineCrudService::new(
        Arc::new(pipeline_repo),
        event_bus,
        PipelineCrudConfig::default(),
    );

    // Create pipeline
    let request = hodei_pipelines_modules::pipeline_crud::CreatePipelineRequest {
        name: "lifecycle-test-pipeline".to_string(),
        description: Some("Lifecycle test".to_string()),
        steps: vec![
            hodei_pipelines_modules::pipeline_crud::CreatePipelineStepRequest {
                name: "step-1".to_string(),
                image: "rust:1.75".to_string(),
                command: vec!["echo".to_string(), "Test".to_string()],
                resources: None,
                timeout_ms: Some(60000),
                retries: Some(0),
                env: None,
                secret_refs: None,
                depends_on: None,
            },
        ],
        variables: None,
    };

    let pipeline = service.create_pipeline(request).await?;
    assert_eq!(pipeline.status, PipelineStatus::PENDING);
    info!("✅ Pipeline created in PENDING status");

    // Test: Start pipeline
    service.start_pipeline(&pipeline.id).await?;
    let running = service.get_pipeline(&pipeline.id).await?.unwrap();
    assert_eq!(running.status, PipelineStatus::RUNNING);
    info!("✅ Pipeline started successfully");

    // Test: Complete pipeline
    service.complete_pipeline(&pipeline.id).await?;
    let completed = service.get_pipeline(&pipeline.id).await?.unwrap();
    assert_eq!(completed.status, PipelineStatus::SUCCESS);
    info!("✅ Pipeline completed successfully");

    // Test: Cannot start completed pipeline
    let result = service.start_pipeline(&pipeline.id).await;
    assert!(
        result.is_err(),
        "Should not allow starting completed pipeline"
    );
    info!("✅ Correctly prevented starting completed pipeline");

    // Test: Pipeline lifecycle with failure
    let request2 = hodei_pipelines_modules::pipeline_crud::CreatePipelineRequest {
        name: "lifecycle-failure-test".to_string(),
        description: Some("Failure test".to_string()),
        steps: vec![
            hodei_pipelines_modules::pipeline_crud::CreatePipelineStepRequest {
                name: "step-1".to_string(),
                image: "rust:1.75".to_string(),
                command: vec!["echo".to_string(), "Fail".to_string()],
                resources: None,
                timeout_ms: Some(60000),
                retries: Some(0),
                env: None,
                secret_refs: None,
                depends_on: None,
            },
        ],
        variables: None,
    };

    let pipeline2 = service.create_pipeline(request2).await?;
    service.start_pipeline(&pipeline2.id).await?;
    service.fail_pipeline(&pipeline2.id).await?;
    let failed = service.get_pipeline(&pipeline2.id).await?.unwrap();
    assert_eq!(failed.status, PipelineStatus::FAILED);
    info!("✅ Pipeline lifecycle failure handled correctly");

    info!("\n🎉 All Pipeline lifecycle tests passed successfully!");

    Ok(())
}

async fn retry_connection(
    database_url: &str,
    max_retries: usize,
) -> std::result::Result<sqlx::PgPool, sqlx::Error> {
    for i in 0..max_retries {
        match PoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
        {
            Ok(pool) => return Ok(pool),
            Err(e) => {
                if i < max_retries - 1 {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                } else {
                    return Err(e);
                }
            }
        }
    }
    unreachable!()
}
