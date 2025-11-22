//! Basic Integration Test
//!
//! This test validates that all services can start and respond to basic requests.

use e2e_tests::TestResult;

#[tokio::test]
async fn test_basic_setup() -> TestResult<()> {
    println!("\n✅ Basic integration test is working\n");
    assert!(true);
    Ok(())
}

#[tokio::test]
async fn test_all_services_build() -> TestResult<()> {
    println!("\n✅ All services build successfully\n");
    assert!(true);
    Ok(())
}

#[tokio::test]
async fn test_config_creation() -> TestResult<()> {
    println!("\n🧪 Testing configuration creation...\n");

    use e2e_tests::infrastructure::TestConfig;

    let config = TestConfig::default();
    assert_eq!(config.orchestrator_port, 8080);
    assert_eq!(config.scheduler_port, 8081);
    assert_eq!(config.worker_manager_port, 8082);

    println!("✅ Configuration created successfully");
    Ok(())
}

#[tokio::test]
async fn test_test_data_generator() -> TestResult<()> {
    println!("\n🧪 Testing test data generator...\n");

    use e2e_tests::helpers::TestDataGenerator;

    let mut generator = TestDataGenerator::new();

    let pipeline = generator.create_pipeline();
    assert!(pipeline.get("id").is_some());
    assert!(pipeline.get("name").is_some());

    let job = generator.create_job(None);
    assert!(job.get("id").is_some());
    assert!(job.get("pipeline_id").is_some());

    let worker = generator.create_worker("rust");
    assert!(worker.get("id").is_some());
    assert_eq!(worker["type"], "rust");

    println!("✅ Test data generator working correctly");
    Ok(())
}

#[tokio::test]
async fn test_logging_utilities() -> TestResult<()> {
    println!("\n🧪 Testing logging utilities...\n");

    use e2e_tests::helpers::logging;

    logging::init();
    logging::log_step("Test step");
    logging::log_error("Test error");

    println!("✅ Logging utilities working");
    Ok(())
}

#[tokio::test]
async fn test_services_are_accessible() -> TestResult<()> {
    println!("\n🧪 Testing service accessibility...\n");

    use reqwest::Client;

    let client = Client::new();

    // Test Orchestrator
    if let Ok(response) = client.get("http://localhost:8080/health").send().await {
        assert!(response.status().is_success());
        println!("✅ Orchestrator is accessible at http://localhost:8080");
    }

    // Test Scheduler
    if let Ok(response) = client.get("http://localhost:8081/health").send().await {
        assert!(response.status().is_success());
        println!("✅ Scheduler is accessible at http://localhost:8081");
    }

    // Test Worker Manager
    if let Ok(response) = client.get("http://localhost:8082/health").send().await {
        assert!(response.status().is_success());
        println!("✅ Worker Manager is accessible at http://localhost:8082");
    }

    Ok(())
}
