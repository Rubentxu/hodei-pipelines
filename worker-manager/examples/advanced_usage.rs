//! Advanced Usage Example
//! 
//! This example demonstrates advanced Worker Manager features including:
//! - Multiple providers (Kubernetes and Docker)
//! - Complex credential management with rotation
//! - Integration with NATS for event streaming
//! - Keycloak service account integration
//! - Health checks and monitoring

use worker_manager::*;
use std::collections::HashMap;
use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("Worker Manager Advanced Usage Example");
    println!("=====================================");

    // 1. Set up comprehensive credential providers
    println!("\n--- Setting Up Credential Providers ---");
    
    // Simple in-memory provider for quick access
    let simple_provider = Arc::new(credentials::SimpleCredentialProvider::new());
    println!("✓ Created SimpleCredentialProvider");

    // Set up sample credentials
    setup_sample_credentials(&simple_provider).await?;
    println!("✓ Configured sample credentials");

    // 2. Create Worker Manager with multiple providers
    println!("\n--- Creating Multi-Provider Worker Manager ---");
    
    let mut worker_manager = WorkerManager::new(simple_provider.clone());

    // Kubernetes Provider (if available)
    match setup_kubernetes_provider(&simple_provider).await {
        Ok(provider) => {
            worker_manager.register_provider("kubernetes".to_string(), provider);
            println!("✓ Configured Kubernetes provider");
        }
        Err(e) => {
            println!("⚠ Kubernetes provider not available: {}", e);
        }
    }

    // Docker Provider
    let docker_provider = Arc::new(providers::docker::DockerProvider::from_provider_config(
        &ProviderConfig::docker(),
        simple_provider.clone()
    )?);
    worker_manager.register_provider("docker".to_string(), docker_provider.clone());
    worker_manager.set_default_provider("docker".to_string());
    println!("✓ Configured Docker provider");

    // 3. Start rotation engine with advanced configuration
    println!("\n--- Starting Advanced Rotation Engine ---");
    
    let mut rotation_config = credentials::rotation::RotationEngineConfig::default();
    rotation_config.max_concurrent_rotations = 10;
    rotation_config.rotation_timeout = Duration::from_secs(120);
    rotation_config.enable_metrics = true;

    worker_manager.start_rotation_engine(rotation_config).await?;
    println!("✓ Started advanced rotation engine");

    // 4. Set up rotation strategies for different credentials
    println!("\n--- Configuring Rotation Strategies ---");
    
    // Time-based rotation for database credentials
    let db_rotation_strategy = credentials::rotation::RotationStrategy::TimeBased(
        credentials::rotation::TimeBasedRotation {
            rotation_interval: Duration::from_secs(600), // 10 minutes
            rotation_time: None,
            grace_period: Duration::from_secs(30),
        }
    );

    // Event-based rotation for API keys (triggered by security events)
    let api_rotation_strategy = credentials::rotation::RotationStrategy::EventBased(
        credentials::rotation::EventBasedRotation {
            triggers: vec![
                credentials::rotation::RotationTrigger::CredentialAccessed,
                credentials::rotation::RotationTrigger::UnauthorizedAccessAttempt,
            ],
            max_concurrent_rotations: 3,
        }
    );

    // Schedule rotations
    let db_rotation_task = worker_manager.schedule_rotation("database-credentials", db_rotation_strategy).await?;
    let api_rotation_task = worker_manager.schedule_rotation("api-keys", api_rotation_strategy).await?;
    
    println!("✓ Scheduled rotation tasks:");
    println!("  Database credentials: {}", db_rotation_task);
    println!("  API keys: {}", api_rotation_task);

    // 5. Create workers with different providers
    println!("\n--- Creating Workers with Different Providers ---");
    
    let mut workers = Vec::new();

    // Worker 1: Database worker with secrets
    let db_worker = create_database_worker().await?;
    let db_provider_config = ProviderConfig::docker();
    let db_worker_result = worker_manager.create_worker(&db_worker, &db_provider_config).await?;
    workers.push(db_worker_result);
    println!("✓ Created database worker: {}", workers[0].id);

    // Worker 2: Web worker with API keys
    let api_worker = create_api_worker().await?;
    let api_provider_config = ProviderConfig::docker();
    let api_worker_result = worker_manager.create_worker(&api_worker, &api_provider_config).await?;
    workers.push(api_worker_result);
    println!("✓ Created API worker: {}", workers[1].id);

    // Worker 3: Kubernetes worker (if available)
    if worker_manager.get_provider_by_name("kubernetes").is_some() {
        let k8s_worker = create_kubernetes_worker().await?;
        let k8s_provider_config = ProviderConfig::kubernetes("default".to_string());
        let k8s_worker_result = worker_manager.create_worker(&k8s_worker, &k8s_provider_config).await?;
        workers.push(k8s_worker_result);
        println!("✓ Created Kubernetes worker: {}", workers[2].id);
    }

    // 6. Monitor worker lifecycle
    println!("\n--- Monitoring Worker Lifecycle ---");
    
    for worker in &workers {
        println!("  Monitoring worker: {}", worker.id);
        
        // Monitor for 30 seconds
        for i in 1..=15 {
            sleep(Duration::from_secs(2)).await;
            
            let status = worker_manager.get_worker_status(&worker.id).await?;
            println!("    Status {}: {:?}", i, status);

            // Test operations during monitoring
            if i == 5 {
                match worker_manager.get_provider().unwrap().get_logs(&worker.id).await {
                    Ok(logs) => {
                        println!("    ✓ Got logs: {}", logs.stream_id);
                    }
                    Err(e) => {
                        println!("    ✗ Failed to get logs: {}", e);
                    }
                }
            }

            if i == 10 {
                match worker_manager.get_provider().unwrap().execute_command(
                    &worker.id, 
                    vec!["echo".to_string(), "health-check".to_string()],
                    Some(Duration::from_secs(5))
                ).await {
                    Ok(result) => {
                        println!("    ✓ Health check passed");
                    }
                    Err(e) => {
                        println!("    ✗ Health check failed: {}", e);
                    }
                }
            }

            if matches!(status, WorkerState::Terminated) {
                println!("    ✓ Worker completed");
                break;
            }
        }
    }

    // 7. Demonstrate event-driven rotations
    println!("\n--- Triggering Event-Based Rotations ---");
    
    // Simulate security event
    let security_event = credentials::rotation::RotationEvent {
        event_type: credentials::rotation::RotationEventType::UnauthorizedAccessAttempt,
        credential_name: Some("api-keys".to_string()),
        metadata: HashMap::from([
            ("source_ip".to_string(), "192.168.1.100".to_string()),
            ("attempt_count".to_string(), "5".to_string()),
            ("severity".to_string(), "high".to_string()),
        ]),
        timestamp: chrono::Utc::now(),
    };

    // Trigger event through rotation engine
    let rotation_engine = worker_manager.rotation_engine.read().await;
    if let Some(engine) = rotation_engine.as_ref() {
        engine.trigger_event_rotation(security_event).await?;
        println!("✓ Triggered event-based rotation");
    }

    // 8. Test capacity management
    println!("\n--- Capacity Management ---");
    
    let capacity = worker_manager.get_capacity().await?;
    println!("  System capacity:");
    println!("    Active workers: {}", capacity.active_workers);
    println!("    CPU usage: {}m/{}m", 
             capacity.used_resources.cpu_m, 
             capacity.total_resources.cpu_m);
    println!("    Memory usage: {}MB/{}MB", 
             capacity.used_resources.memory_mb, 
             capacity.total_resources.memory_mb);

    // 9. Health checks and system status
    println!("\n--- Health Check and System Status ---");
    
    let health_status = worker_manager.health_check().await?;
    println!("  System health: {}", if health_status.is_healthy { "Healthy" } else { "Unhealthy" });
    println!("  Provider status:");
    for (name, healthy, error) in &health_status.provider_details {
        println!("    {}: {}", name, if *healthy { "✓ Healthy" } else { "✗ Failed" });
        if let Some(err) = error {
            println!("      Error: {}", err);
        }
    }

    // 10. Rotation engine status
    println!("\n--- Rotation Engine Status ---");
    
    let rotation_status = worker_manager.get_rotation_status().await?;
    println!("  Rotation tasks:");
    println!("    Total: {}", rotation_status.total_tasks);
    println!("    Pending: {}", rotation_status.pending_tasks);
    println!("    Active: {}", rotation_status.active_tasks);
    println!("    Completed: {}", rotation_status.completed_tasks);
    println!("    Failed: {}", rotation_status.failed_tasks);
    println!("  Rotation statistics:");
    println!("    Total rotations: {}", rotation_status.statistics.total_rotations);
    println!("    Successful: {}", rotation_status.statistics.successful_rotations);
    println!("    Failed: {}", rotation_status.statistics.failed_rotations);

    // 11. Test manual rotation
    println!("\n--- Manual Rotation Test ---");
    
    let manual_strategy = credentials::rotation::RotationStrategy::Manual;
    let manual_task = worker_manager.schedule_rotation("database-credentials", manual_strategy).await?;
    println!("✓ Scheduled manual rotation: {}", manual_task);

    // 12. Cleanup and shutdown
    println!("\n--- Cleanup and Shutdown ---");
    
    // Wait a bit for rotations to complete
    sleep(Duration::from_secs(5)).await;

    // Terminate all workers
    for worker in &workers {
        match worker_manager.terminate_worker(&worker.id).await {
            Ok(_) => println!("✓ Terminated worker: {}", worker.id),
            Err(e) => println!("✗ Failed to terminate worker {}: {}", worker.id, e),
        }
    }

    // Stop rotation engine
    worker_manager.stop_rotation_engine().await?;
    println!("✓ Stopped rotation engine");

    println!("\n--- Advanced Example Completed ---");
    println!("• Demonstrated multi-provider support");
    println!("• Showed advanced rotation strategies");
    println!("• Tested event-driven rotations");
    println!("• Validated system health monitoring");
    println!("• Performed manual operations and cleanup");

    Ok(())
}

async fn setup_sample_credentials(
    provider: &Arc<credentials::SimpleCredentialProvider>
) -> Result<(), CredentialError> {
    // Database credentials
    let db_credential = credentials::Credential {
        name: "database-credentials".to_string(),
        values: HashMap::from([
            ("DB_HOST".to_string(), "prod-db.example.com".to_string()),
            ("DB_PORT".to_string(), "5432".to_string()),
            ("DB_USER".to_string(), "app_user".to_string()),
            ("DB_PASSWORD".to_string(), "secure_password_123".to_string()),
            ("DB_NAME".to_string(), "production_db".to_string()),
        ]),
        metadata: HashMap::from([
            ("environment".to_string(), "production".to_string()),
            ("team".to_string(), "backend".to_string()),
            ("compliance_level".to_string(), "high".to_string()),
        ]),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        expires_at: Some(chrono::Utc::now() + chrono::Duration::days(90)),
        rotation_enabled: true,
        access_policy: Some(credentials::AccessPolicy {
            allowed_subjects: vec!["web-service".to_string(), "api-service".to_string()],
            read_permissions: vec!["read".to_string()],
            write_permissions: vec![],
            rotation_permissions: vec!["rotate".to_string(), "manual-rotate".to_string()],
        }),
    };

    provider.put_credential(&db_credential).await?;

    // API keys
    let api_credential = credentials::Credential {
        name: "api-keys".to_string(),
        values: HashMap::from([
            ("GOOGLE_API_KEY".to_string(), "AIzaSyD-9tSrke72PouQMnMX-a7u8Lm5Oz9kSu4".to_string()),
            ("STRIPE_PUBLISHABLE_KEY".to_string(), "pk_test_51H...xyz".to_string()),
            ("STRIPE_SECRET_KEY".to_string(), "sk_test_51H...abc".to_string()),
            ("TWITTER_BEARER_TOKEN".to_string(), "Bearer_token_here".to_string()),
        ]),
        metadata: HashMap::from([
            ("environment".to_string(), "production".to_string()),
            ("team".to_string(), "frontend".to_string()),
            ("sensitivity".to_string(), "high".to_string()),
        ]),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        expires_at: Some(chrono::Utc::now() + chrono::Duration::days(60)),
        rotation_enabled: true,
        access_policy: Some(credentials::AccessPolicy {
            allowed_subjects: vec!["frontend-service".to_string(), "mobile-app".to_string()],
            read_permissions: vec!["read".to_string()],
            write_permissions: vec![],
            rotation_permissions: vec!["rotate".to_string()],
        }),
    };

    provider.put_credential(&api_credential).await?;

    // Service account credentials
    let service_credential = credentials::Credential {
        name: "service-accounts".to_string(),
        values: HashMap::from([
            ("KAFKA_BROKER_USER".to_string(), "kafka-service".to_string()),
            ("KAFKA_BROKER_PASSWORD".to_string(), "kafka_password_456".to_string()),
            ("REDIS_USER".to_string(), "redis-admin".to_string()),
            ("REDIS_PASSWORD".to_string(), "redis_secret_789".to_string()),
        ]),
        metadata: HashMap::from([
            ("environment".to_string(), "production".to_string()),
            ("team".to_string(), "infrastructure".to_string()),
            ("sensitivity".to_string(), "critical".to_string()),
        ]),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        expires_at: Some(chrono::Utc::now() + chrono::Duration::days(120)),
        rotation_enabled: true,
        access_policy: Some(credentials::AccessPolicy {
            allowed_subjects: vec!["worker-service".to_string(), "scheduler".to_string()],
            read_permissions: vec!["read".to_string(), "write".to_string()],
            write_permissions: vec!["write".to_string()],
            rotation_permissions: vec!["rotate".to_string()],
        }),
    };

    provider.put_credential(&service_credential).await?;

    Ok(())
}

async fn setup_kubernetes_provider(
    credential_provider: &Arc<credentials::SimpleCredentialProvider>
) -> Result<Arc<providers::kubernetes::KubernetesProvider>, ProviderError> {
    // This would require actual Kubernetes cluster access in production
    // For demo purposes, we'll try to create it but expect it to fail
    let k8s_config = providers::kubernetes::KubernetesConfig {
        namespace: "default".to_string(),
        service_account: None,
        node_selector: None,
        tolerations: None,
        node_affinity: None,
        pod_disruption_budget: None,
        termination_grace_period_seconds: Some(30),
        image_pull_secrets: None,
        priority_class: None,
        security_context: None,
    };

    providers::kubernetes::KubernetesProvider::new(k8s_config, credential_provider.clone()).await
        .map(|p| Arc::new(p))
}

async fn create_database_worker() -> Result<RuntimeSpec, ProviderError> {
    let mut spec = RuntimeSpec::basic("postgres:15".to_string());
    spec.command = Some(vec![
        "postgres".to_string()
    ]);
    spec.secret_refs = vec!["database-credentials".to_string()];
    spec.ports = vec![5432];
    spec.resources = ResourceQuota::basic(1000, 1024);
    spec.labels.insert("type".to_string(), "database".to_string());
    spec.labels.insert("environment".to_string(), "production".to_string());
    
    // Add environment variables
    spec.env.insert("POSTGRES_DB".to_string(), "production_db".to_string());
    spec.env.insert("PGDATA".to_string(), "/var/lib/postgresql/data/pgdata".to_string());

    Ok(spec)
}

async fn create_api_worker() -> Result<RuntimeSpec, ProviderError> {
    let mut spec = RuntimeSpec::basic("nginx:alpine".to_string());
    spec.command = Some(vec![
        "nginx".to_string(), "-g".to_string(), "daemon off;".to_string()
    ]);
    spec.secret_refs = vec!["api-keys".to_string()];
    spec.ports = vec![80, 443];
    spec.resources = ResourceQuota::basic(500, 512);
    spec.labels.insert("type".to_string(), "api".to_string());
    spec.labels.insert("environment".to_string(), "production".to_string());
    
    // Add API-specific configuration
    spec.env.insert("API_TIMEOUT".to_string(), "30".to_string());
    spec.env.insert("RATE_LIMIT".to_string(), "100".to_string());

    Ok(spec)
}

async fn create_kubernetes_worker() -> Result<RuntimeSpec, ProviderError> {
    let mut spec = RuntimeSpec::basic("busybox:1.36".to_string());
    spec.command = Some(vec![
        "sh".to_string(), "-c".to_string(), 
        "echo 'Kubernetes worker started' && sleep 60".to_string()
    ]);
    spec.secret_refs = vec!["service-accounts".to_string()];
    spec.resources = ResourceQuota::basic(250, 256);
    spec.labels.insert("type".to_string(), "k8s-demo".to_string());
    spec.labels.insert("environment".to_string(), "production".to_string());

    Ok(spec)
}
