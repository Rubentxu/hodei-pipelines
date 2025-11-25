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
use std::fmt::Display;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(feature = "dashmap")]
use dashmap::DashMap;

/// Trait for all domain events
pub trait DomainEvent: Send + Sync {
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

/// Event Sourced Aggregate
pub trait EventSourcedAggregate {
    type Event: DomainEvent;

    /// Get current version of the aggregate
    fn version(&self) -> u64;

    /// Get uncommitted events (new events since last save)
    fn take_uncommitted_events(&mut self) -> Vec<Self::Event>;

    /// Mark all uncommitted events as committed
    fn mark_events_as_committed(&mut self);

    /// Load state from a sequence of events
    fn load_from_events(&mut self, events: &[Self::Event]);
}

/// Event Store trait for persisting events
#[async_trait::async_trait]
pub trait EventStore {
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
}

#[cfg(feature = "dashmap")]
impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(DashMap::new()),
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

        for event in events {
            let metadata = EventMetadata::new(
                event.event_id(),
                event.event_type().to_string(),
                event.aggregate_id(),
                "Job".to_string(),
                event.occurred_at(),
                event.version(),
            );

            events_guard.push(metadata.clone());
            metadata_list.push(metadata);
        }

        Ok(metadata_list)
    }

    async fn load_events(
        &self,
        aggregate_id: Uuid,
        from_version: Option<u64>,
    ) -> Result<Vec<Box<dyn DomainEvent>>, EventStoreError> {
        if let Some(entry) = self.events.get(&aggregate_id) {
            let events = entry.value();
            let start_index = from_version.unwrap_or(0) as usize;

            // For simplicity, return empty - would need actual deserialization
            Ok(Vec::new())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

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
    }
}
