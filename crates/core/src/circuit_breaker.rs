//! Circuit Breaker Pattern Implementation
//!
//! Provides resilience against external service failures by preventing cascading failures.

use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Circuit Breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub recovery_timeout: Duration,
    pub expected_exception: Option<String>,
}

impl CircuitBreakerConfig {
    pub fn new(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            recovery_timeout,
            expected_exception: None,
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            expected_exception: None,
        }
    }
}

/// Circuit Breaker states
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit Breaker error
#[derive(Error, Debug)]
#[error("Circuit breaker error: {0}")]
pub struct CircuitBreakerError(pub String);

/// Simple Circuit Breaker implementation
#[derive(Debug)]
pub struct CircuitBreaker {
    state: CircuitBreakerState,
    failure_count: u32,
    last_failure_time: Option<Instant>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            last_failure_time: None,
            config,
        }
    }

    pub fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    pub async fn execute<F, T, Fut>(&mut self, operation: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
    {
        if !self.can_execute() {
            return Err(CircuitBreakerError(
                "Circuit breaker is OPEN - request blocked".to_string(),
            ));
        }

        match operation().await {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(err) => {
                self.on_failure();
                Err(CircuitBreakerError(err.to_string()))
            }
        }
    }

    fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                if let Some(last_time) = self.last_failure_time {
                    if last_time.elapsed() >= self.config.recovery_timeout {
                        self.state = CircuitBreakerState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    fn on_success(&mut self) {
        self.failure_count = 0;
        if self.state == CircuitBreakerState::HalfOpen {
            self.state = CircuitBreakerState::Closed;
        }
    }

    fn on_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());

        if self.failure_count >= self.config.failure_threshold {
            self.state = CircuitBreakerState::Open;
        }
    }

    pub fn get_state(&self) -> &CircuitBreakerState {
        &self.state
    }

    pub fn get_state_string(&self) -> String {
        format!("{:?}", self.state)
    }

    pub fn get_failure_count(&self) -> u32 {
        self.failure_count
    }

    pub async fn reset(&mut self) {
        self.state = CircuitBreakerState::Closed;
        self.failure_count = 0;
        self.last_failure_time = None;
    }
}

impl Clone for CircuitBreaker {
    fn clone(&self) -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            last_failure_time: None,
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_allows_success() {
        let mut cb = CircuitBreaker::default();
        let result = cb.execute(|| async { Ok("success") }).await.unwrap();
        assert_eq!(result, "success");
        assert_eq!(cb.get_failure_count(), 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig::new(3, Duration::from_secs(30));
        let mut cb = CircuitBreaker::new(config);

        for _ in 0..3 {
            let _ = cb
                .execute(|| async {
                    Err::<(), Box<dyn std::error::Error + Send + Sync>>(Box::new(
                        std::io::Error::new(std::io::ErrorKind::Other, "failed"),
                    ))
                })
                .await;
        }

        assert_eq!(cb.get_failure_count(), 3);
        assert_eq!(cb.get_state(), &CircuitBreakerState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_fails_fast_when_open() {
        let config = CircuitBreakerConfig::new(1, Duration::from_secs(1));
        let mut cb = CircuitBreaker::new(config);

        let _ = cb
            .execute(|| async {
                Err::<(), Box<dyn std::error::Error + Send + Sync>>(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "failed",
                )))
            })
            .await;

        assert_eq!(cb.get_state(), &CircuitBreakerState::Open);

        let result = cb.execute(|| async { Ok("should not execute") }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let mut cb = CircuitBreaker::default();

        for _ in 0..5 {
            let _ = cb
                .execute(|| async {
                    Err::<(), Box<dyn std::error::Error + Send + Sync>>(Box::new(
                        std::io::Error::new(std::io::ErrorKind::Other, "failed"),
                    ))
                })
                .await;
        }

        assert_eq!(cb.get_state(), &CircuitBreakerState::Open);

        cb.reset().await;
        assert_eq!(cb.get_state(), &CircuitBreakerState::Closed);
        assert_eq!(cb.get_failure_count(), 0);
    }
}
