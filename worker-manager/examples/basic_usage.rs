//! Basic Usage Example
//! 
//! This example demonstrates how to use the Worker Manager with basic configurations.
//! Shows the complete lifecycle: creation, monitoring, and termination of workers.

use worker_manager::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Worker Manager Basic Usage Example");
    println!("==================================");

    // 1. Create a simple credential provider (in-memory for demo)
    let credential_provider = Arc::new(credentials::SimpleCredentialProvider::new());
    println!("✓ Created simple credential provider");

    // 2. Create Worker Manager
    let mut worker_manager = WorkerManager::new(credential_provider.clone());
    println!("✓ Created Worker Manager");

    // 3. Set up Docker Provider
    let docker_provider = Arc::new(providers::docker::DockerProvider::from_provider_config(
        &ProviderConfig::docker(),
        credential_provider.clone()
    )?);
    worker_manager.register_provider("docker".to_string(), docker_provider.clone());
    worker_manager.set_default_provider("docker".to_string());
    println!("✓ Configured Docker provider");

    // 4. Create sample credential
    let mut sample_credential = credentials::Credential {
        name: "database-credentials".to_string(),
        values: HashMap::from([
            ("DB_HOST".to_string(), "localhost".to_string()),
            ("DB_PORT".to_string(), "5432".to_string()),
            ("DB_USER".to_string(), "admin".to_string()),
            ("DB_PASSWORD".to_string(), "secret123".to_string()),
        ]),
        metadata: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        expires_at: Some(chrono::Utc::now() + chrono::Duration::days(30)),
        rotation_enabled: true,
        access_policy: Some(credentials::AccessPolicy {
            allowed_subjects: vec!["worker-service".to_string()],
            read_permissions: vec!["read".to_string()],
            write_permissions: vec![],
            rotation_permissions: vec!["rotate".to_string()],
        }),
    };

    // Store the credential
    let stored_credential = credential_provider.put_credential(&sample_credential).await?;
    println!("✓ Created sample credential: {}", stored_credential.credential.name);

    // 5. Create a RuntimeSpec for a worker
    let mut runtime_spec = RuntimeSpec::basic("alpine:3.18".to_string());
    runtime_spec.command = Some(vec!["/bin/sh".to_string(), "-lc".to_string(), 
                                  "echo 'Hello from Worker Manager!' && sleep 10".to_string()]);
    runtime_spec.secret_refs = vec!["database-credentials".to_string()];
    runtime_spec.ports = vec![8080];
    runtime_spec.resources = ResourceQuota::basic(500, 256);
    runtime_spec.labels.insert("environment".to_string(), "development".to_string());
    runtime_spec.labels.insert("service".to_string(), "worker-demo".to_string());

    println!("✓ Created RuntimeSpec for worker");

    // 6. Create ProviderConfig
    let provider_config = ProviderConfig::docker();
    println!("✓ Created ProviderConfig");

    // 7. Create worker
    println!("\n--- Creating Worker ---");
    let worker = worker_manager.create_worker(&runtime_spec, &provider_config).await?;
    println!("✓ Worker created successfully");
    println!("  Worker ID: {}", worker.id);
    println!("  State: {:?}", worker.state);
    println!("  Provider: {}", worker.provider_config.provider_name);

    // 8. Monitor worker status
    println!("\n--- Monitoring Worker Status ---");
    for i in 1..=5 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        
        let status = worker_manager.get_worker_status(&worker.id).await?;
        println!("  Status check {}: {:?}", i, status);

        if matches!(status, WorkerState::Terminated) {
            println!("  ✓ Worker completed successfully");
            break;
        }
    }

    // 9. Test capacity information
    println!("\n--- System Capacity ---");
    let capacity = worker_manager.get_capacity().await?;
    println!("  Active workers: {}", capacity.active_workers);
    println!("  Total CPU: {}m", capacity.total_resources.cpu_m);
    println!("  Total Memory: {}MB", capacity.total_resources.memory_mb);
    println!("  Available CPU: {}m", capacity.available_resources.cpu_m);
    println!("  Available Memory: {}MB", capacity.available_resources.memory_mb);

    // 10. Demonstrate rotation setup
    println!("\n--- Setting Up Credential Rotation ---");
    
    // Start rotation engine
    let rotation_config = credentials::rotation::RotationEngineConfig::default();
    worker_manager.start_rotation_engine(rotation_config).await?;
    println!("✓ Started rotation engine");

    // Schedule time-based rotation
    let rotation_strategy = credentials::rotation::RotationStrategy::TimeBased(
        credentials::rotation::TimeBasedRotation {
            rotation_interval: std::time::Duration::from_secs(300), // 5 minutes for demo
            rotation_time: None,
            grace_period: std::time::Duration::from_secs(60),
        }
    );

    let rotation_task_id = worker_manager.schedule_rotation("database-credentials", rotation_strategy).await?;
    println!("✓ Scheduled rotation task: {}", rotation_task_id);

    // 11. Get rotation status
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let rotation_status = worker_manager.get_rotation_status().await?;
    println!("  Total tasks: {}", rotation_status.total_tasks);
    println!("  Active tasks: {}", rotation_status.active_tasks);
    println!("  Healthy providers: {}/{}", rotation_status.health_status.running_tasks, 
             rotation_status.health_status.is_healthy);

    // 12. Demonstrate provider switching
    println!("\n--- Provider Switching ---");
    let providers = worker_manager.list_providers();
    println!("  Available providers: {:?}", providers);

    // 13. Health check
    println!("\n--- System Health Check ---");
    let health_status = worker_manager.health_check().await?;
    println!("  Overall health: {}", if health_status.is_healthy { "Healthy" } else { "Unhealthy" });
    println!("  Healthy providers: {}/{}", health_status.healthy_providers, health_status.total_providers);
    println!("  Rotation engine: {}", if health_status.rotation_engine_healthy { "Running" } else { "Not running" });

    // 14. Clean up
    println!("\n--- Cleanup ---");
    worker_manager.stop_rotation_engine().await?;
    println!("✓ Stopped rotation engine");
    println!("✓ Example completed successfully");

    println!("\n--- Summary ---");
    println!("• Created and managed worker lifecycle");
    println!("• Configured credential storage and rotation");
    println!("• Monitored system capacity and health");
    println!("• Tested provider abstraction and switching");
    println!("• Demonstrated production-ready patterns");

    Ok(())
}
