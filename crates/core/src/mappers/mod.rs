//! Database mappers for persistence layer
//!
//! This module provides the mapping layer between domain objects and database rows.
//! By extracting mapping logic into dedicated mappers, we reduce Feature Envy in
//! repository adapters and improve separation of concerns.

pub mod job_mapper;
pub mod worker_mapper;

pub use job_mapper::{JobMapper, JobRow, SqlxJobMapper};
pub use worker_mapper::{SqlxWorkerMapper, WorkerMapper, WorkerRow};
