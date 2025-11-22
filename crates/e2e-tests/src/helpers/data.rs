//! Test data generators

use serde_json::{json, Value};

/// Generator for test data
pub struct TestDataGenerator {
    counter: std::sync::atomic::AtomicU32,
}

impl TestDataGenerator {
    /// Create a new test data generator
    pub fn new() -> Self {
        Self {
            counter: std::sync::atomic::AtomicU32::new(0),
        }
    }

    fn next_id(&self) -> String {
        let id = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("test-{}", id)
    }

    /// Generate a test pipeline
    pub fn create_pipeline(&mut self) -> Value {
        let id = self.next_id();

        json!({
            "id": id,
            "name": format!("test-pipeline-{}", id),
            "description": "Test pipeline for E2E testing",
            "status": "active",
            "created_at": chrono::Utc::now()
        })
    }

    /// Generate a test job
    pub fn create_job(&mut self, pipeline_id: Option<String>) -> Value {
        let id = self.next_id();

        json!({
            "id": id,
            "pipeline_id": pipeline_id.unwrap_or_else(|| self.next_id()),
            "name": format!("test-job-{}", id),
            "status": "pending",
            "created_at": chrono::Utc::now()
        })
    }

    /// Generate a test worker configuration
    pub fn create_worker(&mut self, worker_type: &str) -> Value {
        let id = self.next_id();

        json!({
            "id": id,
            "type": worker_type,
            "name": format!("{}-worker-{}", worker_type, id),
            "status": "online",
            "created_at": chrono::Utc::now()
        })
    }
}

impl Default for TestDataGenerator {
    fn default() -> Self {
        Self::new()
    }
}
