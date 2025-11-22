/// Error types for the SDK Core
use thiserror::Error;

/// Main error type for SDK operations
#[derive(Error, Debug)]
pub enum SdkError {
    /// HTTP communication error
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Authentication error
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    ConfigurationError(String),

    /// API error response
    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Generic error
    #[error("SDK error: {0}")]
    Other(String),
}

/// Type alias for SDK results
pub type SdkResult<T> = Result<T, SdkError>;

impl SdkError {
    /// Create a new API error
    pub fn api_error(status: u16, message: impl Into<String>) -> Self {
        Self::ApiError {
            status,
            message: message.into(),
        }
    }

    /// Create a new authentication error
    pub fn auth_error(message: impl Into<String>) -> Self {
        Self::AuthenticationError(message.into())
    }

    /// Create a new configuration error
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::ConfigurationError(message.into())
    }

    /// Check if error is a 404 Not Found
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            SdkError::NotFound(_) | SdkError::ApiError { status: 404, .. }
        )
    }

    /// Check if error is authentication related
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            SdkError::AuthenticationError(_)
                | SdkError::ApiError {
                    status: 401 | 403,
                    ..
                }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_not_found() {
        let error = SdkError::NotFound("pipeline".to_string());
        assert!(error.is_not_found());

        let error = SdkError::api_error(404, "Not found");
        assert!(error.is_not_found());

        let error = SdkError::api_error(500, "Server error");
        assert!(!error.is_not_found());
    }

    #[test]
    fn test_error_is_auth_error() {
        let error = SdkError::auth_error("Invalid token");
        assert!(error.is_auth_error());

        let error = SdkError::api_error(401, "Unauthorized");
        assert!(error.is_auth_error());

        let error = SdkError::api_error(403, "Forbidden");
        assert!(error.is_auth_error());

        let error = SdkError::api_error(500, "Server error");
        assert!(!error.is_auth_error());
    }
}
