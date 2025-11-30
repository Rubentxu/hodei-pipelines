//! Job definition types and schemas

use crate::Uuid;
use crate::error::DomainError;
use crate::specifications::Specification;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Job identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct JobId(pub Uuid);

impl JobId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for JobId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Resource requirements for a job
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ResourceQuota {
    pub cpu_m: u64,      // CPU in millicores
    pub memory_mb: u64,  // Memory in MB
    pub gpu: Option<u8>, // Optional GPU requirement
}

impl ResourceQuota {
    /// Create a new ResourceQuota (legacy method without validation)
    pub fn new(cpu_m: u64, memory_mb: u64) -> Self {
        Self {
            cpu_m,
            memory_mb,
            gpu: None,
        }
    }

    /// Create a new ResourceQuota with validation
    ///
    /// # Errors
    /// Returns `crate::error::DomainError::Validation` if:
    /// - `cpu_m` is 0
    /// - `memory_mb` is 0
    pub fn create(cpu_m: u64, memory_mb: u64) -> crate::Result<Self> {
        if cpu_m == 0 {
            return Err(crate::error::DomainError::Validation(
                "CPU must be greater than 0 millicores".to_string(),
            ));
        }

        if memory_mb == 0 {
            return Err(crate::error::DomainError::Validation(
                "Memory must be greater than 0 MB".to_string(),
            ));
        }

        Ok(Self {
            cpu_m,
            memory_mb,
            gpu: None,
        })
    }

    /// Create ResourceQuota with GPU requirement
    ///
    /// # Errors
    /// Returns `crate::error::DomainError::Validation` if:
    /// - `cpu_m` is 0
    /// - `memory_mb` is 0
    /// - `gpu` is 0
    pub fn create_with_gpu(cpu_m: u64, memory_mb: u64, gpu: u8) -> crate::Result<Self> {
        if cpu_m == 0 {
            return Err(crate::error::DomainError::Validation(
                "CPU must be greater than 0 millicores".to_string(),
            ));
        }

        if memory_mb == 0 {
            return Err(crate::error::DomainError::Validation(
                "Memory must be greater than 0 MB".to_string(),
            ));
        }

        if gpu == 0 {
            return Err(crate::error::DomainError::Validation(
                "GPU must be greater than 0".to_string(),
            ));
        }

        Ok(Self {
            cpu_m,
            memory_mb,
            gpu: Some(gpu),
        })
    }
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            cpu_m: 1000,
            memory_mb: 1024,
            gpu: None,
        }
    }
}

/// Job specification (immutable value object)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct JobSpec {
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
    pub resources: ResourceQuota,
    pub timeout_ms: u64,
    pub retries: u8,
    pub env: HashMap<String, String>,
    pub secret_refs: Vec<String>,
}

impl JobSpec {
    pub fn validate(&self) -> Result<(), DomainError> {
        use crate::job_specifications::ValidJobSpec;

        let spec = ValidJobSpec::new();
        if spec.is_satisfied_by(self) {
            Ok(())
        } else {
            Err(DomainError::Validation(
                "job specification does not meet validation requirements".to_string(),
            ))
        }
    }

    /// Create a new JobSpec builder
    pub fn builder(name: String, image: String) -> JobSpecBuilder {
        JobSpecBuilder::new(name, image)
    }

    /// Estimate memory size of JobSpec (in bytes)
    pub fn estimated_size(&self) -> usize {
        self.name.len()
            + self.image.len()
            + self.command.iter().map(|s| s.len()).sum::<usize>()
            + self
                .env
                .iter()
                .map(|(k, v)| k.len() + v.len())
                .sum::<usize>()
            + self.secret_refs.iter().map(|s| s.len()).sum::<usize>()
            + std::mem::size_of::<Self>()
    }
}

/// Builder for JobSpec
pub struct JobSpecBuilder {
    name: String,
    image: String,
    command: Vec<String>,
    resources: ResourceQuota,
    timeout_ms: u64,
    retries: u8,
    env: HashMap<String, String>,
    secret_refs: Vec<String>,
}

impl JobSpecBuilder {
    pub fn new(name: String, image: String) -> Self {
        Self {
            name,
            image,
            command: Vec::new(),
            resources: ResourceQuota::default(),
            timeout_ms: 300000, // 5 minutes default
            retries: 0,
            env: HashMap::new(),
            secret_refs: Vec::new(),
        }
    }

    pub fn command(mut self, command: Vec<String>) -> Self {
        self.command = command;
        self
    }

    pub fn resources(mut self, resources: ResourceQuota) -> Self {
        self.resources = resources;
        self
    }

    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn retries(mut self, retries: u8) -> Self {
        self.retries = retries;
        self
    }

    pub fn env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    pub fn secret_refs(mut self, secret_refs: Vec<String>) -> Self {
        self.secret_refs = secret_refs;
        self
    }

    pub fn build(self) -> Result<JobSpec, DomainError> {
        let job_spec = JobSpec {
            name: self.name,
            image: self.image,
            command: self.command,
            resources: self.resources,
            timeout_ms: self.timeout_ms,
            retries: self.retries,
            env: self.env,
            secret_refs: self.secret_refs,
        };

        job_spec.validate()?;
        Ok(job_spec)
    }
}

/// Job state value object - typed enum for better type safety
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub enum JobState {
    Pending,
    Scheduled,
    Running,
    Success,
    Failed,
    Cancelled,
    Timeout,
}

impl JobState {
    /// Check if this state is terminal (no further transitions allowed)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            JobState::Success | JobState::Failed | JobState::Cancelled | JobState::Timeout
        )
    }

    /// Check if transition from this state to target state is valid
    pub fn can_transition_to(&self, target: &JobState) -> bool {
        match (self, target) {
            // Pending transitions
            (JobState::Pending, JobState::Scheduled) => true,
            (JobState::Pending, JobState::Cancelled) => true,

            // Scheduled transitions
            (JobState::Scheduled, JobState::Running) => true,
            (JobState::Scheduled, JobState::Cancelled) => true,

            // Running transitions
            (JobState::Running, JobState::Success) => true,
            (JobState::Running, JobState::Failed) => true,
            (JobState::Running, JobState::Timeout) => true,
            (JobState::Running, JobState::Cancelled) => true,

            // Failed can be retried or cancelled
            (JobState::Failed, JobState::Pending) => true, // Retry
            (JobState::Failed, JobState::Cancelled) => true,

            // No other transitions allowed
            _ => false,
        }
    }

    /// Get string representation for serialization
    pub fn as_str(&self) -> &'static str {
        match self {
            JobState::Pending => "PENDING",
            JobState::Scheduled => "SCHEDULED",
            JobState::Running => "RUNNING",
            JobState::Success => "SUCCESS",
            JobState::Failed => "FAILED",
            JobState::Cancelled => "CANCELLED",
            JobState::Timeout => "TIMEOUT",
        }
    }
}

impl std::fmt::Display for JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<String> for JobState {
    fn from(s: String) -> Self {
        JobState::try_from_str(&s).expect("Invalid job state in From<String> conversion")
    }
}

impl<'a> From<&'a str> for JobState {
    fn from(s: &'a str) -> Self {
        JobState::try_from_str(s).expect("Invalid job state in From<&str> conversion")
    }
}

/// Try to convert a string to JobState, returning an error if invalid
pub fn parse_job_state(s: &str) -> Result<JobState, String> {
    match s {
        "PENDING" => Ok(JobState::Pending),
        "SCHEDULED" => Ok(JobState::Scheduled),
        "RUNNING" => Ok(JobState::Running),
        "SUCCESS" => Ok(JobState::Success),
        "FAILED" => Ok(JobState::Failed),
        "CANCELLED" => Ok(JobState::Cancelled),
        "TIMEOUT" => Ok(JobState::Timeout),
        _ => Err(format!("invalid job state: {}", s)),
    }
}

impl JobState {
    /// Try to convert from string with proper error handling
    pub fn try_from_str(s: &str) -> Result<Self, String> {
        parse_job_state(s)
    }
}

/// Job execution result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Property-Based Testing for JobState Machine =====

    // Property 1: All valid states can be constructed
    #[test]
    fn job_state_valid_states_can_be_constructed() {
        let valid_states = vec![
            JobState::Pending,
            JobState::Scheduled,
            JobState::Running,
            JobState::Success,
            JobState::Failed,
            JobState::Cancelled,
            JobState::Timeout,
        ];

        for state in valid_states {
            assert_eq!(state.as_str().len() > 0, true);
            assert_eq!(
                state.is_terminal(),
                matches!(
                    state,
                    JobState::Success | JobState::Failed | JobState::Cancelled | JobState::Timeout
                )
            );
        }
    }

    // Property 2: Invalid states are rejected
    #[test]
    fn job_state_rejects_invalid_states() {
        let invalid_states = vec![
            "INVALID", "", "pending", "running", "COMPLETE", "CANCELED", // Wrong spelling
        ];

        for state_str in invalid_states {
            let result = JobState::try_from_str(state_str);
            assert!(result.is_err(), "State '{}' should be rejected", state_str);
        }
    }

    // Property 3: Valid transitions return true
    #[test]
    fn job_state_valid_transitions_return_true() {
        let valid_transitions = vec![
            (JobState::Pending, JobState::Scheduled),
            (JobState::Pending, JobState::Cancelled),
            (JobState::Scheduled, JobState::Running),
            (JobState::Scheduled, JobState::Cancelled),
            (JobState::Running, JobState::Success),
            (JobState::Running, JobState::Failed),
            (JobState::Running, JobState::Timeout),
            (JobState::Running, JobState::Cancelled),
            (JobState::Failed, JobState::Pending), // Retry
            (JobState::Failed, JobState::Cancelled),
        ];

        for (from, to) in valid_transitions {
            assert!(
                from.can_transition_to(&to),
                "Transition from {:?} to {:?} should be valid",
                from,
                to
            );
        }
    }

    // Property 4: Invalid transitions return false
    #[test]
    fn job_state_invalid_transitions_return_false() {
        let invalid_transitions = vec![
            (JobState::Pending, JobState::Running),
            (JobState::Pending, JobState::Success),
            (JobState::Pending, JobState::Failed),
            (JobState::Pending, JobState::Timeout),
            (JobState::Scheduled, JobState::Pending),
            (JobState::Scheduled, JobState::Success),
            (JobState::Scheduled, JobState::Failed),
            (JobState::Scheduled, JobState::Timeout),
            (JobState::Running, JobState::Pending),
            (JobState::Running, JobState::Scheduled),
            (JobState::Success, JobState::Pending),
            (JobState::Success, JobState::Scheduled),
            (JobState::Success, JobState::Running),
            (JobState::Success, JobState::Failed),
            (JobState::Success, JobState::Timeout),
            (JobState::Success, JobState::Cancelled),
            (JobState::Cancelled, JobState::Pending),
            (JobState::Cancelled, JobState::Scheduled),
            (JobState::Cancelled, JobState::Running),
            (JobState::Cancelled, JobState::Success),
            (JobState::Failed, JobState::Scheduled),
            (JobState::Failed, JobState::Running),
            (JobState::Failed, JobState::Success),
            (JobState::Timeout, JobState::Pending),
            (JobState::Timeout, JobState::Scheduled),
            (JobState::Timeout, JobState::Running),
            (JobState::Timeout, JobState::Success),
            (JobState::Timeout, JobState::Failed),
            (JobState::Timeout, JobState::Cancelled),
        ];

        for (from, to) in invalid_transitions {
            assert!(
                !from.can_transition_to(&to),
                "Transition from {:?} to {:?} should be invalid",
                from,
                to
            );
        }
    }

    // Property 5: Terminal states cannot transition to other states (except FAILED -> PENDING for retry)
    #[test]
    fn job_state_terminal_states_cannot_transition() {
        // SUCCESS, CANCELLED, and TIMEOUT are true terminal states
        let terminal_states = vec![JobState::Success, JobState::Cancelled, JobState::Timeout];
        let all_states = vec![
            JobState::Pending,
            JobState::Scheduled,
            JobState::Running,
            JobState::Success,
            JobState::Failed,
            JobState::Cancelled,
            JobState::Timeout,
        ];

        for terminal in terminal_states {
            for target in &all_states {
                if terminal != *target {
                    assert!(
                        !terminal.can_transition_to(target),
                        "Terminal state {:?} should not transition to {:?}",
                        terminal,
                        target
                    );
                }
            }
        }

        // FAILED is semi-terminal: can only transition to PENDING (retry) or CANCELLED
        let failed_state = JobState::Failed;
        let valid_failed_transitions = vec![JobState::Pending, JobState::Cancelled];
        let invalid_failed_transitions = vec![
            JobState::Scheduled,
            JobState::Running,
            JobState::Success,
            JobState::Timeout,
        ];

        for target in &valid_failed_transitions {
            assert!(
                failed_state.can_transition_to(target),
                "Failed state should transition to {:?}",
                target
            );
        }

        for target in &invalid_failed_transitions {
            assert!(
                !failed_state.can_transition_to(target),
                "Failed state should not transition to {:?}",
                target
            );
        }
    }

    // Property 6: State machine forms a DAG (no cycles except explicit retry)
    #[test]
    fn job_state_machine_form_dag() {
        // Build adjacency list using enum variants
        let mut transitions: HashMap<JobState, Vec<JobState>> = HashMap::new();
        transitions.insert(
            JobState::Pending,
            vec![JobState::Scheduled, JobState::Cancelled],
        );
        transitions.insert(
            JobState::Scheduled,
            vec![JobState::Running, JobState::Cancelled],
        );
        transitions.insert(
            JobState::Running,
            vec![
                JobState::Success,
                JobState::Failed,
                JobState::Timeout,
                JobState::Cancelled,
            ],
        );
        transitions.insert(
            JobState::Failed,
            vec![JobState::Pending, JobState::Cancelled],
        );
        transitions.insert(JobState::Success, vec![]);
        transitions.insert(JobState::Cancelled, vec![]);
        transitions.insert(JobState::Timeout, vec![]);

        // Verify no back edges except explicit retry
        assert!(!has_path(
            &transitions,
            JobState::Success,
            JobState::Pending
        ));
        assert!(!has_path(
            &transitions,
            JobState::Success,
            JobState::Scheduled
        ));
        assert!(!has_path(
            &transitions,
            JobState::Success,
            JobState::Running
        ));
        assert!(!has_path(&transitions, JobState::Success, JobState::Failed));
        assert!(!has_path(
            &transitions,
            JobState::Success,
            JobState::Timeout
        ));

        assert!(!has_path(
            &transitions,
            JobState::Cancelled,
            JobState::Pending
        ));
        assert!(!has_path(
            &transitions,
            JobState::Cancelled,
            JobState::Scheduled
        ));
        assert!(!has_path(
            &transitions,
            JobState::Cancelled,
            JobState::Running
        ));

        assert!(!has_path(
            &transitions,
            JobState::Timeout,
            JobState::Pending
        ));
        assert!(!has_path(
            &transitions,
            JobState::Timeout,
            JobState::Scheduled
        ));
        assert!(!has_path(
            &transitions,
            JobState::Timeout,
            JobState::Running
        ));
    }

    // Helper function to check path existence in graph
    fn has_path(
        transitions: &HashMap<JobState, Vec<JobState>>,
        from: JobState,
        to: JobState,
    ) -> bool {
        if from == to {
            return true;
        }

        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![from];

        while let Some(current) = stack.pop() {
            if current == to {
                return true;
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            if let Some(neighbors) = transitions.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        stack.push(neighbor.clone());
                    }
                }
            }
        }

        false
    }

    // ===== TDD Tests: ResourceQuota Validation =====

    #[test]
    fn resource_quota_rejects_zero_cpu() {
        let result = ResourceQuota::create(0, 1024);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("CPU must be greater than 0"));
        }
    }

    #[test]
    fn resource_quota_rejects_zero_memory() {
        let result = ResourceQuota::create(1000, 0);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Memory must be greater than 0"));
        }
    }

    #[test]
    fn resource_quota_rejects_zero_gpu() {
        let result = ResourceQuota::create_with_gpu(1000, 1024, 0);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("GPU must be greater than 0"));
        }
    }

    #[test]
    fn resource_quota_accepts_valid_values() {
        let quota = ResourceQuota::create(1000, 2048).unwrap();
        assert_eq!(quota.cpu_m, 1000);
        assert_eq!(quota.memory_mb, 2048);
        assert_eq!(quota.gpu, None);
    }

    #[test]
    fn resource_quota_with_gpu_accepts_valid_values() {
        let quota = ResourceQuota::create_with_gpu(2000, 4096, 1).unwrap();
        assert_eq!(quota.cpu_m, 2000);
        assert_eq!(quota.memory_mb, 4096);
        assert_eq!(quota.gpu, Some(1));
    }
}

#[cfg(test)]
mod property_based_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property-based test: All valid state transitions preserve state machine invariants
        #[test]
        fn job_state_property_valid_transitions(
            from_state in prop::sample::select(vec![
                JobState::Pending,
                JobState::Scheduled,
                JobState::Running,
                JobState::Failed,
            ]),
            to_state in prop::sample::select(vec![
                JobState::Pending,
                JobState::Scheduled,
                JobState::Running,
                JobState::Success,
                JobState::Failed,
                JobState::Cancelled,
                JobState::Timeout,
            ]),
        ) {
            let is_valid = from_state.can_transition_to(&to_state);

            // Property: Terminal states (SUCCESS, CANCELLED, TIMEOUT) cannot transition anywhere
            if matches!(
                from_state,
                JobState::Success | JobState::Cancelled | JobState::Timeout
            ) {
                assert!(
                    !is_valid,
                    "Terminal state {:?} should not transition to {:?}",
                    from_state,
                    to_state
                );
            }

            // Property: Only certain transitions are valid
            match (from_state, to_state) {
                (JobState::Pending, JobState::Scheduled) => assert!(is_valid),
                (JobState::Pending, JobState::Cancelled) => assert!(is_valid),
                (JobState::Scheduled, JobState::Running) => assert!(is_valid),
                (JobState::Scheduled, JobState::Cancelled) => assert!(is_valid),
                (JobState::Running, JobState::Success) => assert!(is_valid),
                (JobState::Running, JobState::Failed) => assert!(is_valid),
                (JobState::Running, JobState::Timeout) => assert!(is_valid),
                (JobState::Running, JobState::Cancelled) => assert!(is_valid),
                (JobState::Failed, JobState::Pending) => assert!(is_valid), // Retry
                (JobState::Failed, JobState::Cancelled) => assert!(is_valid),
                // Direct invalid jumps
                (JobState::Pending, JobState::Running) => assert!(!is_valid),
                (JobState::Pending, JobState::Success) => assert!(!is_valid),
                (JobState::Pending, JobState::Failed) => assert!(!is_valid),
                (JobState::Pending, JobState::Timeout) => assert!(!is_valid),
                (JobState::Scheduled, JobState::Success) => assert!(!is_valid),
                (JobState::Scheduled, JobState::Failed) => assert!(!is_valid),
                (JobState::Scheduled, JobState::Timeout) => assert!(!is_valid),
                (JobState::Success, _) => assert!(!is_valid),
                (JobState::Cancelled, _) => assert!(!is_valid),
                (JobState::Timeout, _) => assert!(!is_valid),
                _ => {}
            }
        }

        /// Property-based test: State construction is deterministic
        #[test]
        fn job_state_property_construction_is_deterministic(
            state_str in prop::sample::select(vec![
                "PENDING",
                "SCHEDULED",
                "RUNNING",
                "SUCCESS",
                "FAILED",
                "CANCELLED",
                "TIMEOUT",
            ])
        ) {
            let state1 = JobState::from(state_str.to_string());
            let state2 = JobState::from(state_str.to_string());

            // Property: Construction of same state should produce equal states
            assert_eq!(state1, state2);
            assert_eq!(state1.as_str(), state2.as_str());
            assert_eq!(state1.as_str(), state_str);
        }

        /// Property-based test: Invalid states are rejected
        #[test]
        fn job_state_property_rejects_invalid_states(
            invalid_state in "[A-Z]{10,20}" // Invalid: uppercase random strings
        ) {
            // Property: Invalid states should fail validation
            let result = JobState::try_from_str(&invalid_state);
            assert!(result.is_err(), "Invalid state should be rejected");
        }
    }
}
