//! Distributed communication module

pub mod error_handling;
pub mod nats_adapter;
pub mod tests;

pub use error_handling::DistributedError;
pub use nats_adapter::{EventBus, NatsClient, NatsError, NatsEventBus, TopicManager};
