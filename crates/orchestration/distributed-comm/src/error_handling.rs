//! Error handling for distributed communication

#[derive(Debug, thiserror::Error)]
pub enum DistributedError {
    #[error("connection error: {0}")]
    Connection(String),

    #[error("publish error: {0}")]
    Publish(String),

    #[error("subscribe error: {0}")]
    Subscribe(String),

    #[error("timeout: {0}")]
    Timeout(String),
}
