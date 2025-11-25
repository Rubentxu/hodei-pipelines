//! Event Registry for deserializing events from storage
//!
//! This module provides a registry to map event types to their deserialization functions.

use crate::events::{DomainEvent, EventStoreError};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Type alias for event deserializer function
pub type EventDeserializer = fn(serde_json::Value) -> Result<Box<dyn DomainEvent>, EventStoreError>;

/// Registry for event types and their deserializers
#[derive(Default)]
pub struct EventRegistry {
    deserializers: HashMap<&'static str, EventDeserializer>,
}

impl EventRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_default_events();
        registry
    }

    /// Register an event type with its deserializer
    pub fn register<T>(&mut self, deserializer: EventDeserializer)
    where
        T: DomainEvent + DeserializeOwned + 'static,
    {
        let event_type = std::any::type_name::<T>();
        // Get the last segment of the type name
        let event_name = event_type.rsplit("::").next().unwrap_or(event_type);
        self.deserializers.insert(event_name, deserializer);
    }

    /// Register built-in event types
    fn register_default_events(&mut self) {
        // Job events will be registered here
        // For now, we don't have concrete event types yet
    }

    /// Deserialize an event from JSON
    pub fn deserialize(
        &self,
        event_type: &str,
        data: serde_json::Value,
    ) -> Result<Box<dyn DomainEvent>, EventStoreError> {
        if let Some(deserializer) = self.deserializers.get(event_type) {
            deserializer(data)
        } else {
            Err(EventStoreError::DatabaseError(format!(
                "Unknown event type: {}",
                event_type
            )))
        }
    }
}

/// Helper macro to register events easily
#[macro_export]
macro_rules! register_event_type {
    ($registry:expr, $event_type:ty) => {
        $registry.register::<$event_type>(|data| {
            let event: $event_type = serde_json::from_value(data)
                .map_err(|e| EventStoreError::SerializationError(e.to_string()))?;
            Ok(Box::new(event) as Box<dyn DomainEvent>)
        });
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = EventRegistry::new();
        assert!(!registry.deserializers.is_empty());
    }
}
