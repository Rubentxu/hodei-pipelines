/// HTTP client for API communication
use crate::error::{SdkError, SdkResult};
use reqwest::{header, Client as ReqwestClient, Method, RequestBuilder, Response};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, error, info};

/// HTTP client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL for the API
    pub base_url: String,

    /// API authentication token
    pub api_token: String,

    /// Request timeout in seconds
    pub timeout: Duration,

    /// Maximum number of retries
    pub max_retries: u32,
}

impl ClientConfig {
    /// Create a new client configuration
    pub fn new(base_url: impl Into<String>, api_token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_token: api_token.into(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
        }
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

/// HTTP client for making API requests
#[derive(Clone)]
pub struct HttpClient {
    client: ReqwestClient,
    config: ClientConfig,
}

impl HttpClient {
    /// Create a new HTTP client
    pub fn new(config: ClientConfig) -> SdkResult<Self> {
        let client = ReqwestClient::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| SdkError::ConnectionFailed(e.to_string()))?;

        Ok(Self { client, config })
    }

    /// Make a GET request
    pub async fn get<T>(&self, path: &str) -> SdkResult<T>
    where
        T: DeserializeOwned,
    {
        self.request(Method::GET, path, None::<&()>).await
    }

    /// Make a POST request
    pub async fn post<B, T>(&self, path: &str, body: &B) -> SdkResult<T>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        self.request(Method::POST, path, Some(body)).await
    }

    /// Make a PUT request
    pub async fn put<B, T>(&self, path: &str, body: &B) -> SdkResult<T>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        self.request(Method::PUT, path, Some(body)).await
    }

    /// Make a DELETE request
    pub async fn delete<T>(&self, path: &str) -> SdkResult<T>
    where
        T: DeserializeOwned,
    {
        self.request(Method::DELETE, path, None::<&()>).await
    }

    /// Make a generic HTTP request
    async fn request<B, T>(&self, method: Method, path: &str, body: Option<&B>) -> SdkResult<T>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        let url = format!("{}{}", self.config.base_url, path);
        debug!("Making {} request to {}", method, url);

        let mut request = self.client.request(method.clone(), &url);
        request = self.add_auth_header(request);

        if let Some(body) = body {
            request = request
                .header(header::CONTENT_TYPE, "application/json")
                .json(body);
        }

        let response = request.send().await.map_err(|e| {
            error!("Request failed: {}", e);
            if e.is_timeout() {
                SdkError::Timeout
            } else {
                SdkError::ConnectionFailed(e.to_string())
            }
        })?;

        self.handle_response(response).await
    }

    /// Add authentication header to request
    fn add_auth_header(&self, request: RequestBuilder) -> RequestBuilder {
        request.header(
            header::AUTHORIZATION,
            format!("Bearer {}", self.config.api_token),
        )
    }

    /// Handle HTTP response
    async fn handle_response<T>(&self, response: Response) -> SdkResult<T>
    where
        T: DeserializeOwned,
    {
        let status = response.status();

        if status.is_success() {
            info!("Request successful: {}", status);
            response.json().await.map_err(|e| {
                error!("Failed to deserialize response: {}", e);
                SdkError::Other(format!("Deserialization failed: {}", e))
            })
        } else {
            let status_code = status.as_u16();
            let error_body = response.text().await.unwrap_or_default();
            error!("Request failed with status {}: {}", status_code, error_body);

            match status_code {
                401 | 403 => Err(SdkError::auth_error(error_body)),
                404 => Err(SdkError::NotFound(error_body)),
                _ => Err(SdkError::api_error(status_code, error_body)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};

    #[tokio::test]
    async fn test_get_request_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api/v1/test")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message":"success"}"#)
            .create_async()
            .await;

        let config = ClientConfig::new(server.url(), "test-token");
        let client = HttpClient::new(config).unwrap();

        #[derive(Debug, serde::Deserialize)]
        struct TestResponse {
            message: String,
        }

        let result: SdkResult<TestResponse> = client.get("/api/v1/test").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().message, "success");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_request_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/test")
            .match_header("authorization", "Bearer test-token")
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(serde_json::json!({"name": "test"})))
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"123","name":"test"}"#)
            .create_async()
            .await;

        let config = ClientConfig::new(server.url(), "test-token");
        let client = HttpClient::new(config).unwrap();

        #[derive(serde::Serialize)]
        struct TestRequest {
            name: String,
        }

        #[derive(serde::Deserialize)]
        struct TestResponse {
            id: String,
            name: String,
        }

        let request = TestRequest {
            name: "test".to_string(),
        };
        let result: SdkResult<TestResponse> = client.post("/api/v1/test", &request).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.id, "123");
        assert_eq!(response.name, "test");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_unauthorized_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api/v1/test")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let config = ClientConfig::new(server.url(), "invalid-token");
        let client = HttpClient::new(config).unwrap();

        #[derive(Debug, serde::Deserialize)]
        struct TestResponse {
            message: String,
        }

        let result: SdkResult<TestResponse> = client.get("/api/v1/test").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().is_auth_error());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_not_found_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api/v1/test/999")
            .with_status(404)
            .with_body("Not found")
            .create_async()
            .await;

        let config = ClientConfig::new(server.url(), "test-token");
        let client = HttpClient::new(config).unwrap();

        #[derive(Debug, serde::Deserialize)]
        struct TestResponse {
            message: String,
        }

        let result: SdkResult<TestResponse> = client.get("/api/v1/test/999").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().is_not_found());

        mock.assert_async().await;
    }
}
