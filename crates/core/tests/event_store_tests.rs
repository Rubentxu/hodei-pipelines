use hodei_core::events::*;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[sqlx::test]
async fn test_event_store_basic_operations(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let store = PostgreSqlEventStore::new(Arc::new(pool));
    let aggregate_id = Uuid::new_v4();

    // Test saving events
    let event = TestEvent::new(aggregate_id, 0, "test data".to_string());
    let events = vec![Box::new(event) as Box<dyn DomainEvent>];

    let metadata_list = store.save_events(aggregate_id, &events, 0).await?;
    assert_eq!(metadata_list.len(), 1);
    assert_eq!(metadata_list[0].aggregate_id, aggregate_id);

    // Test loading events
    let loaded_events = store.load_events(aggregate_id, None).await?;
    assert_eq!(loaded_events.len(), 1);

    Ok(())
}

#[sqlx::test]
async fn test_event_store_concurrency_check(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = PostgreSqlEventStore::new(Arc::new(pool));
    let aggregate_id = Uuid::new_v4();

    // Save first event
    let event1 = TestEvent::new(aggregate_id, 0, "data1".to_string());
    store
        .save_events(aggregate_id, &[Box::new(event1)], 0)
        .await?;

    // Try to save with wrong expected version (should fail)
    let event2 = TestEvent::new(aggregate_id, 1, "data2".to_string());
    let result = store
        .save_events(aggregate_id, &[Box::new(event2)], 0)
        .await;

    assert!(matches!(
        result,
        Err(EventStoreError::ConcurrencyError { .. })
    ));

    Ok(())
}

#[sqlx::test]
async fn test_event_store_version_tracking(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let store = PostgreSqlEventStore::new(Arc::new(pool));
    let aggregate_id = Uuid::new_v4();

    // Get initial version
    let version = store.get_latest_version(aggregate_id).await?;
    assert_eq!(version, 0);

    // Save event v0
    let event1 = TestEvent::new(aggregate_id, 0, "data1".to_string());
    store
        .save_events(aggregate_id, &[Box::new(event1)], 0)
        .await?;

    // Get version after save
    let version = store.get_latest_version(aggregate_id).await?;
    assert_eq!(version, 1);

    Ok(())
}

#[sqlx::test]
async fn test_event_store_load_by_type(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let store = PostgreSqlEventStore::new(Arc::new(pool));
    let aggregate_id1 = Uuid::new_v4();
    let aggregate_id2 = Uuid::new_v4();

    // Save different event types
    let event1 = TestEvent::new(aggregate_id1, 0, "data1".to_string());
    let event2 = OtherEvent::new(aggregate_id2, 0, 42);

    store
        .save_events(aggregate_id1, &[Box::new(event1)], 0)
        .await?;
    store
        .save_events(aggregate_id2, &[Box::new(event2)], 0)
        .await?;

    // Load by type
    let test_events = store.load_events_by_type("TestEvent", None).await?;
    assert_eq!(test_events.len(), 1);

    let other_events = store.load_events_by_type("OtherEvent", None).await?;
    assert_eq!(other_events.len(), 1);

    Ok(())
}

// Test events
#[derive(Debug, Clone)]
struct TestEvent {
    event_id: Uuid,
    aggregate_id: Uuid,
    occurred_at: DateTime<Utc>,
    version: u64,
    data: String,
}

impl TestEvent {
    fn new(aggregate_id: Uuid, version: u64, data: String) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            aggregate_id,
            occurred_at: Utc::now(),
            version,
            data,
        }
    }
}

impl DomainEvent for TestEvent {
    fn event_id(&self) -> Uuid {
        self.event_id
    }

    fn event_type(&self) -> &'static str {
        "TestEvent"
    }

    fn aggregate_id(&self) -> Uuid {
        self.aggregate_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    fn version(&self) -> u64 {
        self.version
    }
}

#[derive(Debug, Clone)]
struct OtherEvent {
    event_id: Uuid,
    aggregate_id: Uuid,
    occurred_at: DateTime<Utc>,
    version: u64,
    value: i32,
}

impl OtherEvent {
    fn new(aggregate_id: Uuid, version: u64, value: i32) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            aggregate_id,
            occurred_at: Utc::now(),
            version,
            value,
        }
    }
}

impl DomainEvent for OtherEvent {
    fn event_id(&self) -> Uuid {
        self.event_id
    }

    fn event_type(&self) -> &'static str {
        "OtherEvent"
    }

    fn aggregate_id(&self) -> Uuid {
        self.aggregate_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    fn version(&self) -> u64 {
        self.version
    }
}
