//! Command-Query Separation (CQRS) infrastructure
//!
//! This module provides the building blocks for implementing CQRS pattern:
//! - Commands: Operations that change state
//! - Queries: Operations that read state
//! - Command Bus: Dispatches commands to handlers
//! - Query Bus: Dispatches queries to handlers

use async_trait::async_trait;
use std::fmt::Display;
use std::sync::Arc;
use uuid::Uuid;

/// Command error types
#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    #[error("Aggregate not found: {0}")]
    AggregateNotFound(Uuid),

    #[error("Concurrency error: {0}")]
    ConcurrencyError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Business rule violation: {0}")]
    BusinessRuleViolation(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl CommandError {
    pub fn aggregate_not_found(id: Uuid) -> Self {
        CommandError::AggregateNotFound(id)
    }

    pub fn concurrency(expected: u64, actual: u64) -> Self {
        CommandError::ConcurrencyError(format!(
            "Expected version {} but found {}",
            expected, actual
        ))
    }

    pub fn validation(msg: &str) -> Self {
        CommandError::ValidationError(msg.to_string())
    }

    pub fn business_rule(msg: &str) -> Self {
        CommandError::BusinessRuleViolation(msg.to_string())
    }
}

/// Base trait for all commands
pub trait Command: Send + Sync {
    type AggregateId: Display + Send + Sync;

    /// Get the ID of the aggregate this command targets
    fn aggregate_id(&self) -> Self::AggregateId;

    /// Get a unique correlation ID for tracking
    fn correlation_id(&self) -> Option<Uuid>;
}

/// Command handler trait
#[async_trait]
pub trait CommandHandler<C>
where
    C: Command,
{
    type Context: Send + Sync;
    type Result: Send;

    /// Handle a command and return a result
    async fn handle(
        &self,
        command: C,
        context: &Self::Context,
    ) -> Result<Self::Result, CommandError>;
}

/// Command Bus
pub struct CommandBus<C, H, T>
where
    C: Command,
    H: CommandHandler<C, Context = T>,
    T: Send + Sync,
{
    handler: Arc<H>,
    context: Arc<T>,
}

impl<C, H, T> CommandBus<C, H, T>
where
    C: Command,
    H: CommandHandler<C, Context = T>,
    T: Send + Sync,
{
    pub fn new(handler: Arc<H>, context: Arc<T>) -> Self {
        Self { handler, context }
    }

    /// Execute a command
    pub async fn execute(&self, command: C) -> Result<H::Result, CommandError> {
        self.handler.handle(command, &*self.context).await
    }
}

impl<C, H, T> Clone for CommandBus<C, H, T>
where
    C: Command,
    H: CommandHandler<C, Context = T>,
    T: Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            context: self.context.clone(),
        }
    }
}

impl<C, H, T> std::ops::Deref for CommandBus<C, H, T>
where
    C: Command,
    H: CommandHandler<C, Context = T>,
    T: Send + Sync,
{
    type Target = H;

    fn deref(&self) -> &Self::Target {
        &*self.handler
    }
}

/// Query error types
#[derive(thiserror::Error, Debug)]
pub enum QueryError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl QueryError {
    pub fn not_found(msg: &str) -> Self {
        QueryError::NotFound(msg.to_string())
    }

    pub fn validation(msg: &str) -> Self {
        QueryError::ValidationError(msg.to_string())
    }
}

/// Base trait for all queries
pub trait Query: Send + Sync {
    type Result: Send;
}

/// Query handler trait
#[async_trait]
pub trait QueryHandler<Q>
where
    Q: Query,
{
    type Context: Send + Sync;

    /// Handle a query and return a result
    async fn handle(&self, query: Q, context: &Self::Context) -> Result<Q::Result, QueryError>;
}

/// Query Bus
pub struct QueryBus<Q, H, T>
where
    Q: Query,
    H: QueryHandler<Q, Context = T>,
    T: Send + Sync,
{
    handler: Arc<H>,
    context: Arc<T>,
}

impl<Q, H, T> QueryBus<Q, H, T>
where
    Q: Query,
    H: QueryHandler<Q, Context = T>,
    T: Send + Sync,
{
    pub fn new(handler: Arc<H>, context: Arc<T>) -> Self {
        Self { handler, context }
    }

    /// Execute a query
    pub async fn execute(&self, query: Q) -> Result<Q::Result, QueryError> {
        self.handler.handle(query, &*self.context).await
    }
}

impl<Q, H, T> Clone for QueryBus<Q, H, T>
where
    Q: Query,
    H: QueryHandler<Q, Context = T>,
    T: Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            context: self.context.clone(),
        }
    }
}

impl<Q, H, T> std::ops::Deref for QueryBus<Q, H, T>
where
    Q: Query,
    H: QueryHandler<Q, Context = T>,
    T: Send + Sync,
{
    type Target = H;

    fn deref(&self) -> &Self::Target {
        &*self.handler
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_command_error_types() {
        let err = CommandError::validation("test error");
        assert!(matches!(err, CommandError::ValidationError(_)));
    }

    #[test]
    fn test_query_error_types() {
        let err = QueryError::not_found("test");
        assert!(matches!(err, QueryError::NotFound(_)));
    }
}
