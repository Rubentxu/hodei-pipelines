#[cfg(test)]
mod tests {
    use crate::{EventBus, NatsClient, NatsEventBus, TopicManager};
    use hodei_shared_types::{CorrelationId, TenantId};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_nats_client_creation() {
        let client = NatsClient::new();
        assert!(client.publish("test.subject", b"test data").await.is_ok());
    }

    #[tokio::test]
    async fn test_event_publishing() {
        let client = Arc::new(NatsClient::new());
        let topic_manager = Arc::new(TopicManager::new(client));
        let event_bus = Arc::new(NatsEventBus::new(topic_manager));

        let test_event = "test event";
        let correlation_id = CorrelationId::new();
        let tenant_id = TenantId::new("test-tenant".to_string());

        let result = event_bus
            .publish("test.topic", &test_event, correlation_id, tenant_id)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stream_creation() {
        let client = Arc::new(NatsClient::new());
        let topic_manager = Arc::new(TopicManager::new(client));

        let result = topic_manager
            .create_stream("test-stream", &["test.subject.*"])
            .await;

        assert!(result.is_ok());
    }
}
