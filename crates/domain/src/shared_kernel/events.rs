//! Domain Events and Event Sourcing Infrastructure
//!
//! This module provides infrastructure for implementing Event Sourcing pattern,
//! allowing complete audit trails and temporal queries.
//!
//! Generic building blocks for event sourcing:
//! - DomainEvent trait for all domain events
//! - EventMetadata for storing event metadata
//! - EventStore trait for persisting events
//! - EventSourcedAggregate trait for aggregates that support event sourcing

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "sqlx")]
use sqlx::Row;

#[cfg(feature = "dashmap")]
use dashmap::DashMap;

/// Trait for all domain events
pub trait DomainEvent: Send + Sync + std::fmt::Debug {
    /// Unique identifier of the event
    fn event_id(&self) -> Uuid;

    /// Type identifier of the event
    fn event_type(&self) -> &'static str;

    /// Aggregate ID this event belongs to
    fn aggregate_id(&self) -> Uuid;

    /// Timestamp when the event occurred
    fn occurred_at(&self) -> DateTime<Utc>;

    /// Event version (for optimistic locking)
    fn version(&self) -> u64;

    /// Serialize the event data to JSON
    fn serialize(&self) -> Result<serde_json::Value, serde_json::Error>;

    /// Convert to trait object
    fn as_trait_object(&self) -> Box<dyn DomainEvent>;
}

/// Event metadata for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub event_id: Uuid,
    pub event_type: String,
    pub aggregate_id: Uuid,
    pub aggregate_type: String,
    pub occurred_at: DateTime<Utc>,
    pub version: u64,
    pub user_id: Option<String>,
    pub correlation_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl EventMetadata {
    pub fn new(
        event_id: Uuid,
        event_type: String,
        aggregate_id: Uuid,
        aggregate_type: String,
        occurred_at: DateTime<Utc>,
        version: u64,
    ) -> Self {
        Self {
            event_id,
            event_type,
            aggregate_id,
            aggregate_type,
            occurred_at,
            version,
            user_id: None,
            correlation_id: None,
            metadata: None,
        }
    }

    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_correlation(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Base event implementation
#[derive(Debug, Clone)]
pub struct BaseEvent<T> {
    pub event_id: Uuid,
    pub aggregate_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub version: u64,
    pub data: T,
}

impl<T> BaseEvent<T> {
    pub fn new(aggregate_id: Uuid, version: u64, data: T) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            aggregate_id,
            occurred_at: Utc::now(),
            version,
            data,
        }
    }
}

/// Event Registry for deserializing events
#[allow(clippy::type_complexity)]
pub struct EventRegistry {
    builders: std::collections::HashMap<
        &'static str,
        Box<
            dyn Fn(serde_json::Value) -> Result<Box<dyn DomainEvent>, serde_json::Error>
                + Send
                + Sync,
        >,
    >,
}

impl EventRegistry {
    pub fn new() -> Self {
        Self {
            builders: std::collections::HashMap::new(),
        }
    }

    /// Register an event type with its deserializer
    pub fn register<T>(&mut self, event_type: &'static str)
    where
        T: DomainEvent + for<'de> serde::de::Deserialize<'de> + 'static,
    {
        let builder =
            move |value: serde_json::Value| -> Result<Box<dyn DomainEvent>, serde_json::Error> {
                let event: T = serde_json::from_value(value)?;
                Ok(Box::new(event) as Box<dyn DomainEvent>)
            };
        self.builders.insert(event_type, Box::new(builder));
    }

    /// Deserialize a JSON event into a DomainEvent
    pub fn deserialize(
        &self,
        event_type: &str,
        value: serde_json::Value,
    ) -> Result<Box<dyn DomainEvent>, EventStoreError> {
        self.builders
            .get(event_type)
            .ok_or_else(|| {
                EventStoreError::SerializationError(format!(
                    "Event type '{}' not registered in registry",
                    event_type
                ))
            })
            .and_then(|builder| {
                builder(value).map_err(|e| EventStoreError::SerializationError(e.to_string()))
            })
    }
}

impl Default for EventRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Event Sourced Aggregate
pub trait EventSourcedAggregate: Send + Sync {
    /// Get current version of the aggregate
    fn version(&self) -> u64;

    /// Get uncommitted events (new events since last save)
    fn take_uncommitted_events(&mut self) -> Vec<Box<dyn DomainEvent>>;

    /// Mark all uncommitted events as committed
    fn mark_events_as_committed(&mut self);

    /// Load state from a sequence of events
    fn load_from_events(&mut self, events: &[Box<dyn DomainEvent>]);
}

/// Event Store trait for persisting events
#[async_trait::async_trait]
pub trait EventStore: Send + Sync {
    /// Save events to the store
    async fn save_events(
        &self,
        aggregate_id: Uuid,
        events: &[Box<dyn DomainEvent>],
        expected_version: u64,
    ) -> Result<Vec<EventMetadata>, EventStoreError>;

    /// Load events for an aggregate
    async fn load_events(
        &self,
        aggregate_id: Uuid,
        from_version: Option<u64>,
    ) -> Result<Vec<Box<dyn DomainEvent>>, EventStoreError>;

    /// Load all events of a specific type
    async fn load_events_by_type(
        &self,
        event_type: &'static str,
        from_timestamp: Option<DateTime<Utc>>,
    ) -> Result<Vec<Box<dyn DomainEvent>>, EventStoreError>;

    /// Replay events for an aggregate using a registry for deserialization
    async fn replay_events(
        &self,
        aggregate_id: Uuid,
        from_version: Option<u64>,
        registry: &EventRegistry,
    ) -> Result<Vec<Box<dyn DomainEvent>>, EventStoreError>;

    /// Get the latest version of an aggregate
    async fn get_latest_version(&self, aggregate_id: Uuid) -> Result<u64, EventStoreError>;
}

/// Event Store Error
#[derive(thiserror::Error, Debug)]
pub enum EventStoreError {
    #[error("Concurrency error: expected version {expected} but found {actual}")]
    ConcurrencyError { expected: u64, actual: u64 },

    #[error("Aggregate not found: {0}")]
    AggregateNotFound(Uuid),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// In-memory Event Store for testing
#[cfg(feature = "dashmap")]
pub struct InMemoryEventStore {
    events: Arc<DashMap<Uuid, Vec<EventMetadata>>>,
    event_data: Arc<DashMap<Uuid, Vec<serde_json::Value>>>,
}

#[cfg(feature = "dashmap")]
impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(DashMap::new()),
            event_data: Arc::new(DashMap::new()),
        }
    }
}

#[cfg(feature = "dashmap")]
#[async_trait::async_trait]
impl EventStore for InMemoryEventStore {
    async fn save_events(
        &self,
        aggregate_id: Uuid,
        events: &[Box<dyn DomainEvent>],
        expected_version: u64,
    ) -> Result<Vec<EventMetadata>, EventStoreError> {
        let mut metadata_list = Vec::new();

        let mut events_guard = self.events.entry(aggregate_id).or_insert_with(Vec::new);
        let current_version = events_guard.len() as u64;

        if current_version != expected_version {
            return Err(EventStoreError::ConcurrencyError {
                expected: expected_version,
                actual: current_version,
            });
        }

        let mut event_data_list = self.event_data.entry(aggregate_id).or_insert_with(Vec::new);

        for event in events {
            let event_data = event
                .serialize()
                .map_err(|e| EventStoreError::SerializationError(e.to_string()))?;

            let metadata = EventMetadata::new(
                event.event_id(),
                event.event_type().to_string(),
                event.aggregate_id(),
                "Job".to_string(),
                event.occurred_at(),
                event.version(),
            );

            events_guard.push(metadata.clone());
            event_data_list.push(event_data);
            metadata_list.push(metadata);
        }

        Ok(metadata_list)
    }

    async fn load_events(
        &self,
        aggregate_id: Uuid,
        from_version: Option<u64>,
    ) -> Result<Vec<Box<dyn DomainEvent>>, EventStoreError> {
        if let Some(event_data) = self.event_data.get(&aggregate_id) {
            let events = event_data.value();
            let start_index = from_version.unwrap_or(0) as usize;

            let mut loaded_events = Vec::new();
            for event_data in &events[start_index..] {
                // For load_events without registry, we can't deserialize properly
                // In a real implementation, you'd need a default registry or return raw data
                let _ = event_data;
            }
            Ok(loaded_events)
        } else {
            Ok(Vec::new())
        }
    }

    async fn load_events_by_type(
        &self,
        event_type: &'static str,
        _from_timestamp: Option<DateTime<Utc>>,
    ) -> Result<Vec<Box<dyn DomainEvent>>, EventStoreError> {
        let mut all_events = Vec::new();

        for entry in self.events.iter() {
            for event_meta in entry.value() {
                if event_meta.event_type == event_type {
                    // Would need to deserialize actual event
                }
            }
        }

        Ok(all_events)
    }

    async fn replay_events(
        &self,
        aggregate_id: Uuid,
        from_version: Option<u64>,
        registry: &EventRegistry,
    ) -> Result<Vec<Box<dyn DomainEvent>>, EventStoreError> {
        if let (Some(event_data), Some(events_metadata)) = (
            self.event_data.get(&aggregate_id),
            self.events.get(&aggregate_id),
        ) {
            let event_data_list = event_data.value();
            let events_meta_list = events_metadata.value();
            let start_index = from_version.unwrap_or(0) as usize;

            let mut replayed_events = Vec::new();

            // Replay events from the specified version
            for i in start_index..event_data_list.len() {
                let event_json = &event_data_list[i];
                let event_meta = &events_meta_list[i];

                let event = registry.deserialize(&event_meta.event_type, event_json.clone())?;
                replayed_events.push(event);
            }

            Ok(replayed_events)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_latest_version(&self, aggregate_id: Uuid) -> Result<u64, EventStoreError> {
        if let Some(entry) = self.events.get(&aggregate_id) {
            Ok(entry.value().len() as u64)
        } else {
            Ok(0)
        }
    }
}

#[cfg(feature = "dashmap")]
impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

/// PostgreSQL Event Store implementation
#[cfg(feature = "sqlx")]
pub struct PostgreSqlEventStore {
    pool: std::sync::Arc<sqlx::PgPool>,
}

#[cfg(feature = "sqlx")]
impl PostgreSqlEventStore {
    pub fn new(pool: std::sync::Arc<sqlx::PgPool>) -> Self {
        Self { pool }
    }

    /// Initialize the events table
    pub async fn init(&self) -> Result<(), EventStoreError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                event_id UUID PRIMARY KEY,
                aggregate_id UUID NOT NULL,
                aggregate_type VARCHAR(255) NOT NULL,
                event_type VARCHAR(255) NOT NULL,
                event_data JSONB NOT NULL,
                version BIGINT NOT NULL,
                occurred_at TIMESTAMPTZ NOT NULL,
                user_id VARCHAR(255),
                correlation_id VARCHAR(255),
                metadata JSONB
            );
        "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| EventStoreError::DatabaseError(e.to_string()))?;

        // Create index on aggregate_id for faster lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_events_aggregate_id
            ON events (aggregate_id, version);
        "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| EventStoreError::DatabaseError(e.to_string()))?;

        // Create index on event_type for filtering
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_events_event_type
            ON events (event_type);
        "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| EventStoreError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(feature = "sqlx")]
#[async_trait::async_trait]
impl EventStore for PostgreSqlEventStore {
    async fn save_events(
        &self,
        aggregate_id: Uuid,
        events: &[Box<dyn DomainEvent>],
        expected_version: u64,
    ) -> Result<Vec<EventMetadata>, EventStoreError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| EventStoreError::DatabaseError(e.to_string()))?;

        // Check current version for optimistic locking
        let row: (i64,) =
            sqlx::query_as("SELECT COALESCE(MAX(version), -1) FROM events WHERE aggregate_id = $1")
                .bind(aggregate_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| EventStoreError::DatabaseError(e.to_string()))?;

        let current_version = (row.0 + 1) as u64;

        if current_version != expected_version {
            tx.rollback()
                .await
                .map_err(|e| EventStoreError::DatabaseError(e.to_string()))?;
            return Err(EventStoreError::ConcurrencyError {
                expected: expected_version,
                actual: current_version,
            });
        }

        let mut metadata_list = Vec::new();

        for (idx, event) in events.iter().enumerate() {
            let expected_ev_version = expected_version + idx as u64;
            let event_version = event.version();

            // Verify event version matches expected
            if event_version != expected_ev_version {
                tx.rollback()
                    .await
                    .map_err(|e| EventStoreError::DatabaseError(e.to_string()))?;
                return Err(EventStoreError::ConcurrencyError {
                    expected: expected_ev_version,
                    actual: event_version,
                });
            }

            let event_data = event
                .serialize()
                .map_err(|e| EventStoreError::SerializationError(e.to_string()))?;

            let metadata = EventMetadata::new(
                event.event_id(),
                event.event_type().to_string(),
                event.aggregate_id(),
                "Job".to_string(),
                event.occurred_at(),
                event.version(),
            );

            sqlx::query(
                r#"
                INSERT INTO events (
                    event_id, aggregate_id, aggregate_type, event_type,
                    event_data, version, occurred_at, user_id, correlation_id, metadata
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            )
            .bind(event.event_id())
            .bind(event.aggregate_id())
            .bind(&metadata.aggregate_type)
            .bind(event.event_type())
            .bind(event_data)
            .bind(event.version() as i64)
            .bind(event.occurred_at())
            .bind(&metadata.user_id)
            .bind(&metadata.correlation_id)
            .bind(&metadata.metadata)
            .execute(&mut *tx)
            .await
            .map_err(|e| EventStoreError::DatabaseError(e.to_string()))?;

            metadata_list.push(metadata);
        }

        tx.commit()
            .await
            .map_err(|e| EventStoreError::DatabaseError(e.to_string()))?;

        Ok(metadata_list)
    }

    async fn load_events(
        &self,
        aggregate_id: Uuid,
        from_version: Option<u64>,
    ) -> Result<Vec<Box<dyn DomainEvent>>, EventStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, aggregate_id, event_type, event_data, version, occurred_at
            FROM events
            WHERE aggregate_id = $1 AND version >= $2
            ORDER BY version ASC
        "#,
        )
        .bind(aggregate_id)
        .bind(from_version.unwrap_or(0) as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| EventStoreError::DatabaseError(e.to_string()))?;

        let events = Vec::new();

        for row in rows {
            let event_type: String = row.get("event_type");
            let _event_data: serde_json::Value = row.get("event_data");

            // Note: Event deserialization requires a proper event registry
            // For now, we'll return an empty events list
            // In production, use an EventRegistry to dynamically deserialize events
            let _ = event_type;
        }

        Ok(events)
    }

    async fn load_events_by_type(
        &self,
        event_type: &'static str,
        _from_timestamp: Option<DateTime<Utc>>,
    ) -> Result<Vec<Box<dyn DomainEvent>>, EventStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, aggregate_id, event_type, event_data, version, occurred_at
            FROM events
            WHERE event_type = $1
            ORDER BY occurred_at ASC
        "#,
        )
        .bind(event_type)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| EventStoreError::DatabaseError(e.to_string()))?;

        let events = Vec::new();

        for row in rows {
            let event_type_str: String = row.get("event_type");
            let _event_data: serde_json::Value = row.get("event_data");

            if event_type_str == event_type {
                // Note: Event deserialization requires a proper event registry
                // For now, we'll return an empty events list
            }
        }

        Ok(events)
    }

    async fn replay_events(
        &self,
        aggregate_id: Uuid,
        from_version: Option<u64>,
        registry: &EventRegistry,
    ) -> Result<Vec<Box<dyn DomainEvent>>, EventStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, aggregate_id, event_type, event_data, version, occurred_at
            FROM events
            WHERE aggregate_id = $1 AND version >= $2
            ORDER BY version ASC
        "#,
        )
        .bind(aggregate_id)
        .bind(from_version.unwrap_or(0) as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| EventStoreError::DatabaseError(e.to_string()))?;

        let mut events = Vec::new();

        for row in rows {
            let event_type: String = row.get("event_type");
            let event_data: serde_json::Value = row.get("event_data");

            let event = registry.deserialize(&event_type, event_data)?;
            events.push(event);
        }

        Ok(events)
    }

    async fn get_latest_version(&self, aggregate_id: Uuid) -> Result<u64, EventStoreError> {
        let row: (Option<i64>,) =
            sqlx::query_as("SELECT MAX(version) FROM events WHERE aggregate_id = $1")
                .bind(aggregate_id)
                .fetch_one(&*self.pool)
                .await
                .map_err(|e| EventStoreError::DatabaseError(e.to_string()))?;

        Ok(match row.0 {
            Some(max_version) => (max_version + 1) as u64,
            None => 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_metadata_creation() {
        let metadata = EventMetadata::new(
            Uuid::new_v4(),
            "TestEvent".to_string(),
            Uuid::new_v4(),
            "TestAggregate".to_string(),
            Utc::now(),
            1,
        );

        assert_eq!(metadata.version, 1);
        assert_eq!(metadata.aggregate_type, "TestAggregate");
    }

    #[test]
    fn test_base_event_creation() {
        let event = BaseEvent::new(Uuid::new_v4(), 1, "test data".to_string());

        assert_eq!(event.version, 1);
        assert_eq!(event.data, "test data");
    }

    #[tokio::test]
    #[cfg(feature = "dashmap")]
    async fn test_in_memory_event_store() {
        let store = InMemoryEventStore::new();
        let aggregate_id = Uuid::new_v4();

        // Create a test event
        let event = TestDomainEvent {
            event_id: Uuid::new_v4(),
            aggregate_id,
            occurred_at: Utc::now(),
            version: 0,
            data: "test data".to_string(),
        };

        let metadata_list = store
            .save_events(aggregate_id, &[Box::new(event)], 0)
            .await
            .unwrap();

        assert_eq!(metadata_list.len(), 1);
        assert_eq!(metadata_list[0].event_type, "TestDomainEvent");

        let version = store.get_latest_version(aggregate_id).await.unwrap();
        assert_eq!(version, 1);
    }

    /// Test event for demonstrations
    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct TestDomainEvent {
        event_id: Uuid,
        aggregate_id: Uuid,
        occurred_at: DateTime<Utc>,
        version: u64,
        data: String,
    }

    impl DomainEvent for TestDomainEvent {
        fn event_id(&self) -> Uuid {
            self.event_id
        }

        fn event_type(&self) -> &'static str {
            "TestDomainEvent"
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

        fn serialize(&self) -> Result<serde_json::Value, serde_json::Error> {
            serde_json::to_value(self)
        }

        fn as_trait_object(&self) -> Box<dyn DomainEvent> {
            Box::new(TestDomainEvent {
                event_id: self.event_id,
                aggregate_id: self.aggregate_id,
                occurred_at: self.occurred_at,
                version: self.version,
                data: self.data.clone(),
            })
        }
    }

    #[tokio::test]
    #[cfg(feature = "dashmap")]
    async fn test_event_registry_and_replay() {
        use std::sync::Arc;

        let store = Arc::new(InMemoryEventStore::new());
        let aggregate_id = Uuid::new_v4();

        // Create registry
        let mut registry = EventRegistry::new();
        registry.register::<TestDomainEvent>("TestDomainEvent");

        // Create and save multiple events
        let event1 = TestDomainEvent {
            event_id: Uuid::new_v4(),
            aggregate_id,
            occurred_at: Utc::now(),
            version: 0,
            data: "event 1".to_string(),
        };

        let event2 = TestDomainEvent {
            event_id: Uuid::new_v4(),
            aggregate_id,
            occurred_at: Utc::now(),
            version: 1,
            data: "event 2".to_string(),
        };

        let event3 = TestDomainEvent {
            event_id: Uuid::new_v4(),
            aggregate_id,
            occurred_at: Utc::now(),
            version: 2,
            data: "event 3".to_string(),
        };

        // Save all events
        let metadata_list = store
            .save_events(
                aggregate_id,
                &[Box::new(event1), Box::new(event2), Box::new(event3)],
                0,
            )
            .await
            .unwrap();

        assert_eq!(metadata_list.len(), 3);

        // Replay all events
        let replayed_events = store
            .replay_events(aggregate_id, None, &registry)
            .await
            .unwrap();

        assert_eq!(replayed_events.len(), 3);

        // Verify that events were properly deserialized by checking their types
        let event_types: Vec<&str> = replayed_events.iter().map(|e| e.event_type()).collect();
        assert_eq!(
            event_types,
            vec!["TestDomainEvent", "TestDomainEvent", "TestDomainEvent"]
        );

        // Verify versions are correct
        let versions: Vec<u64> = replayed_events.iter().map(|e| e.version()).collect();
        assert_eq!(versions, vec![0, 1, 2]);
    }

    #[tokio::test]
    #[cfg(feature = "dashmap")]
    async fn test_event_replay_from_version() {
        use std::sync::Arc;

        let store = Arc::new(InMemoryEventStore::new());
        let aggregate_id = Uuid::new_v4();

        // Create registry
        let mut registry = EventRegistry::new();
        registry.register::<TestDomainEvent>("TestDomainEvent");

        // Create and save multiple events
        for i in 0..5 {
            let event = TestDomainEvent {
                event_id: Uuid::new_v4(),
                aggregate_id,
                occurred_at: Utc::now(),
                version: i as u64,
                data: format!("event {}", i),
            };

            store
                .save_events(aggregate_id, &[Box::new(event)], i as u64)
                .await
                .unwrap();
        }

        // Replay from version 2
        let replayed_events = store
            .replay_events(aggregate_id, Some(2), &registry)
            .await
            .unwrap();

        assert_eq!(replayed_events.len(), 3); // events 2, 3, 4

        // Verify event versions
        for (i, event) in replayed_events.iter().enumerate() {
            assert_eq!(event.version(), (i + 2) as u64);
        }
    }

    #[tokio::test]
    #[cfg(feature = "dashmap")]
    async fn test_event_replay_empty_aggregate() {
        let store = InMemoryEventStore::new();
        let aggregate_id = Uuid::new_v4();

        let mut registry = EventRegistry::new();
        registry.register::<TestDomainEvent>("TestDomainEvent");

        // Try to replay events for non-existent aggregate
        let replayed_events = store
            .replay_events(aggregate_id, None, &registry)
            .await
            .unwrap();

        assert_eq!(replayed_events.len(), 0);
    }

    #[test]
    fn test_event_registry_error_on_unregistered_type() {
        let registry = EventRegistry::new();

        let result = registry.deserialize("UnknownEvent", serde_json::json!({"test": "data"}));

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("not registered"));
        }
    }
}
