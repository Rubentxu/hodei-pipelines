//! NATS JetStream adapter for distributed communication (Mock Implementation)

use async_trait::async_trait;
use hodei_shared_types::{CorrelationId, TenantId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Mock NATS connection wrapper
pub struct NatsClient {
    _phantom: std::marker::PhantomData<()>,
}

impl NatsClient {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Publish a message (mock)
    pub async fn publish(&self, _subject: &str, _data: &[u8]) -> Result<(), NatsError> {
        tracing::debug!("Mock publish to subject");
        Ok(())
    }

    /// Subscribe to a subject (mock)
    pub fn subscribe(
        &self,
        _subject: &str,
    ) -> Result<std::sync::mpsc::Receiver<Vec<u8>>, NatsError> {
        let (_tx, rx) = std::sync::mpsc::channel();
        tracing::debug!("Mock subscribe to subject");
        // Return a mock receiver
        Ok(rx)
    }
}

/// NATS error types
#[derive(Debug, thiserror::Error)]
pub enum NatsError {
    #[error("publish error: {0}")]
    PublishError(String),

    #[error("subscribe error: {0}")]
    SubscribeError(String),

    #[error("connection error: {0}")]
    ConnectionError(String),
}

/// Message types for distributed communication
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub subject: String,
    pub payload: Vec<u8>,
    pub correlation_id: CorrelationId,
    pub tenant_id: TenantId,
}

/// NATS topics manager
pub struct TopicManager {
    nats_client: Arc<NatsClient>,
}

impl TopicManager {
    pub fn new(nats_client: Arc<NatsClient>) -> Self {
        Self { nats_client }
    }

    /// Create a JetStream stream (mock)
    pub async fn create_stream(&self, name: &str, subjects: &[&str]) -> Result<(), NatsError> {
        tracing::info!(
            "Mock: Creating stream {} with subjects {:?}",
            name,
            subjects
        );
        Ok(())
    }

    /// Publish event to topic
    pub async fn publish_event<T: Serialize>(
        &self,
        topic: &str,
        event: &T,
        correlation_id: CorrelationId,
        tenant_id: TenantId,
    ) -> Result<(), NatsError> {
        let payload =
            bincode::serialize(event).map_err(|e| NatsError::PublishError(e.to_string()))?;

        let message = Message {
            subject: topic.to_string(),
            payload,
            correlation_id,
            tenant_id,
        };

        let message_bytes =
            bincode::serialize(&message).map_err(|e| NatsError::PublishError(e.to_string()))?;

        self.nats_client.publish(topic, &message_bytes).await?;
        Ok(())
    }
}

/// EventBus trait for publishing events
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish<T: Serialize + Sync>(
        &self,
        topic: &str,
        event: &T,
        correlation_id: CorrelationId,
        tenant_id: TenantId,
    ) -> Result<(), NatsError>;
}

/// NATS EventBus implementation
pub struct NatsEventBus {
    topic_manager: Arc<TopicManager>,
}

impl NatsEventBus {
    pub fn new(topic_manager: Arc<TopicManager>) -> Self {
        Self { topic_manager }
    }
}

#[async_trait]
impl EventBus for NatsEventBus {
    async fn publish<T: Serialize + Sync>(
        &self,
        topic: &str,
        event: &T,
        correlation_id: CorrelationId,
        tenant_id: TenantId,
    ) -> Result<(), NatsError> {
        self.topic_manager
            .publish_event(topic, event, correlation_id, tenant_id)
            .await
    }
}
