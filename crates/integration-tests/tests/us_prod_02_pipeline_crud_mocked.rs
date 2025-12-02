//! US-PROD-02: Pipeline CRUD Integration Tests with Mocks
//!
//! This test suite validates complete CRUD operations for pipelines using
//! mocked repositories, avoiding the need for Docker containers.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::info;

use hodei_pipelines_adapters::InMemoryBus;
use hodei_pipelines_core::{
    Result,
    job::ResourceQuota,
    pipeline::{Pipeline, PipelineId, PipelineStatus},
};
use hodei_pipelines_modules::{PipelineCrudConfig, PipelineCrudService};
use hodei_pipelines_ports::{EventSubscriber, MockPipelineRepository};

#[tokio::test]
async fn test_pipeline_crud_operations_mocked() -> Result<()> {
    info!("🧪 Starting Pipeline CRUD integration tests with MOCKS");

    // Setup Mock Repository
    let mut mock_repo = MockPipelineRepository::new();

    // We'll use a simple in-memory storage to simulate DB behavior in the mock
    // This allows us to verify state changes without complex expectation sequences
    let pipelines_db: Arc<Mutex<HashMap<PipelineId, Pipeline>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Clone for closures
    let db_clone_save = pipelines_db.clone();
    let db_clone_get = pipelines_db.clone();
    let db_clone_delete = pipelines_db.clone();
    let db_clone_list = pipelines_db.clone();

    // Mock save_pipeline
    mock_repo.expect_save_pipeline().returning(move |pipeline| {
        let mut db = db_clone_save.lock().unwrap();
        db.insert(pipeline.id.clone(), pipeline.clone());
        Ok(())
    });

    // Mock get_pipeline
    mock_repo.expect_get_pipeline().returning(move |id| {
        let db = db_clone_get.lock().unwrap();
        Ok(db.get(id).cloned())
    });

    // Mock delete_pipeline
    mock_repo.expect_delete_pipeline().returning(move |id| {
        let mut db = db_clone_delete.lock().unwrap();
        db.remove(id);
        Ok(())
    });

    // Mock get_all_pipelines
    mock_repo.expect_get_all_pipelines().returning(move || {
        let db = db_clone_list.lock().unwrap();
        Ok(db.values().cloned().collect())
    });

    // Initialize service
    let event_bus = Arc::new(InMemoryBus::new(1000));
    let config = PipelineCrudConfig::default();
    let service = PipelineCrudService::new(Arc::new(mock_repo), event_bus.clone(), config);

    // Subscribe to event bus to prevent "No subscribers" error (mapped to "Bus full")
    let _rx = event_bus.subscribe().await.unwrap();

    info!("✅ Pipeline CRUD service initialized with MOCK repository");

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
        ],
        variables: Some(HashMap::from([("ENV".to_string(), "test".to_string())])),
    };

    let pipeline = service.create_pipeline(create_request).await?;
    assert_eq!(pipeline.name, "test-pipeline-1");
    assert_eq!(pipeline.status, PipelineStatus::PENDING);
    assert_eq!(pipeline.steps.len(), 1);
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
        variables: None,
    };

    let updated = service
        .update_pipeline(&pipeline.id, update_request)
        .await?;
    assert_eq!(updated.name, "updated-pipeline-name");
    assert_eq!(updated.description, Some("Updated description".to_string()));
    info!("✅ Pipeline updated successfully");

    // Test 5: Start pipeline
    info!("\n📝 Test 5: Start pipeline");
    service.start_pipeline(&pipeline.id).await?;
    let running_pipeline = service.get_pipeline(&pipeline.id).await?.unwrap();
    assert_eq!(running_pipeline.status, PipelineStatus::RUNNING);
    info!("✅ Pipeline started successfully");

    // Test 6: Complete pipeline
    info!("\n📝 Test 6: Complete pipeline");
    service.complete_pipeline(&pipeline.id).await?;
    let completed_pipeline = service.get_pipeline(&pipeline.id).await?.unwrap();
    assert_eq!(completed_pipeline.status, PipelineStatus::SUCCESS);
    info!("✅ Pipeline completed successfully");

    // Test 7: Delete pipeline
    info!("\n📝 Test 7: Delete pipeline");
    // Note: In real logic, we might need to force delete or archive if it's completed.
    // The mock logic allows deletion.
    service.delete_pipeline(&pipeline.id).await?;
    let deleted = service.get_pipeline(&pipeline.id).await?;
    assert!(deleted.is_none());
    info!("✅ Pipeline deleted successfully");

    info!("\n🎉 All Mocked Pipeline CRUD tests passed successfully!");
    info!("✅ Memory usage: Minimal (No containers)");

    Ok(())
}
