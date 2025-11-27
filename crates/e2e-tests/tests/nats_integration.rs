//! NATS Integration Tests with Testcontainers
//!
//! These tests verify the NATS JetStream integration using a real NATS container.
//! They test publish/subscribe functionality with an actual NATS server.

use hodei_adapters::bus::config::{EventBusConfig, EventBusType, NatsConfig};
use hodei_adapters::bus::{EventBusFactory, InMemoryBus};
use hodei_core::{JobSpec, ResourceQuota, WorkerId};
use hodei_ports::event_bus::{EventPublisher, EventSubscriber, SystemEvent};
use std::time::Duration;
use testcontainers::Docker;
use tokio::time::timeout;

#[tokio::test]
async fn test_inmemory_bus_basic() {
    let bus = InMemoryBus::new(100);

    let worker_id = WorkerId::new();
    let event = SystemEvent::WorkerConnected {
        worker_id: worker_id.clone(),
    };

    // Subscribe before publishing
    let mut receiver = bus.subscribe().await.unwrap();

    // Publish event
    bus.publish(event.clone()).await.unwrap();

    // Receive event
    let received = receiver.recv().await.unwrap();

    if let SystemEvent::WorkerConnected {
        worker_id: received_id,
    } = received
    {
        assert_eq!(worker_id, received_id);
    } else {
        panic!("Expected WorkerConnected event");
    }
}

#[tokio::test]
async fn test_nats_factory_publish_subscribe() -> Result<(), Box<dyn std::error::Error>> {
    // Start NATS container
    let docker = Docker::default();
    let nats_container = docker.run("nats:2.10-alpine");

    // Wait for container to be ready
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Get NATS port
    let nats_port = nats_container.get_host_port_ipv4(4222);
    let nats_url = format!("nats://127.0.0.1:{}", nats_port);

    println!("Using NATS at: {}", nats_url);

    // Configure event bus with NATS
    let config = EventBusConfig {
        bus_type: EventBusType::Nats,
        inmemory_capacity: 1000,
        nats_config: NatsConfig {
            url: nats_url,
            connection_timeout_ms: 5000,
        },
    };

    // Create publisher and subscriber
    let publisher = EventBusFactory::create(config.clone()).await?;
    let subscriber = EventBusFactory::create_subscriber(config.clone()).await?;

    // Create receiver
    let mut receiver = subscriber.subscribe().await?;

    // Create test event
    let worker_id = WorkerId::new();
    let event = SystemEvent::WorkerConnected {
        worker_id: worker_id.clone(),
    };

    // Publish event
    println!("Publishing event...");
    publisher.publish(event.clone()).await?;

    // Receive event with timeout
    let received = timeout(Duration::from_secs(10), receiver.recv()).await??;

    if let SystemEvent::WorkerConnected {
        worker_id: received_id,
    } = received
    {
        assert_eq!(worker_id, received_id);
        println!("✅ Successfully published and received NATS event");
    } else {
        panic!("Expected WorkerConnected event");
    }

    Ok(())
}

#[tokio::test]
async fn test_nats_multiple_subscribers() -> Result<(), Box<dyn std::error::Error>> {
    // Start NATS container
    let docker = Docker::default();
    let nats_container = docker.run("nats:2.10-alpine");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let nats_port = nats_container.get_host_port_ipv4(4222);
    let nats_url = format!("nats://127.0.0.1:{}", nats_port);

    let config = EventBusConfig {
        bus_type: EventBusType::Nats,
        inmemory_capacity: 1000,
        nats_config: NatsConfig {
            url: nats_url,
            connection_timeout_ms: 5000,
        },
    };

    // Create publisher and multiple subscribers
    let publisher = EventBusFactory::create(config.clone()).await?;
    let subscriber1 = EventBusFactory::create_subscriber(config.clone()).await?;
    let subscriber2 = EventBusFactory::create_subscriber(config.clone()).await?;
    let subscriber3 = EventBusFactory::create_subscriber(config.clone()).await?;

    // Create receivers
    let mut receiver1 = subscriber1.subscribe().await?;
    let mut receiver2 = subscriber2.subscribe().await?;
    let mut receiver3 = subscriber3.subscribe().await?;

    // Create test event
    let worker_id = WorkerId::new();
    let event = SystemEvent::WorkerConnected {
        worker_id: worker_id.clone(),
    };

    // Publish event once
    publisher.publish(event.clone()).await?;

    // All subscribers should receive the event
    let received1 = timeout(Duration::from_secs(10), receiver1.recv()).await??;
    let received2 = timeout(Duration::from_secs(10), receiver2.recv()).await??;
    let received3 = timeout(Duration::from_secs(10), receiver3.recv()).await??;

    assert!(matches!(received1, SystemEvent::WorkerConnected { .. }));
    assert!(matches!(received2, SystemEvent::WorkerConnected { .. }));
    assert!(matches!(received3, SystemEvent::WorkerConnected { .. }));

    println!("✅ All 3 subscribers received the event");
    Ok(())
}

#[tokio::test]
async fn test_nats_job_created_event() -> Result<(), Box<dyn std::error::Error>> {
    // Start NATS container
    let docker = Docker::default();
    let nats_container = docker.run("nats:2.10-alpine");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let nats_port = nats_container.get_host_port_ipv4(4222);
    let nats_url = format!("nats://127.0.0.1:{}", nats_port);

    let config = EventBusConfig {
        bus_type: EventBusType::Nats,
        inmemory_capacity: 1000,
        nats_config: NatsConfig {
            url: nats_url,
            connection_timeout_ms: 5000,
        },
    };

    let publisher = EventBusFactory::create(config.clone()).await?;
    let subscriber = EventBusFactory::create_subscriber(config.clone()).await?;
    let mut receiver = subscriber.subscribe().await?;

    // Create a JobCreated event
    let job_spec = JobSpec::builder("test-job".to_string(), "ubuntu".to_string())
        .command(vec!["echo".to_string(), "hello".to_string()])
        .resources(ResourceQuota::default())
        .build()
        .unwrap();

    let event = SystemEvent::JobCreated(job_spec.clone());

    // Publish and receive
    publisher.publish(event.clone()).await?;

    let received = timeout(Duration::from_secs(10), receiver.recv()).await??;

    if let SystemEvent::JobCreated(received_job) = received {
        assert_eq!(received_job.name, job_spec.name);
        println!("✅ JobCreated event published and received successfully");
    } else {
        panic!("Expected JobCreated event");
    }

    Ok(())
}

#[tokio::test]
async fn test_nats_batch_publish() -> Result<(), Box<dyn std::error::Error>> {
    // Start NATS container
    let docker = Docker::default();
    let nats_container = docker.run("nats:2.10-alpine");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let nats_port = nats_container.get_host_port_ipv4(4222);
    let nats_url = format!("nats://127.0.0.1:{}", nats_port);

    let config = EventBusConfig {
        bus_type: EventBusType::Nats,
        inmemory_capacity: 1000,
        nats_config: NatsConfig {
            url: nats_url,
            connection_timeout_ms: 5000,
        },
    };

    let publisher = EventBusFactory::create(config.clone()).await?;
    let subscriber = EventBusFactory::create_subscriber(config.clone()).await?;
    let mut receiver = subscriber.subscribe().await?;

    // Create batch of events
    let events = vec![
        SystemEvent::WorkerConnected {
            worker_id: WorkerId::new(),
        },
        SystemEvent::WorkerConnected {
            worker_id: WorkerId::new(),
        },
        SystemEvent::WorkerConnected {
            worker_id: WorkerId::new(),
        },
    ];

    // Publish batch
    publisher.publish_batch(events.clone()).await?;

    // Receive all events
    for i in 0..3 {
        let received = timeout(Duration::from_secs(10), receiver.recv()).await??;
        assert!(matches!(received, SystemEvent::WorkerConnected { .. }));
        println!("✅ Received event {}", i + 1);
    }

    Ok(())
}

#[tokio::test]
async fn test_event_bus_factory_inmemory() {
    let config = EventBusConfig {
        bus_type: EventBusType::InMemory,
        inmemory_capacity: 5000,
        nats_config: NatsConfig::default(),
    };

    let bus = EventBusFactory::create(config.clone()).await.unwrap();

    let worker_id = WorkerId::new();
    let event = SystemEvent::WorkerConnected {
        worker_id: worker_id.clone(),
    };

    // Subscribe before publishing
    let subscriber = EventBusFactory::create_subscriber(config.clone())
        .await
        .unwrap();
    let mut receiver = subscriber.subscribe().await.unwrap();

    // Publish event
    bus.publish(event.clone()).await.unwrap();

    // Receive event
    let received = receiver.recv().await.unwrap();

    if let SystemEvent::WorkerConnected {
        worker_id: received_id,
    } = received
    {
        assert_eq!(worker_id, received_id);
        println!("✅ InMemory factory test passed");
    } else {
        panic!("Expected WorkerConnected event");
    }
}
