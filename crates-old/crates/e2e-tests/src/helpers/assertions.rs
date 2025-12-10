//! Test assertions
//!
//! This module provides custom assertions for E2E tests.

use pretty_assertions::assert_eq;
use serde_json::Value;

/// Custom test assertions
pub struct TestAssertions;

impl TestAssertions {
    /// Assert that a JSON value contains a key
    pub fn has_key(value: &Value, key: &str) {
        assert!(
            value.get(key).is_some(),
            "Expected key '{}' not found in JSON: {}",
            key,
            value
        );
    }

    /// Assert that a JSON value does not contain a key
    pub fn does_not_have_key(value: &Value, key: &str) {
        assert!(
            value.get(key).is_none(),
            "Unexpected key '{}' found in JSON: {}",
            key,
            value
        );
    }

    /// Assert that a value has a specific status
    pub fn has_status(value: &Value, expected_status: &str) {
        let status = value.get("status").expect("No 'status' field in response");

        assert_eq!(
            status.as_str().unwrap_or(""),
            expected_status,
            "Expected status '{}' but got '{}'",
            expected_status,
            status
        );
    }

    /// Assert that a list is not empty
    pub fn is_not_empty(list: &Value) {
        assert!(list.is_array(), "Expected a list but got: {}", list);

        let arr = list.as_array().unwrap();
        assert!(
            !arr.is_empty(),
            "Expected non-empty list but got an empty list"
        );
    }

    /// Assert that a list has exactly N items
    pub fn has_length(list: &Value, expected_length: usize) {
        assert!(list.is_array(), "Expected a list but got: {}", list);

        let arr = list.as_array().unwrap();
        assert_eq!(
            arr.len(),
            expected_length,
            "Expected list of length {} but got {}",
            expected_length,
            arr.len()
        );
    }

    /// Assert that a numeric value is greater than a threshold
    pub fn is_greater_than(value: &Value, threshold: f64) {
        assert!(value.is_number(), "Expected a number but got: {}", value);
        assert!(
            value.as_f64().unwrap() > threshold,
            "Expected value > {} but got {}",
            threshold,
            value
        );
    }

    /// Assert that a numeric value is within a range
    pub fn is_in_range(value: &Value, min: f64, max: f64) {
        assert!(value.is_number(), "Expected a number but got: {}", value);
        let num = value.as_f64().unwrap();
        assert!(
            num >= min && num <= max,
            "Expected value in range [{}, {}] but got {}",
            min,
            max,
            num
        );
    }

    /// Assert that a string contains a substring
    pub fn contains(value: &Value, substring: &str) {
        assert!(value.is_string(), "Expected a string but got: {}", value);
        let s = value.as_str().unwrap();
        assert!(
            s.contains(substring),
            "Expected string to contain '{}' but got '{}'",
            substring,
            s
        );
    }

    /// Assert that a service response is successful
    pub fn is_success(response: &Value) {
        if let Some(status) = response.get("status") {
            assert_ne!(
                status.as_str().unwrap_or(""),
                "error",
                "Service returned an error: {}",
                response
            );
        }
    }

    /// Assert that a service response is an error
    pub fn is_error(response: &Value) {
        if let Some(status) = response.get("status") {
            assert_eq!(
                status.as_str().unwrap_or(""),
                "error",
                "Expected an error response but got: {}",
                response
            );
        }
    }

    /// Assert that two JSON values are approximately equal (for numeric values)
    pub fn approximately_equal(a: &Value, b: &Value, tolerance: f64) {
        assert!(
            a.is_number() && b.is_number(),
            "Both values must be numbers"
        );

        let diff = (a.as_f64().unwrap() - b.as_f64().unwrap()).abs();
        assert!(
            diff < tolerance,
            "Values differ by {} which exceeds tolerance of {}",
            diff,
            tolerance
        );
    }

    /// Assert that all required fields are present
    pub fn has_all_fields(value: &Value, fields: &[&str]) {
        for field in fields {
            Self::has_key(value, field);
        }
    }
}

/// Macro for common assertions
#[macro_export]
macro_rules! assert_response_success {
    ($response:expr) => {
        assert!($response.get("status").is_some());
        assert_ne!($response["status"], "error");
    };
}

#[macro_export]
macro_rules! assert_response_has_id {
    ($response:expr) => {
        assert!($response.get("id").is_some());
        assert!($response["id"].is_string());
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_has_key() {
        let value = json!({"key": "value"});
        TestAssertions::has_key(&value, "key");
    }

    #[test]
    #[should_panic(expected = "Expected key 'missing' not found")]
    fn test_has_key_panic() {
        let value = json!({"key": "value"});
        TestAssertions::has_key(&value, "missing");
    }

    #[test]
    fn test_has_status() {
        let value = json!({"status": "healthy"});
        TestAssertions::has_status(&value, "healthy");
    }

    #[test]
    fn test_is_not_empty() {
        let value = json!([1, 2, 3]);
        TestAssertions::is_not_empty(&value);
    }

    #[test]
    fn test_has_length() {
        let value = json!([1, 2, 3]);
        TestAssertions::has_length(&value, 3);
    }

    #[test]
    fn test_is_greater_than() {
        let value = json!(10);
        TestAssertions::is_greater_than(&value, 5);
    }

    #[test]
    fn test_contains() {
        let value = json!("hello world");
        TestAssertions::contains(&value, "world");
    }

    #[test]
    fn test_is_success() {
        let value = json!({"status": "healthy"});
        TestAssertions::is_success(&value);
    }

    #[test]
    fn test_has_all_fields() {
        let value = json!({"a": 1, "b": 2, "c": 3});
        TestAssertions::has_all_fields(&value, &["a", "b", "c"]);
    }
}
