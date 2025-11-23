//! Real services E2E test
//!
//! This test validates actual functionality by calling the running HTTP services
//!
//! NOTE: These tests require the worker-lifecycle-manager service to be running
//! on port 8082. Run with: make test-all-e2e-services

use e2e_tests::TestResult;
use reqwest::Client;
use serde_json::{json, Value};

#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_real_pipeline_creation_and_retrieval() -> TestResult<()> {
    println!("\n🧪 Testing REAL pipeline creation and retrieval...\n");

    let client = Client::new();

    // Create a pipeline
    let pipeline_data = json!({
        "name": "real-test-pipeline",
        "description": "Testing real HTTP API functionality"
    });

    let response = client
        .post("http://localhost:8080/api/v1/pipelines")
        .json(&pipeline_data)
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "Pipeline creation should succeed"
    );

    let created_pipeline: Value = response.json().await?;
    let pipeline_id = created_pipeline["id"].as_str().unwrap();

    println!("   ✅ Pipeline created with ID: {}", pipeline_id);
    assert!(
        created_pipeline.get("id").is_some(),
        "Pipeline should have ID"
    );
    assert_eq!(created_pipeline["name"], "real-test-pipeline");

    // Retrieve the pipeline
    let get_response = client
        .get(&format!(
            "http://localhost:8080/api/v1/pipelines/{}",
            pipeline_id
        ))
        .send()
        .await?;

    assert!(
        get_response.status().is_success(),
        "Pipeline retrieval should succeed"
    );

    let retrieved_pipeline: Value = get_response.json().await?;
    assert_eq!(retrieved_pipeline["id"], pipeline_id);
    assert_eq!(retrieved_pipeline["name"], "real-test-pipeline");

    println!("   ✅ Pipeline retrieved successfully");

    // List all pipelines
    let list_response = client
        .get("http://localhost:8080/api/v1/pipelines")
        .send()
        .await?;

    assert!(
        list_response.status().is_success(),
        "Pipeline listing should succeed"
    );

    let pipelines: Value = list_response.json().await?;
    assert!(pipelines.is_array(), "Should return an array");

    println!(
        "   ✅ Pipeline list retrieved with {} items",
        pipelines.as_array().unwrap().len()
    );

    println!("\n✅ All pipeline tests passed!\n");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_real_job_creation_and_tracking() -> TestResult<()> {
    println!("\n🧪 Testing REAL job creation and tracking...\n");

    let client = Client::new();

    // First create a pipeline
    let pipeline_data = json!({
        "name": "job-test-pipeline",
        "description": "Pipeline for job testing"
    });

    let pipeline_response = client
        .post("http://localhost:8080/api/v1/pipelines")
        .json(&pipeline_data)
        .send()
        .await?;

    let pipeline: Value = pipeline_response.json().await?;
    let pipeline_id = pipeline["id"].as_str().unwrap();

    println!("   ✅ Pipeline created for job testing: {}", pipeline_id);

    // Create a job
    let job_data = json!({
        "pipeline_id": pipeline_id
    });

    let job_response = client
        .post("http://localhost:8080/api/v1/jobs")
        .json(&job_data)
        .send()
        .await?;

    assert!(
        job_response.status().is_success(),
        "Job creation should succeed"
    );

    let created_job: Value = job_response.json().await?;
    let job_id = created_job["id"].as_str().unwrap();

    println!("   ✅ Job created with ID: {}", job_id);
    assert!(created_job.get("id").is_some(), "Job should have ID");
    assert_eq!(created_job["pipeline_id"], pipeline_id);

    // List all jobs
    let jobs_response = client
        .get("http://localhost:8080/api/v1/jobs")
        .send()
        .await?;

    assert!(
        jobs_response.status().is_success(),
        "Job listing should succeed"
    );

    let jobs: Value = jobs_response.json().await?;
    assert!(jobs.is_array(), "Should return an array");

    println!(
        "   ✅ Job list retrieved with {} items",
        jobs.as_array().unwrap().len()
    );

    println!("\n✅ All job tests passed!\n");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_real_worker_registration_and_management() -> TestResult<()> {
    println!("\n🧪 Testing REAL worker registration and management...\n");

    let client = Client::new();

    // Register a worker
    let worker_data = json!({
        "type": "rust",
        "name": "test-worker"
    });

    let register_response = client
        .post("http://localhost:8081/api/v1/workers")
        .json(&worker_data)
        .send()
        .await?;

    assert!(
        register_response.status().is_success(),
        "Worker registration should succeed"
    );

    let worker: Value = register_response.json().await?;
    let worker_id = worker["id"].as_str().unwrap();

    println!("   ✅ Worker registered with ID: {}", worker_id);
    assert!(worker.get("id").is_some(), "Worker should have ID");
    assert_eq!(worker["type"], "rust");

    // List all workers
    let workers_response = client
        .get("http://localhost:8081/api/v1/workers")
        .send()
        .await?;

    assert!(
        workers_response.status().is_success(),
        "Worker listing should succeed"
    );

    let workers: Value = workers_response.json().await?;
    assert!(workers.is_array(), "Should return an array");

    println!(
        "   ✅ Worker list retrieved with {} items",
        workers.as_array().unwrap().len()
    );

    println!("\n✅ All worker tests passed!\n");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_real_worker_lifecycle_management() -> TestResult<()> {
    println!("\n🧪 Testing REAL worker lifecycle management...\n");

    let client = Client::new();

    // Start a worker
    let worker_data = json!({
        "type": "python"
    });

    let start_response = client
        .post("http://localhost:8082/api/v1/workers")
        .json(&worker_data)
        .send()
        .await?;

    assert!(
        start_response.status().is_success(),
        "Worker start should succeed"
    );

    let worker: Value = start_response.json().await?;
    let worker_id = worker["id"].as_str().unwrap();

    println!("   ✅ Worker started with ID: {}", worker_id);
    assert_eq!(worker["status"], "running");

    // Get worker status
    let status_response = client
        .get(&format!(
            "http://localhost:8082/api/v1/workers/{}",
            worker_id
        ))
        .send()
        .await?;

    assert!(
        status_response.status().is_success(),
        "Worker status should be accessible"
    );

    let worker_status: Value = status_response.json().await?;
    assert_eq!(worker_status["id"], worker_id);

    println!("   ✅ Worker status retrieved successfully");

    // List workers
    let list_response = client
        .get("http://localhost:8082/api/v1/workers")
        .send()
        .await?;

    assert!(
        list_response.status().is_success(),
        "Worker listing should succeed"
    );

    let workers: Value = list_response.json().await?;
    println!(
        "   ✅ Worker list retrieved with {} items",
        workers.as_array().unwrap().len()
    );

    println!("\n✅ All worker lifecycle tests passed!\n");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_real_job_execution() -> TestResult<()> {
    println!("\n🧪 Testing REAL job execution...\n");

    let client = Client::new();

    // Execute a job
    let execution_data = json!({
        "command": "echo 'Hello from worker'",
        "type": "bash"
    });

    let exec_response = client
        .post("http://localhost:8082/api/v1/execute")
        .json(&execution_data)
        .send()
        .await?;

    assert!(
        exec_response.status().is_success(),
        "Job execution should succeed"
    );

    let execution: Value = exec_response.json().await?;
    let execution_id = execution["id"].as_str().unwrap();

    println!("   ✅ Job execution started with ID: {}", execution_id);
    assert!(execution.get("id").is_some(), "Execution should have ID");

    // List executions
    let list_response = client
        .get("http://localhost:8082/api/v1/executions")
        .send()
        .await?;

    assert!(
        list_response.status().is_success(),
        "Execution listing should succeed"
    );

    let executions: Value = list_response.json().await?;
    println!(
        "   ✅ Execution list retrieved with {} items",
        executions.as_array().unwrap().len()
    );

    println!("\n✅ All job execution tests passed!\n");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_complete_workflow() -> TestResult<()> {
    println!("\n🧪 Testing COMPLETE end-to-end workflow...\n");

    let client = Client::new();

    // Step 1: Create a pipeline
    println!("   1️⃣  Creating pipeline...");
    let pipeline_data = json!({
        "name": "complete-workflow-pipeline",
        "description": "End-to-end workflow test"
    });

    let pipeline_response = client
        .post("http://localhost:8080/api/v1/pipelines")
        .json(&pipeline_data)
        .send()
        .await?;

    let pipeline: Value = pipeline_response.json().await?;
    let pipeline_id = pipeline["id"].as_str().unwrap();
    println!("      ✅ Pipeline created: {}", pipeline_id);

    // Step 2: Create a job
    println!("   2️⃣  Creating job...");
    let job_data = json!({
        "pipeline_id": pipeline_id
    });

    let job_response = client
        .post("http://localhost:8080/api/v1/jobs")
        .json(&job_data)
        .send()
        .await?;

    let job: Value = job_response.json().await?;
    let job_id = job["id"].as_str().unwrap();
    println!("      ✅ Job created: {}", job_id);

    // Step 3: Register a worker
    println!("   3️⃣  Registering worker...");
    let worker_data = json!({
        "type": "rust"
    });

    let worker_response = client
        .post("http://localhost:8081/api/v1/workers")
        .json(&worker_data)
        .send()
        .await?;

    let worker: Value = worker_response.json().await?;
    let worker_id = worker["id"].as_str().unwrap();
    println!("      ✅ Worker registered: {}", worker_id);

    // Step 4: Start worker in worker manager
    println!("   4️⃣  Starting worker...");
    let started_worker_response = client
        .post("http://localhost:8082/api/v1/workers")
        .json(&json!({"type": "rust"}))
        .send()
        .await?;

    let started_worker: Value = started_worker_response.json().await?;
    let started_worker_id = started_worker["id"].as_str().unwrap();
    println!("      ✅ Worker started: {}", started_worker_id);

    // Step 5: Execute a job
    println!("   5️⃣  Executing job...");
    let execution_response = client
        .post("http://localhost:8082/api/v1/execute")
        .json(&json!({"job_id": job_id}))
        .send()
        .await?;

    let execution: Value = execution_response.json().await?;
    let execution_id = execution["id"].as_str().unwrap();
    println!("      ✅ Job execution started: {}", execution_id);

    // Verify all resources were created
    println!("   🔍 Verifying all resources...");

    let pipelines = client
        .get("http://localhost:8080/api/v1/pipelines")
        .send()
        .await?;
    let pipelines_list: Value = pipelines.json().await?;
    assert!(
        pipelines_list.as_array().unwrap().len() > 0,
        "Should have pipelines"
    );
    println!(
        "      ✅ Pipelines: {}",
        pipelines_list.as_array().unwrap().len()
    );

    let jobs = client
        .get("http://localhost:8080/api/v1/jobs")
        .send()
        .await?;
    let jobs_list: Value = jobs.json().await?;
    assert!(jobs_list.as_array().unwrap().len() > 0, "Should have jobs");
    println!("      ✅ Jobs: {}", jobs_list.as_array().unwrap().len());

    let workers = client
        .get("http://localhost:8081/api/v1/workers")
        .send()
        .await?;
    let workers_list: Value = workers.json().await?;
    assert!(
        workers_list.as_array().unwrap().len() > 0,
        "Should have workers"
    );
    println!(
        "      ✅ Workers: {}",
        workers_list.as_array().unwrap().len()
    );

    let executions = client
        .get("http://localhost:8082/api/v1/executions")
        .send()
        .await?;
    let executions_list: Value = executions.json().await?;
    assert!(
        executions_list.as_array().unwrap().len() > 0,
        "Should have executions"
    );
    println!(
        "      ✅ Executions: {}",
        executions_list.as_array().unwrap().len()
    );

    println!("\n✅ COMPLETE WORKFLOW TEST PASSED!\n");
    println!("   Successfully created and managed:");
    println!("     - Pipeline: {}", pipeline_id);
    println!("     - Job: {}", job_id);
    println!("     - Worker (Scheduler): {}", worker_id);
    println!("     - Worker (Worker Manager): {}", started_worker_id);
    println!("     - Execution: {}", execution_id);

    Ok(())
}
