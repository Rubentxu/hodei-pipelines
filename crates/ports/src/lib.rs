//! Ports - Abstraction Layer
//!
//! This crate defines ports (traits) that represent the interfaces
//! needed by the application layer. These are implemented by adapters
//! in the infrastructure layer.

pub mod event_bus;
pub mod job_repository;
pub mod pipeline_repository;
pub mod worker_repository;
pub mod worker_client;

pub use crate::event_bus::{EventPublisher, EventSubscriber, SystemEvent, EventBusError};
pub use crate::job_repository::{JobRepository, JobRepositoryError};
pub use crate::pipeline_repository::{PipelineRepository, PipelineRepositoryError};
pub use crate::worker_repository::{WorkerRepository, WorkerRepositoryError};
pub use crate::worker_client::{WorkerClient, WorkerClientError, WorkerStatus};
