//! Resilience Policy Port
//!
//! Defines the interface for managing resilience policies (retry, circuit breaker, timeout)
//! in the orchestrator bounded context.

use async_trait::async_trait;
use hodei_pipelines_domain::Result;
use std::time::Duration;

/// Configuration for circuit breaker pattern
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Maximum number of failures before opening circuit
    pub failure_threshold: u32,
    /// Timeout in seconds before attempting to close circuit
    pub timeout_secs: u64,
    /// Minimum number of requests required to evaluate success rate
    pub minimum_requests: u32,
    /// Success rate threshold (0.0 to 1.0)
    pub success_rate_threshold: f64,
}

/// Configuration for retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicyConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u8,
    /// Base delay in milliseconds
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
    /// Backoff multiplier (e.g., 2.0 for exponential backoff)
    pub backoff_multiplier: f64,
    /// Jitter percentage (0.0 to 1.0)
    pub jitter: f64,
}

/// Configuration for timeout policy
#[derive(Debug, Clone)]
pub struct TimeoutPolicyConfig {
    /// Default timeout for operations
    pub default_timeout: Duration,
    /// Timeout for pipeline execution
    pub pipeline_timeout: Duration,
    /// Timeout for individual steps
    pub step_timeout: Duration,
}

/// Configuration for fallback strategy
#[derive(Debug, Clone)]
pub struct FallbackConfig {
    /// Enable fallback on failure
    pub enabled: bool,
    /// Fallback strategy type
    pub strategy: FallbackStrategy,
    /// Maximum fallback attempts
    pub max_attempts: u8,
}

/// Strategy for handling failures
#[derive(Debug, Clone)]
pub enum FallbackStrategy {
    /// Skip the failed operation
    Skip,
    /// Use cached result if available
    UseCache,
    /// Use default value
    UseDefault { default_value: String },
    /// Execute alternative operation
    ExecuteAlternative { operation_id: String },
}

/// Complete resilience policy configuration
#[derive(Debug, Clone)]
pub struct ResiliencePolicyConfig {
    pub circuit_breaker: CircuitBreakerConfig,
    pub retry: RetryPolicyConfig,
    pub timeout: TimeoutPolicyConfig,
    pub fallback: Option<FallbackConfig>,
}

impl Default for ResiliencePolicyConfig {
    fn default() -> Self {
        Self {
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 5,
                timeout_secs: 60,
                minimum_requests: 10,
                success_rate_threshold: 0.5,
            },
            retry: RetryPolicyConfig {
                max_attempts: 3,
                base_delay_ms: 100,
                max_delay_ms: 10000,
                backoff_multiplier: 2.0,
                jitter: 0.1,
            },
            timeout: TimeoutPolicyConfig {
                default_timeout: Duration::from_secs(30),
                pipeline_timeout: Duration::from_secs(3600),
                step_timeout: Duration::from_secs(300),
            },
            fallback: None,
        }
    }
}

/// Metrics for resilience policies
#[derive(Debug, Clone)]
pub struct ResilienceMetrics {
    pub total_attempts: u64,
    pub successful_attempts: u64,
    pub failed_attempts: u64,
    pub retries: u64,
    pub circuit_breaker_state: CircuitBreakerState,
    pub average_latency_ms: u64,
}

/// State of the circuit breaker
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    /// Circuit is closed, allowing requests
    Closed,
    /// Circuit is open, rejecting requests
    Open,
    /// Circuit is half-open, allowing a test request
    HalfOpen,
}

/// Resilience Policy Port
/// Manages resilience policies for orchestrator operations
#[async_trait]
pub trait ResiliencePolicyPort: Send + Sync {
    /// Get resilience policy for a specific operation
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The ID of the operation
    /// * `tenant_id` - The tenant requesting the operation
    ///
    /// # Returns
    ///
    /// Returns the resilience policy configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the policy cannot be retrieved
    async fn get_resilience_policy(
        &self,
        operation_id: &str,
        tenant_id: &str,
    ) -> Result<ResiliencePolicyConfig>;

    /// Update resilience policy for an operation
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The ID of the operation
    /// * `tenant_id` - The tenant requesting the update
    /// * `policy` - The new policy configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the policy cannot be updated
    async fn update_resilience_policy(
        &self,
        operation_id: &str,
        tenant_id: &str,
        policy: ResiliencePolicyConfig,
    ) -> Result<()>;

    /// Get metrics for resilience policies
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The ID of the operation (optional, returns all if None)
    ///
    /// # Returns
    ///
    /// Returns the metrics for the specified operation(s)
    ///
    /// # Errors
    ///
    /// Returns an error if metrics cannot be retrieved
    async fn get_resilience_metrics(
        &self,
        operation_id: Option<&str>,
    ) -> Result<Vec<ResilienceMetrics>>;

    /// Reset circuit breaker for an operation
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The ID of the operation
    ///
    /// # Errors
    ///
    /// Returns an error if the circuit breaker cannot be reset
    async fn reset_circuit_breaker(&self, operation_id: &str) -> Result<()>;

    /// Check if an operation is allowed based on circuit breaker state
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The ID of the operation
    ///
    /// # Returns
    ///
    /// Returns true if the operation is allowed, false otherwise
    ///
    /// # Errors
    ///
    /// Returns an error if the state cannot be checked
    async fn is_operation_allowed(&self, operation_id: &str) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resilience_policy_port_exists() {
        // Test that ResiliencePolicyConfig can be created and customized
        let default_policy = ResiliencePolicyConfig::default();
        assert_eq!(default_policy.retry.max_attempts, 3);
        assert_eq!(default_policy.circuit_breaker.failure_threshold, 5);

        let custom_policy = ResiliencePolicyConfig {
            retry: RetryPolicyConfig {
                max_attempts: 5,
                ..default_policy.retry
            },
            ..default_policy
        };
        assert_eq!(custom_policy.retry.max_attempts, 5);

        // Test FallbackStrategy variants
        let skip_strategy = FallbackStrategy::Skip;
        match skip_strategy {
            FallbackStrategy::Skip => {}
            _ => panic!("Expected Skip strategy"),
        }

        let default_strategy = FallbackStrategy::UseDefault {
            default_value: "default".to_string(),
        };
        match default_strategy {
            FallbackStrategy::UseDefault { default_value } => {
                assert_eq!(default_value, "default");
            }
            _ => panic!("Expected UseDefault strategy"),
        }

        // Test CircuitBreakerState
        let state = CircuitBreakerState::Closed;
        assert_eq!(state, CircuitBreakerState::Closed);

        let state = CircuitBreakerState::Open;
        assert_eq!(state, CircuitBreakerState::Open);

        // Test ResilienceMetrics
        let metrics = ResilienceMetrics {
            total_attempts: 100,
            successful_attempts: 90,
            failed_attempts: 10,
            retries: 5,
            circuit_breaker_state: CircuitBreakerState::Closed,
            average_latency_ms: 50,
        };
        assert_eq!(metrics.total_attempts, 100);
        assert_eq!(metrics.successful_attempts, 90);
    }
}
