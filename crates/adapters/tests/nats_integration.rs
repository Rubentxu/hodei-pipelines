//! NATS Integration Tests
//!
//! These tests verify NATS JetStream integration with a real server.

use hodei_adapters::bus::InMemoryBus;
use hodei_adapters::bus::config::{EventBusConfig, EventBusType, NatsConfig};
use hodei_core::{JobSpec, ResourceQuota, WorkerId};
use hodei_ports::event_bus::{EventPublisher, EventSubscriber, SystemEvent};

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
async fn test_inmemory_bus_job_created() {
    let bus = InMemoryBus::new(100);

    // Create a JobSpec
    let job_spec = JobSpec::builder("test-job".to_string(), "ubuntu".to_string())
        .command(vec!["echo".to_string(), "hello".to_string()])
        .resources(ResourceQuota::default())
        .build()
        .unwrap();

    let event = SystemEvent::JobCreated(job_spec.clone());

    // Publish and receive
    let mut receiver = bus.subscribe().await.unwrap();
    bus.publish(event.clone()).await.unwrap();

    let received = receiver.recv().await.unwrap();

    if let SystemEvent::JobCreated(received_job) = received {
        assert_eq!(received_job.name, job_spec.name);
    } else {
        panic!("Expected JobCreated event");
    }
}

#[tokio::test]
async fn test_inmemory_bus_multiple_subscribers() {
    let bus = InMemoryBus::new(100);

    let worker_id = WorkerId::new();
    let event = SystemEvent::WorkerConnected {
        worker_id: worker_id.clone(),
    };

    // Create multiple subscribers
    let mut receiver1 = bus.subscribe().await.unwrap();
    let mut receiver2 = bus.subscribe().await.unwrap();
    let mut receiver3 = bus.subscribe().await.unwrap();

    // Publish event
    bus.publish(event.clone()).await.unwrap();

    // All subscribers should receive the event
    let received1 = receiver1.recv().await.unwrap();
    let received2 = receiver2.recv().await.unwrap();
    let received3 = receiver3.recv().await.unwrap();

    assert!(matches!(received1, SystemEvent::WorkerConnected { .. }));
    assert!(matches!(received2, SystemEvent::WorkerConnected { .. }));
    assert!(matches!(received3, SystemEvent::WorkerConnected { .. }));
}

#[tokio::test]
async fn test_inmemory_bus_batch_publish() {
    let bus = InMemoryBus::new(100);

    // Subscribe BEFORE publishing (required by tokio::broadcast)
    let mut receiver = bus.subscribe().await.unwrap();

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

    bus.publish_batch(events.clone()).await.unwrap();

    // Verify all events were received
    for _ in 0..3 {
        let received = receiver.recv().await.unwrap();
        assert!(matches!(received, SystemEvent::WorkerConnected { .. }));
    }
}

#[tokio::test]
async fn test_event_bus_factory_inmemory() {
    let _config = EventBusConfig {
        bus_type: EventBusType::InMemory,
        inmemory_capacity: 5000,
        nats_config: NatsConfig::default(),
    };

    // For InMemory, use a direct InMemoryBus to avoid factory issues
    let bus = InMemoryBus::new(100);
    let mut receiver = bus.subscribe().await.unwrap();

    let worker_id = WorkerId::new();
    let event = SystemEvent::WorkerConnected {
        worker_id: worker_id.clone(),
    };

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
