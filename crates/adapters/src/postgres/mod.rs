//! PostgreSQL Adapter Module
//!
//! Production-ready PostgreSQL implementations for repositories and services.

pub mod functional_tests;
pub mod job_repository;
pub mod pipeline_execution_repository;
pub mod pipeline_repository;
pub mod worker_repository;

pub use job_repository::PostgreSqlJobRepository;
pub use pipeline_execution_repository::PostgreSqlPipelineExecutionRepository;
pub use pipeline_repository::PostgreSqlPipelineRepository;
pub use worker_repository::PostgreSqlWorkerRepository;
