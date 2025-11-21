//! Concurrency patterns module

pub mod worker_pools;

pub use worker_pools::{BackpressureController, CircuitBreaker, DynamicWorkerPool};
