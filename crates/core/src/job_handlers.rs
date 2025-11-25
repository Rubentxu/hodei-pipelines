//! Job Command and Query Handlers
//!
//! This module provides handlers for Job commands and queries.

use async_trait::async_trait;

use crate::cqrs::{CommandHandler, QueryHandler};
use crate::event_registry::EventRegistry;
use crate::event_sourced_job::EventSourcedJob;
use crate::events::{EventStore, EventStoreError};
use crate::job_cqrs::*;
use crate::{Result as DomainResult, Uuid};
use std::sync::Arc;

/// Application context for command handlers
#[derive(Clone)]
pub struct CommandContext {
    pub event_store: Arc<dyn EventStore>,
    pub event_registry: Arc<EventRegistry>,
}

impl CommandContext {
    pub fn new(event_store: Arc<dyn EventStore>, event_registry: Arc<EventRegistry>) -> Self {
        Self {
            event_store,
            event_registry,
        }
    }
}

/// Command Handler for Job commands
pub struct JobCommandHandler {
    context: Arc<CommandContext>,
}

impl JobCommandHandler {
    pub fn new(context: Arc<CommandContext>) -> Self {
        Self { context }
    }
}

#[async_trait]
impl CommandHandler<CreateJobCommand> for JobCommandHandler {
    type Context = CommandContext;
    type Result = Uuid;

    async fn handle(
        &self,
        command: CreateJobCommand,
        _context: &Self::Context,
    ) -> Result<Self::Result, crate::cqrs::CommandError> {
        let (job, event) = EventSourcedJob::create(
            command.job_id,
            command.tenant_id,
            command.name,
            command.command,
            command.image,
        );

        // Save the event to the event store
        let events = vec![Box::new(event) as Box<dyn crate::events::DomainEvent>];

        match self
            .context
            .event_store
            .save_events(
                command.job_id.0,
                &events,
                0, // Expected version for new aggregate
            )
            .await
        {
            Ok(_) => Ok(command.job_id.0),
            Err(EventStoreError::DatabaseError(e)) => {
                Err(crate::cqrs::CommandError::InternalError(e))
            }
            Err(EventStoreError::ConcurrencyError { expected, actual }) => {
                Err(crate::cqrs::CommandError::concurrency(expected, actual))
            }
            Err(_) => Err(crate::cqrs::CommandError::InternalError(
                "Unknown error".to_string(),
            )),
        }
    }
}

#[async_trait]
impl CommandHandler<StartJobCommand> for JobCommandHandler {
    type Context = CommandContext;
    type Result = Uuid;

    async fn handle(
        &self,
        command: StartJobCommand,
        _context: &Self::Context,
    ) -> Result<Self::Result, crate::cqrs::CommandError> {
        // Load current job state from events
        let events = match self
            .context
            .event_store
            .load_events(command.job_id.0, None)
            .await
        {
            Ok(events) => events,
            Err(_) => {
                return Err(crate::cqrs::CommandError::AggregateNotFound(
                    command.job_id.0,
                ));
            }
        };

        // Reconstruct job state
        let mut job = EventSourcedJob {
            id: command.job_id.clone(),
            name: "".to_string(),
            description: None,
            command: "".to_string(),
            image: "".to_string(),
            state: crate::job_definitions::JobState::new(
                crate::job_definitions::JobState::PENDING.to_string(),
            )
            .unwrap(),
            tenant_id: None,
            worker_id: None,
            retry_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            metadata: std::collections::HashMap::new(),
            version: 0,
        };

        job.load_from_events(&events);

        // Create and apply start event
        let start_event = match job.start() {
            Ok(event) => event,
            Err(_) => {
                return Err(crate::cqrs::CommandError::BusinessRule(
                    "Job cannot be started in current state".to_string(),
                ));
            }
        };

        // Save the event
        let events = vec![Box::new(start_event) as Box<dyn crate::events::DomainEvent>];
        let current_version = job.version();

        match self
            .context
            .event_store
            .save_events(command.job_id.0, &events, current_version)
            .await
        {
            Ok(_) => Ok(command.job_id.0),
            Err(EventStoreError::ConcurrencyError { expected, actual }) => {
                Err(crate::cqrs::CommandError::concurrency(expected, actual))
            }
            Err(_) => Err(crate::cqrs::CommandError::InternalError(
                "Unknown error".to_string(),
            )),
        }
    }
}

#[async_trait]
impl CommandHandler<CompleteJobCommand> for JobCommandHandler {
    type Context = CommandContext;
    type Result = Uuid;

    async fn handle(
        &self,
        command: CompleteJobCommand,
        _context: &Self::Context,
    ) -> Result<Self::Result, crate::cqrs::CommandError> {
        // Load and reconstruct job state
        let events = match self
            .context
            .event_store
            .load_events(command.job_id.0, None)
            .await
        {
            Ok(events) => events,
            Err(_) => {
                return Err(crate::cqrs::CommandError::AggregateNotFound(
                    command.job_id.0,
                ));
            }
        };

        let mut job = EventSourcedJob {
            id: command.job_id.clone(),
            name: "".to_string(),
            description: None,
            command: "".to_string(),
            image: "".to_string(),
            state: crate::job_definitions::JobState::new(
                crate::job_definitions::JobState::PENDING.to_string(),
            )
            .unwrap(),
            tenant_id: None,
            worker_id: None,
            retry_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            metadata: std::collections::HashMap::new(),
            version: 0,
        };

        job.load_from_events(&events);

        // Create and apply complete event
        let complete_event =
            match job.complete(command.exit_code, command.duration_ms, command.output) {
                Ok(event) => event,
                Err(_) => {
                    return Err(crate::cqrs::CommandError::BusinessRule(
                        "Job cannot be completed in current state".to_string(),
                    ));
                }
            };

        // Save the event
        let events = vec![Box::new(complete_event) as Box<dyn crate::events::DomainEvent>];
        let current_version = job.version();

        match self
            .context
            .event_store
            .save_events(command.job_id.0, &events, current_version)
            .await
        {
            Ok(_) => Ok(command.job_id.0),
            Err(EventStoreError::ConcurrencyError { expected, actual }) => {
                Err(crate::cqrs::CommandError::concurrency(expected, actual))
            }
            Err(_) => Err(crate::cqrs::CommandError::InternalError(
                "Unknown error".to_string(),
            )),
        }
    }
}

#[async_trait]
impl CommandHandler<FailJobCommand> for JobCommandHandler {
    type Context = CommandContext;
    type Result = Uuid;

    async fn handle(
        &self,
        command: FailJobCommand,
        _context: &Self::Context,
    ) -> Result<Self::Result, crate::cqrs::CommandError> {
        // Load and reconstruct job state
        let events = match self
            .context
            .event_store
            .load_events(command.job_id.0, None)
            .await
        {
            Ok(events) => events,
            Err(_) => {
                return Err(crate::cqrs::CommandError::AggregateNotFound(
                    command.job_id.0,
                ));
            }
        };

        let mut job = EventSourcedJob {
            id: command.job_id.clone(),
            name: "".to_string(),
            description: None,
            command: "".to_string(),
            image: "".to_string(),
            state: crate::job_definitions::JobState::new(
                crate::job_definitions::JobState::PENDING.to_string(),
            )
            .unwrap(),
            tenant_id: None,
            worker_id: None,
            retry_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            metadata: std::collections::HashMap::new(),
            version: 0,
        };

        job.load_from_events(&events);

        // Create and apply fail event
        let fail_event = match job.fail(command.error_message) {
            Ok(event) => event,
            Err(_) => {
                return Err(crate::cqrs::CommandError::BusinessRule(
                    "Job cannot be failed in current state".to_string(),
                ));
            }
        };

        // Save the event
        let events = vec![Box::new(fail_event) as Box<dyn crate::events::DomainEvent>];
        let current_version = job.version();

        match self
            .context
            .event_store
            .save_events(command.job_id.0, &events, current_version)
            .await
        {
            Ok(_) => Ok(command.job_id.0),
            Err(EventStoreError::ConcurrencyError { expected, actual }) => {
                Err(crate::cqrs::CommandError::concurrency(expected, actual))
            }
            Err(_) => Err(crate::cqrs::CommandError::InternalError(
                "Unknown error".to_string(),
            )),
        }
    }
}

/// Query Handler for Job queries
pub struct JobQueryHandler {
    event_store: Arc<dyn EventStore>,
}

impl JobQueryHandler {
    pub fn new(event_store: Arc<dyn EventStore>) -> Self {
        Self { event_store }
    }
}

#[async_trait]
impl QueryHandler<GetJobQuery> for JobQueryHandler {
    type Context = ();

    async fn handle(
        &self,
        query: GetJobQuery,
        _context: &Self::Context,
    ) -> Result<Option<JobView>, crate::cqrs::QueryError> {
        let events = match self.event_store.load_events(query.job_id.0, None).await {
            Ok(events) => events,
            Err(_) => return Ok(None),
        };

        if events.is_empty() {
            return Ok(None);
        }

        // Reconstruct job state
        let mut job = EventSourcedJob {
            id: query.job_id.clone(),
            name: "".to_string(),
            description: None,
            command: "".to_string(),
            image: "".to_string(),
            state: crate::job_definitions::JobState::new(
                crate::job_definitions::JobState::PENDING.to_string(),
            )
            .unwrap(),
            tenant_id: None,
            worker_id: None,
            retry_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            metadata: std::collections::HashMap::new(),
            version: 0,
        };

        job.load_from_events(&events);

        // Create read model
        let view = JobView {
            job_id: job.id,
            name: job.name,
            command: job.command,
            image: job.image,
            state: job.state,
            tenant_id: job.tenant_id,
            worker_id: job.worker_id,
            retry_count: job.retry_count,
            created_at: job.created_at,
            updated_at: job.updated_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            metadata: job.metadata,
        };

        Ok(Some(view))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_registry::EventRegistry;
    use crate::events::InMemoryEventStore;

    #[test]
    fn test_command_context_creation() {
        let store = Arc::new(InMemoryEventStore::new());
        let registry = Arc::new(EventRegistry::new());
        let context = CommandContext::new(store, registry);

        assert!(context.event_store.len() > 0);
    }

    #[tokio::test]
    async fn test_create_job_handler() {
        let store = Arc::new(InMemoryEventStore::new());
        let registry = Arc::new(EventRegistry::new());
        let context = Arc::new(CommandContext::new(store, registry));
        let handler = JobCommandHandler::new(context);

        let command = CreateJobCommand {
            job_id: crate::job_definitions::JobId::new(),
            tenant_id: Some("tenant-1".to_string()),
            name: "Test Job".to_string(),
            command: "echo hello".to_string(),
            image: "ubuntu:latest".to_string(),
            correlation_id: Some(Uuid::new_v4()),
        };

        let result = handler.handle(command, &handler.context).await;
        assert!(result.is_ok());
    }
}
