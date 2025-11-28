//! Job definition types and schemas

use crate::Uuid;
use crate::error::DomainError;
use crate::specifications::Specification;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Job state value object
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct JobState(String);

impl JobState {
    pub const PENDING: &'static str = "PENDING";
    pub const SCHEDULED: &'static str = "SCHEDULED";
    pub const RUNNING: &'static str = "RUNNING";
    pub const SUCCESS: &'static str = "SUCCESS";
    pub const FAILED: &'static str = "FAILED";
    pub const CANCELLED: &'static str = "CANCELLED";

    pub fn new(state: String) -> Result<Self, DomainError> {
        match state.as_str() {
            Self::PENDING
            | Self::SCHEDULED
            | Self::RUNNING
            | Self::SUCCESS
            | Self::FAILED
            | Self::CANCELLED => Ok(Self(state)),
            _ => Err(DomainError::Validation(format!(
                "invalid job state: {}",
                state
            ))),
        }
    }

    pub fn can_transition_to(&self, target: &Self) -> bool {
        match (self.0.as_str(), target.0.as_str()) {
            (Self::PENDING, Self::SCHEDULED) => true,
            (Self::PENDING, Self::CANCELLED) => true,
            (Self::SCHEDULED, Self::RUNNING) => true,
            (Self::SCHEDULED, Self::CANCELLED) => true,
            (Self::RUNNING, Self::SUCCESS) => true,
            (Self::RUNNING, Self::FAILED) => true,
            (Self::RUNNING, Self::CANCELLED) => true,
            (Self::FAILED, Self::PENDING) => true, // For retry
            (Self::FAILED, Self::CANCELLED) => true,
            _ => false,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.0.as_str(),
            Self::SUCCESS | Self::FAILED | Self::CANCELLED
        )
    }
}

impl std::fmt::Display for JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for JobState {
    fn from(s: String) -> Self {
        Self::new(s).expect("valid state")
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
            JobState::new(JobState::PENDING.to_string()).unwrap(),
            JobState::new(JobState::SCHEDULED.to_string()).unwrap(),
            JobState::new(JobState::RUNNING.to_string()).unwrap(),
            JobState::new(JobState::SUCCESS.to_string()).unwrap(),
            JobState::new(JobState::FAILED.to_string()).unwrap(),
            JobState::new(JobState::CANCELLED.to_string()).unwrap(),
        ];

        for state in valid_states {
            assert_eq!(state.as_str().len() > 0, true);
            assert_eq!(
                state.is_terminal(),
                matches!(
                    state.as_str(),
                    JobState::SUCCESS | JobState::FAILED | JobState::CANCELLED
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
            let result = JobState::new(state_str.to_string());
            assert!(result.is_err(), "State '{}' should be rejected", state_str);
        }
    }

    // Property 3: Valid transitions return true
    #[test]
    fn job_state_valid_transitions_return_true() {
        let valid_transitions = vec![
            (
                JobState::new(JobState::PENDING.to_string()).unwrap(),
                JobState::new(JobState::SCHEDULED.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::PENDING.to_string()).unwrap(),
                JobState::new(JobState::CANCELLED.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::SCHEDULED.to_string()).unwrap(),
                JobState::new(JobState::RUNNING.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::SCHEDULED.to_string()).unwrap(),
                JobState::new(JobState::CANCELLED.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::RUNNING.to_string()).unwrap(),
                JobState::new(JobState::SUCCESS.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::RUNNING.to_string()).unwrap(),
                JobState::new(JobState::FAILED.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::RUNNING.to_string()).unwrap(),
                JobState::new(JobState::CANCELLED.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::FAILED.to_string()).unwrap(),
                JobState::new(JobState::PENDING.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::FAILED.to_string()).unwrap(),
                JobState::new(JobState::CANCELLED.to_string()).unwrap(),
            ),
        ];

        for (from, to) in valid_transitions {
            assert!(
                from.can_transition_to(&to),
                "Transition from {} to {} should be valid",
                from.as_str(),
                to.as_str()
            );
        }
    }

    // Property 4: Invalid transitions return false
    #[test]
    fn job_state_invalid_transitions_return_false() {
        let invalid_transitions = vec![
            (
                JobState::new(JobState::PENDING.to_string()).unwrap(),
                JobState::new(JobState::RUNNING.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::PENDING.to_string()).unwrap(),
                JobState::new(JobState::SUCCESS.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::PENDING.to_string()).unwrap(),
                JobState::new(JobState::FAILED.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::SCHEDULED.to_string()).unwrap(),
                JobState::new(JobState::PENDING.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::SCHEDULED.to_string()).unwrap(),
                JobState::new(JobState::SUCCESS.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::SCHEDULED.to_string()).unwrap(),
                JobState::new(JobState::FAILED.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::RUNNING.to_string()).unwrap(),
                JobState::new(JobState::PENDING.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::RUNNING.to_string()).unwrap(),
                JobState::new(JobState::SCHEDULED.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::SUCCESS.to_string()).unwrap(),
                JobState::new(JobState::PENDING.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::SUCCESS.to_string()).unwrap(),
                JobState::new(JobState::RUNNING.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::SUCCESS.to_string()).unwrap(),
                JobState::new(JobState::FAILED.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::SUCCESS.to_string()).unwrap(),
                JobState::new(JobState::CANCELLED.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::CANCELLED.to_string()).unwrap(),
                JobState::new(JobState::PENDING.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::CANCELLED.to_string()).unwrap(),
                JobState::new(JobState::RUNNING.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::CANCELLED.to_string()).unwrap(),
                JobState::new(JobState::SUCCESS.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::FAILED.to_string()).unwrap(),
                JobState::new(JobState::RUNNING.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::FAILED.to_string()).unwrap(),
                JobState::new(JobState::SCHEDULED.to_string()).unwrap(),
            ),
            (
                JobState::new(JobState::FAILED.to_string()).unwrap(),
                JobState::new(JobState::SUCCESS.to_string()).unwrap(),
            ),
        ];

        for (from, to) in invalid_transitions {
            assert!(
                !from.can_transition_to(&to),
                "Transition from {} to {} should be invalid",
                from.as_str(),
                to.as_str()
            );
        }
    }

    // Property 5: Terminal states cannot transition to other states (except FAILED -> PENDING for retry)
    #[test]
    fn job_state_terminal_states_cannot_transition() {
        // SUCCESS and CANCELLED are true terminal states
        let terminal_states = vec![
            JobState::new(JobState::SUCCESS.to_string()).unwrap(),
            JobState::new(JobState::CANCELLED.to_string()).unwrap(),
        ];
        let all_states = vec![
            JobState::new(JobState::PENDING.to_string()).unwrap(),
            JobState::new(JobState::SCHEDULED.to_string()).unwrap(),
            JobState::new(JobState::RUNNING.to_string()).unwrap(),
            JobState::new(JobState::SUCCESS.to_string()).unwrap(),
            JobState::new(JobState::FAILED.to_string()).unwrap(),
            JobState::new(JobState::CANCELLED.to_string()).unwrap(),
        ];

        for terminal in terminal_states {
            for target in &all_states {
                if terminal.as_str() != target.as_str() {
                    assert!(
                        !terminal.can_transition_to(target),
                        "Terminal state {} should not transition to {}",
                        terminal.as_str(),
                        target.as_str()
                    );
                }
            }
        }

        // FAILED is semi-terminal: can only transition to PENDING (retry) or CANCELLED
        let failed_state = JobState::new(JobState::FAILED.to_string()).unwrap();
        let valid_failed_transitions = vec![
            JobState::new(JobState::PENDING.to_string()).unwrap(), // Retry
            JobState::new(JobState::CANCELLED.to_string()).unwrap(),
        ];
        let invalid_failed_transitions = vec![
            JobState::new(JobState::SCHEDULED.to_string()).unwrap(),
            JobState::new(JobState::RUNNING.to_string()).unwrap(),
            JobState::new(JobState::SUCCESS.to_string()).unwrap(),
        ];

        for target in &valid_failed_transitions {
            assert!(
                failed_state.can_transition_to(target),
                "FAILED state should transition to {}",
                target.as_str()
            );
        }

        for target in &invalid_failed_transitions {
            assert!(
                !failed_state.can_transition_to(target),
                "FAILED state should not transition to {}",
                target.as_str()
            );
        }
    }

    // Property 6: State machine forms a DAG (no cycles except explicit retry)
    #[test]
    fn job_state_machine_form_dag() {
        // Build adjacency list using string references
        let mut transitions: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
        transitions.insert(
            JobState::PENDING,
            vec![JobState::SCHEDULED, JobState::CANCELLED],
        );
        transitions.insert(
            JobState::SCHEDULED,
            vec![JobState::RUNNING, JobState::CANCELLED],
        );
        transitions.insert(
            JobState::RUNNING,
            vec![JobState::SUCCESS, JobState::FAILED, JobState::CANCELLED],
        );
        transitions.insert(
            JobState::FAILED,
            vec![JobState::PENDING, JobState::CANCELLED],
        );
        transitions.insert(JobState::SUCCESS, vec![]);
        transitions.insert(JobState::CANCELLED, vec![]);

        // Verify no back edges except explicit retry
        assert!(!has_path(
            &transitions,
            JobState::SUCCESS,
            JobState::PENDING
        ));
        assert!(!has_path(
            &transitions,
            JobState::SUCCESS,
            JobState::SCHEDULED
        ));
        assert!(!has_path(
            &transitions,
            JobState::SUCCESS,
            JobState::RUNNING
        ));
        assert!(!has_path(
            &transitions,
            JobState::CANCELLED,
            JobState::PENDING
        ));
        assert!(!has_path(
            &transitions,
            JobState::CANCELLED,
            JobState::SCHEDULED
        ));
        assert!(!has_path(
            &transitions,
            JobState::CANCELLED,
            JobState::RUNNING
        ));
    }

    // Helper function to check path existence in graph
    fn has_path(
        transitions: &HashMap<&'static str, Vec<&'static str>>,
        from: &'static str,
        to: &'static str,
    ) -> bool {
        if from == to {
            return true;
        }

        let mut visited = HashSet::new();
        let mut stack = vec![from];

        while let Some(current) = stack.pop() {
            if current == to {
                return true;
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if let Some(neighbors) = transitions.get(current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        stack.push(*neighbor);
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
                JobState::PENDING.to_string(),
                JobState::SCHEDULED.to_string(),
                JobState::RUNNING.to_string(),
                JobState::FAILED.to_string(),
            ]),
            to_state in prop::sample::select(vec![
                JobState::PENDING.to_string(),
                JobState::SCHEDULED.to_string(),
                JobState::RUNNING.to_string(),
                JobState::SUCCESS.to_string(),
                JobState::FAILED.to_string(),
                JobState::CANCELLED.to_string(),
            ]),
        ) {
            let from = JobState::new(from_state).unwrap();
            let to = JobState::new(to_state).unwrap();

            let is_valid = from.can_transition_to(&to);

            // Property: Terminal states (SUCCESS, CANCELLED) cannot transition anywhere
            if from.as_str() == JobState::SUCCESS || from.as_str() == JobState::CANCELLED {
                assert!(!is_valid, "Terminal state {} should not transition to {}",
                    from.as_str(), to.as_str());
            }

            // Property: Only certain transitions are valid
            match (from.as_str(), to.as_str()) {
                (JobState::PENDING, JobState::SCHEDULED) => assert!(is_valid),
                (JobState::PENDING, JobState::CANCELLED) => assert!(is_valid),
                (JobState::SCHEDULED, JobState::RUNNING) => assert!(is_valid),
                (JobState::SCHEDULED, JobState::CANCELLED) => assert!(is_valid),
                (JobState::RUNNING, JobState::SUCCESS) => assert!(is_valid),
                (JobState::RUNNING, JobState::FAILED) => assert!(is_valid),
                (JobState::RUNNING, JobState::CANCELLED) => assert!(is_valid),
                (JobState::FAILED, JobState::PENDING) => assert!(is_valid), // Retry
                (JobState::FAILED, JobState::CANCELLED) => assert!(is_valid),
                // Direct invalid jumps
                (JobState::PENDING, JobState::RUNNING) => assert!(!is_valid),
                (JobState::PENDING, JobState::SUCCESS) => assert!(!is_valid),
                (JobState::SCHEDULED, JobState::SUCCESS) => assert!(!is_valid),
                (JobState::SCHEDULED, JobState::FAILED) => assert!(!is_valid),
                (JobState::SUCCESS, _) => assert!(!is_valid),
                (JobState::CANCELLED, _) => assert!(!is_valid),
                _ => {}
            }
        }

        /// Property-based test: State construction is deterministic
        #[test]
        fn job_state_property_construction_is_deterministic(
            state_str in prop::sample::select(vec![
                JobState::PENDING.to_string(),
                JobState::SCHEDULED.to_string(),
                JobState::RUNNING.to_string(),
                JobState::SUCCESS.to_string(),
                JobState::FAILED.to_string(),
                JobState::CANCELLED.to_string(),
            ])
        ) {
            let state1 = JobState::new(state_str.clone()).unwrap();
            let state2 = JobState::new(state_str.clone()).unwrap();

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
            let result = JobState::new(invalid_state);
            assert!(result.is_err(), "Invalid state should be rejected");
        }
    }
}
