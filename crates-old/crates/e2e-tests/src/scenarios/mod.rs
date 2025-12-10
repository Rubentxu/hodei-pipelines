//! Test scenarios
//!
//! This module provides pre-defined test scenarios for the E2E tests.

pub mod error_handling;
pub mod happy_path;
pub mod performance;

pub use error_handling::ErrorHandlingScenario;
pub use happy_path::HappyPathScenario;
pub use performance::PerformanceScenario;

use crate::{TestEnvironment, TestResult};
use serde_json::Value;
use std::collections::HashMap;

/// Result of running a scenario
#[derive(Debug)]
pub struct ScenarioResult {
    pub name: String,
    pub passed: bool,
    pub duration_ms: u64,
    pub details: String,
    pub metrics: Option<HashMap<String, String>>,
}

impl ScenarioResult {
    /// Create a successful result
    pub fn success(name: String, duration_ms: u64, details: String) -> Self {
        Self {
            name,
            passed: true,
            duration_ms,
            details,
            metrics: None,
        }
    }

    /// Create a failed result
    pub fn failure(name: String, duration_ms: u64, details: String) -> Self {
        Self {
            name,
            passed: false,
            duration_ms,
            details,
            metrics: None,
        }
    }

    /// Add metrics to the result
    pub fn with_metrics(mut self, metrics: HashMap<String, String>) -> Self {
        self.metrics = Some(metrics);
        self
    }
}

/// Trait for test scenarios
#[async_trait::async_trait]
pub trait Scenario: Send + Sync {
    /// Get scenario name
    fn name(&self) -> &'static str;

    /// Get scenario description
    fn description(&self) -> &'static str;

    /// Run the scenario
    async fn run(&self, env: &TestEnvironment) -> TestResult<ScenarioResult>;
}

/// Runner for multiple scenarios
pub struct ScenarioRunner {
    scenarios: Vec<Box<dyn Scenario>>,
}

impl ScenarioRunner {
    /// Create a new scenario runner
    pub fn new() -> Self {
        Self {
            scenarios: Vec::new(),
        }
    }

    /// Add a scenario
    pub fn add_scenario(&mut self, scenario: Box<dyn Scenario>) {
        self.scenarios.push(scenario);
    }

    /// Add multiple scenarios
    pub fn add_scenarios(&mut self, scenarios: Vec<Box<dyn Scenario>>) {
        for scenario in scenarios {
            self.scenarios.push(scenario);
        }
    }

    /// Run all scenarios
    pub async fn run_all(&self, env: &TestEnvironment) -> TestResult<Vec<ScenarioResult>> {
        let mut results = Vec::new();

        for scenario in &self.scenarios {
            println!("\n🧪 Running scenario: {}", scenario.name());
            println!("   {}", scenario.description());

            let start = std::time::Instant::now();
            match scenario.run(env).await {
                Ok(result) => {
                    let duration = start.elapsed().as_millis() as u64;
                    let mut result = result;
                    result.duration_ms = duration;

                    if result.passed {
                        println!("   ✅ PASSED in {}ms", duration);
                    } else {
                        println!("   ❌ FAILED in {}ms", duration);
                        println!("   Error: {}", result.details);
                    }

                    results.push(result);
                }
                Err(e) => {
                    let duration = start.elapsed().as_millis() as u64;
                    let result = ScenarioResult::failure(
                        scenario.name().to_string(),
                        duration,
                        format!("Scenario execution failed: {}", e),
                    );
                    println!("   ❌ FAILED in {}ms", duration);
                    println!("   Error: {}", e);
                    results.push(result);
                }
            }
        }

        Ok(results)
    }

    /// Get summary of results
    pub fn summarize(&self, results: &[ScenarioResult]) -> String {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total - passed;

        format!(
            "\n📊 Scenario Summary: {} total, {} passed, {} failed",
            total, passed, failed
        )
    }
}

impl Default for ScenarioRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create common test scenarios
pub fn create_common_scenarios() -> Vec<Box<dyn Scenario>> {
    vec![
        Box::new(HappyPathScenario::new()),
        Box::new(ErrorHandlingScenario::new()),
        Box::new(PerformanceScenario::new()),
    ]
}
