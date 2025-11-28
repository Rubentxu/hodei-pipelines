use chrono::{DateTime, Utc};
use hodei_core::events::*;
use serde_json::Value;
use uuid::Uuid;

// Tests are gated behind event-store-tests feature
// This allows running unit tests without PostgreSQL

#[cfg(feature = "event-store-tests")]
#[sqlx::test]
async fn test_event_store_basic_operations(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let store = PostgreSqlEventStore::new(Arc::new(pool));

    // Initialize the events table
    store.init().await?;

    let aggregate_id = Uuid::new_v4();

    // Test saving events
    let event = TestEvent::new(aggregate_id, 0, "test data".to_string());
    let events = vec![Box::new(event) as Box<dyn DomainEvent>];

    let metadata_list = store.save_events(aggregate_id, &events, 0).await?;
    assert_eq!(metadata_list.len(), 1);
    assert_eq!(metadata_list[0].aggregate_id, aggregate_id);

    // Verify that events were persisted by checking the version
    let version = store.get_latest_version(aggregate_id).await?;
    assert_eq!(version, 1);

    Ok(())
}

#[cfg(feature = "event-store-tests")]
#[sqlx::test]
async fn test_event_store_concurrency_check(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = PostgreSqlEventStore::new(Arc::new(pool));

    // Initialize the events table
    store.init().await?;

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

#[cfg(feature = "event-store-tests")]
#[sqlx::test]
async fn test_event_store_version_tracking(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let store = PostgreSqlEventStore::new(Arc::new(pool));

    // Initialize the events table
    store.init().await?;

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

#[cfg(feature = "event-store-tests")]
#[sqlx::test]
async fn test_event_store_load_by_type(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let store = PostgreSqlEventStore::new(Arc::new(pool));

    // Initialize the events table
    store.init().await?;

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

    // Verify that both events were persisted
    let version1 = store.get_latest_version(aggregate_id1).await?;
    let version2 = store.get_latest_version(aggregate_id2).await?;
    assert_eq!(version1, 1);
    assert_eq!(version2, 1);

    Ok(())
}

// Test events - available for both unit and integration tests
#[derive(Debug, Clone, serde::Serialize)]
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

    fn serialize(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    fn as_trait_object(&self) -> Box<dyn DomainEvent> {
        Box::new(TestEvent {
            event_id: self.event_id,
            aggregate_id: self.aggregate_id,
            occurred_at: self.occurred_at,
            version: self.version,
            data: self.data.clone(),
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
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

    fn serialize(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    fn as_trait_object(&self) -> Box<dyn DomainEvent> {
        Box::new(OtherEvent {
            event_id: self.event_id,
            aggregate_id: self.aggregate_id,
            occurred_at: self.occurred_at,
            version: self.version,
            value: self.value,
        })
    }
}
