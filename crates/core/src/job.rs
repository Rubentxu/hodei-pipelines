//! Job Domain Entity with Memory Optimizations
//!
//! This module contains the Job aggregate root optimized for memory efficiency using:
//! - Arc for shared immutable data (spec, name, description)
//! - Copy-on-Write (Cow) for lazy cloning
//! - Compact representations for frequently accessed fields
//!
//! NOTE: Value objects (JobId, JobState, JobSpec, ResourceQuota) are
//! defined in shared-types crate to avoid duplication across bounded contexts.

pub use hodei_shared_types::{DomainError, ExecResult, JobId, JobSpec, JobState, ResourceQuota};

use crate::Result;
use std::borrow::Cow;
use std::sync::Arc;

/// Extension trait for serde_json::Value to estimate memory size
trait JsonSize {
    fn estimated_size(&self) -> usize;
}

impl JsonSize for serde_json::Value {
    fn estimated_size(&self) -> usize {
        match self {
            serde_json::Value::Null => 0,
            serde_json::Value::Bool(_) => 1,
            serde_json::Value::Number(_n) => 8,
            serde_json::Value::String(s) => s.len(),
            serde_json::Value::Array(arr) => arr.iter().map(|v| v.estimated_size()).sum(),
            serde_json::Value::Object(map) => {
                map.iter().map(|(k, v)| k.len() + v.estimated_size()).sum()
            }
        }
    }
}

/// Job aggregate root with memory optimizations
///
/// This entity encapsulates the business logic for job lifecycle management
/// and maintains consistency of job state transitions while optimizing memory usage
/// through Arc and Copy-on-Write patterns.
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub id: JobId,
    /// Arc for shared name (immutable after creation)
    pub name: Arc<String>,
    /// Cow for lazy cloning of description
    pub description: Option<Cow<'static, str>>,
    /// Arc for shared JobSpec (immutable)
    pub spec: Arc<JobSpec>,
    /// Current job state (small value object)
    pub state: JobState,
    /// Timestamps (8 bytes each in practice)
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Arc for shared tenant_id
    pub tenant_id: Option<Arc<String>>,
    /// Job result (can be large)
    pub result: serde_json::Value,
}

impl Job {
    /// Create a new job with PENDING state
    ///
    /// # Errors
    /// Returns `DomainError::Validation` if the job spec is invalid
    pub fn new(id: JobId, spec: JobSpec) -> Result<Self> {
        spec.validate()?;

        let now = chrono::Utc::now();
        Ok(Self {
            id,
            name: Arc::new(spec.name.clone()),
            description: None,
            spec: Arc::new(spec),
            state: JobState::new(JobState::PENDING.to_string())?,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            tenant_id: None,
            result: serde_json::Value::Null,
        })
    }

    /// Create job with description (using Cow for lazy allocation)
    pub fn with_description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Create job with tenant_id (Arc for shared ownership)
    pub fn with_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(Arc::new(tenant_id.into()));
        self
    }

    /// Get name as string slice (zero-copy)
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get description as string slice (zero-copy) or return None
    pub fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|cow| cow.as_ref())
    }

    /// Get tenant_id as string slice (zero-copy) or return None
    pub fn tenant_id(&self) -> Option<&str> {
        self.tenant_id.as_ref().map(|arc| arc.as_ref().as_str())
    }

    /// Clone job with Copy-on-Write for description (only clones if modified)
    pub fn cloned_with_description(&self, description: impl Into<Cow<'static, str>>) -> Self {
        let mut clone = self.clone();
        clone.description = Some(description.into());
        clone
    }

    /// Clone job with new tenant (Arc sharing)
    pub fn cloned_with_tenant(&self, tenant_id: impl Into<String>) -> Self {
        let mut clone = self.clone();
        clone.tenant_id = Some(Arc::new(tenant_id.into()));
        clone
    }

    /// Transition job to SCHEDULED state
    ///
    /// # Errors
    /// Returns `DomainError::InvalidStateTransition` if transition is invalid
    pub fn schedule(&mut self) -> Result<()> {
        let new_state = JobState::new(JobState::SCHEDULED.to_string())?;

        if !self.state.can_transition_to(&new_state) {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.as_str().to_string(),
                to: new_state.as_str().to_string(),
            });
        }

        self.state = new_state;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// Transition job to RUNNING state
    ///
    /// # Errors
    /// Returns `DomainError::InvalidStateTransition` if transition is invalid
    pub fn start(&mut self) -> Result<()> {
        let new_state = JobState::new(JobState::RUNNING.to_string())?;

        if !self.state.can_transition_to(&new_state) {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.as_str().to_string(),
                to: new_state.as_str().to_string(),
            });
        }

        self.state = new_state;
        self.updated_at = chrono::Utc::now();
        self.started_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Transition job to SUCCESS state (terminal)
    ///
    /// # Errors
    /// Returns `DomainError::InvalidStateTransition` if transition is invalid
    pub fn complete(&mut self) -> Result<()> {
        let new_state = JobState::new(JobState::SUCCESS.to_string())?;

        if !self.state.can_transition_to(&new_state) {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.as_str().to_string(),
                to: new_state.as_str().to_string(),
            });
        }

        self.state = new_state;
        self.updated_at = chrono::Utc::now();
        self.completed_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Transition job to FAILED state
    ///
    /// # Errors
    /// Returns `DomainError::InvalidStateTransition` if transition is invalid
    pub fn fail(&mut self) -> Result<()> {
        let new_state = JobState::new(JobState::FAILED.to_string())?;

        if !self.state.can_transition_to(&new_state) {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.as_str().to_string(),
                to: new_state.as_str().to_string(),
            });
        }

        self.state = new_state;
        self.updated_at = chrono::Utc::now();
        self.completed_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Transition job to CANCELLED state (terminal)
    ///
    /// # Errors
    /// Returns `DomainError::InvalidStateTransition` if transition is invalid
    pub fn cancel(&mut self) -> Result<()> {
        let new_state = JobState::new(JobState::CANCELLED.to_string())?;

        if !self.state.can_transition_to(&new_state) {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.as_str().to_string(),
                to: new_state.as_str().to_string(),
            });
        }

        self.state = new_state;
        self.updated_at = chrono::Utc::now();
        self.completed_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Check if job is in PENDING state
    pub fn is_pending(&self) -> bool {
        self.state.as_str() == JobState::PENDING
    }

    /// Check if job is in RUNNING state
    pub fn is_running(&self) -> bool {
        self.state.as_str() == JobState::RUNNING
    }

    /// Check if job is in a terminal state (SUCCESS, FAILED, or CANCELLED)
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// Retry a failed job by transitioning back to PENDING
    ///
    /// # Errors
    /// Returns `DomainError::InvalidStateTransition` if job is not in FAILED state
    pub fn retry(&mut self) -> Result<()> {
        if self.state.as_str() != JobState::FAILED {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.as_str().to_string(),
                to: JobState::PENDING.to_string(),
            });
        }

        let new_state = JobState::new(JobState::PENDING.to_string())?;
        self.state = new_state;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// Get estimated memory size of the job (for monitoring)
    pub fn estimated_memory_size(&self) -> usize {
        // Base size estimation (in bytes)
        let name_size = self.name.as_ref().len() + std::mem::size_of::<String>();
        let spec_size = self.spec.as_ref().estimated_size() + std::mem::size_of::<Arc<JobSpec>>();
        let description_size = self
            .description
            .as_ref()
            .map(|cow| cow.as_ref().len() + std::mem::size_of::<Cow<'static, str>>())
            .unwrap_or(0);
        let tenant_size = self
            .tenant_id
            .as_ref()
            .map(|arc| arc.as_ref().len() + std::mem::size_of::<Arc<String>>())
            .unwrap_or(0);
        let result_size = self.result.estimated_size();

        name_size
            + spec_size
            + description_size
            + tenant_size
            + result_size
            + std::mem::size_of::<Job>()
            - std::mem::size_of::<String>()
            - std::mem::size_of::<JobSpec>()
            - std::mem::size_of::<Option<Cow<'static, str>>>()
            - std::mem::size_of::<Option<Arc<String>>>()
            - std::mem::size_of::<serde_json::Value>()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Job {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("Job", 12)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", self.name.as_ref())?;
        state.serialize_field(
            "description",
            &self.description.as_ref().map(|cow| cow.as_ref()),
        )?;
        state.serialize_field("spec", self.spec.as_ref())?;
        state.serialize_field("state", &self.state)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.serialize_field("started_at", &self.started_at)?;
        state.serialize_field("completed_at", &self.completed_at)?;
        state.serialize_field(
            "tenant_id",
            &self.tenant_id.as_ref().map(|arc| arc.as_ref().as_str()),
        )?;
        state.serialize_field("result", &self.result)?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Job {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(serde::Deserialize)]
        struct JobHelper {
            id: JobId,
            name: String,
            description: Option<String>,
            spec: JobSpec,
            state: JobState,
            created_at: chrono::DateTime<chrono::Utc>,
            updated_at: chrono::DateTime<chrono::Utc>,
            started_at: Option<chrono::DateTime<chrono::Utc>>,
            completed_at: Option<chrono::DateTime<chrono::Utc>>,
            tenant_id: Option<String>,
            result: serde_json::Value,
        }

        let helper = JobHelper::deserialize(deserializer)?;

        Ok(Job {
            id: helper.id,
            name: Arc::new(helper.name),
            description: helper.description.map(|s| Cow::<'static, str>::from(s)),
            spec: Arc::new(helper.spec),
            state: helper.state,
            created_at: helper.created_at,
            updated_at: helper.updated_at,
            started_at: helper.started_at,
            completed_at: helper.completed_at,
            tenant_id: helper.tenant_id.map(|s| Arc::new(s)),
            result: helper.result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Helper function to create a valid JobSpec
    fn create_valid_job_spec() -> JobSpec {
        JobSpec {
            name: "test-job".to_string(),
            image: "test:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        }
    }

    #[test]
    fn test_job_with_arc_name() {
        let spec = create_valid_job_spec();
        let job = Job::new(JobId::new(), spec.clone()).unwrap();

        // Verify Arc is being used
        assert_eq!(job.name.as_ref(), &spec.name);
    }

    #[test]
    fn test_job_with_cow_description() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();

        job.description = Some(Cow::Borrowed("Test description"));

        assert_eq!(job.description(), Some("Test description"));
    }

    #[test]
    fn test_job_with_arc_tenant() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();

        job.tenant_id = Some(Arc::new("tenant-123".to_string()));

        assert_eq!(job.tenant_id(), Some("tenant-123"));
    }

    #[test]
    fn test_job_with_description_helper() {
        let spec = create_valid_job_spec();
        let job = Job::new(JobId::new(), spec)
            .unwrap()
            .with_description("Test description");

        assert_eq!(job.description(), Some("Test description"));
    }

    #[test]
    fn test_job_with_tenant_helper() {
        let spec = create_valid_job_spec();
        let job = Job::new(JobId::new(), spec)
            .unwrap()
            .with_tenant("tenant-123");

        assert_eq!(job.tenant_id(), Some("tenant-123"));
    }

    #[test]
    fn test_job_cloned_with_description() {
        let spec = create_valid_job_spec();
        let job = Job::new(JobId::new(), spec).unwrap();
        let cloned = job.cloned_with_description("New description");

        assert_eq!(cloned.description(), Some("New description"));
        assert_eq!(job.description(), None);
    }

    #[test]
    fn test_job_cloned_with_tenant() {
        let spec = create_valid_job_spec();
        let job = Job::new(JobId::new(), spec).unwrap();
        let cloned = job.cloned_with_tenant("tenant-456");

        assert_eq!(cloned.tenant_id(), Some("tenant-456"));
        assert_eq!(job.tenant_id(), None);
    }

    #[test]
    fn test_job_estimated_memory_size() {
        let spec = create_valid_job_spec();
        let job = Job::new(JobId::new(), spec)
            .unwrap()
            .with_description("Test description")
            .with_tenant("tenant-123");

        let size = job.estimated_memory_size();
        assert!(size > 0);
        println!("Estimated job memory size: {} bytes", size);
    }

    // ===== Original tests =====

    #[test]
    fn test_job_state_transition() {
        let pending = JobState::new(JobState::PENDING.to_string()).unwrap();
        let scheduled = JobState::new(JobState::SCHEDULED.to_string()).unwrap();
        let running = JobState::new(JobState::RUNNING.to_string()).unwrap();
        let success = JobState::new(JobState::SUCCESS.to_string()).unwrap();

        assert!(pending.can_transition_to(&scheduled));
        assert!(scheduled.can_transition_to(&running));
        assert!(running.can_transition_to(&success));
        assert!(!pending.can_transition_to(&running));
    }

    #[test]
    fn test_job_new_pending() {
        let spec = create_valid_job_spec();
        let job = Job::new(JobId::new(), spec).unwrap();

        assert_eq!(job.state.as_str(), JobState::PENDING);
        assert!(job.is_pending());
        assert!(!job.is_running());
        assert!(!job.is_terminal());
        assert!(job.started_at.is_none());
        assert!(job.completed_at.is_none());
    }

    #[test]
    fn test_job_full_lifecycle() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();

        // Full lifecycle: PENDING -> SCHEDULED -> RUNNING -> SUCCESS
        assert_eq!(job.state.as_str(), JobState::PENDING);
        assert!(job.schedule().is_ok());
        assert_eq!(job.state.as_str(), JobState::SCHEDULED);
        assert!(job.start().is_ok());
        assert_eq!(job.state.as_str(), JobState::RUNNING);
        assert!(job.complete().is_ok());
        assert_eq!(job.state.as_str(), JobState::SUCCESS);
        assert!(job.is_terminal());
    }

    #[test]
    fn test_job_retry() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();
        job.schedule().unwrap();
        job.start().unwrap();
        job.fail().unwrap();

        // Can retry from failed state
        assert!(job.retry().is_ok());
        assert_eq!(job.state.as_str(), JobState::PENDING);
        assert!(job.is_pending());
        assert!(!job.is_terminal());
    }

    #[test]
    fn test_job_invalid_state_transitions() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();

        // Cannot go directly from PENDING to RUNNING
        assert!(job.start().is_err());

        // Cannot complete from PENDING
        assert!(job.complete().is_err());

        // Cannot fail from PENDING
        assert!(job.fail().is_err());
    }
}
