//! HTTP utilities for testing
//!
//! This module provides utilities for making HTTP requests in tests.

use reqwest::Client;
use serde_json::{json, Value};
use tracing::{error, info, warn};

use crate::TestResult;

/// HTTP client wrapper with retry logic
pub struct HttpClient {
    client: Client,
    base_url: String,
    max_retries: u32,
}

impl HttpClient {
    /// Create a new HTTP client
    pub fn new(base_url: String, max_retries: u32) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            max_retries,
        }
    }

    /// Make a GET request with retry
    pub async fn get(&self, path: &str) -> TestResult<Value> {
        let url = format!("{}{}", self.base_url, path);

        for attempt in 0..=self.max_retries {
            match self.client.get(&url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let value: Value = response.json().await?;
                        info!("GET {} succeeded (attempt {})", url, attempt + 1);
                        return Ok(value);
                    } else {
                        warn!(
                            "GET {} failed with status {} (attempt {})",
                            url,
                            response.status(),
                            attempt + 1
                        );
                    }
                }
                Err(e) => {
                    error!("GET {} error (attempt {}): {}", url, attempt + 1, e);
                }
            }

            if attempt < self.max_retries {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        Err(format!("GET {} failed after {} attempts", url, self.max_retries + 1).into())
    }

    /// Make a POST request with retry
    pub async fn post(&self, path: &str, body: Value) -> TestResult<Value> {
        let url = format!("{}{}", self.base_url, path);

        for attempt in 0..=self.max_retries {
            match self.client.post(&url).json(&body).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let value: Value = response.json().await?;
                        info!("POST {} succeeded (attempt {})", url, attempt + 1);
                        return Ok(value);
                    } else {
                        warn!(
                            "POST {} failed with status {} (attempt {})",
                            url,
                            response.status(),
                            attempt + 1
                        );
                    }
                }
                Err(e) => {
                    error!("POST {} error (attempt {}): {}", url, attempt + 1, e);
                }
            }

            if attempt < self.max_retries {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        Err(format!(
            "POST {} failed after {} attempts",
            url,
            self.max_retries + 1
        )
        .into())
    }

    /// Make a DELETE request with retry
    pub async fn delete(&self, path: &str) -> TestResult<Value> {
        let url = format!("{}{}", self.base_url, path);

        for attempt in 0..=self.max_retries {
            match self.client.delete(&url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let value: Value = response.json().await?;
                        info!("DELETE {} succeeded (attempt {})", url, attempt + 1);
                        return Ok(value);
                    } else {
                        warn!(
                            "DELETE {} failed with status {} (attempt {})",
                            url,
                            response.status(),
                            attempt + 1
                        );
                    }
                }
                Err(e) => {
                    error!("DELETE {} error (attempt {}): {}", url, attempt + 1, e);
                }
            }

            if attempt < self.max_retries {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        Err(format!(
            "DELETE {} failed after {} attempts",
            url,
            self.max_retries + 1
        )
        .into())
    }

    /// Check if service is healthy
    pub async fn is_healthy(&self) -> bool {
        if let Ok(response) = self
            .client
            .get(&format!("{}/health", self.base_url))
            .send()
            .await
        {
            response.status().is_success()
        } else {
            false
        }
    }

    /// Wait for service to be healthy
    pub async fn wait_for_healthy(&self, timeout_secs: u64) -> TestResult<()> {
        info!(
            "⏳ Waiting for service at {} to be healthy...",
            self.base_url
        );

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            if self.is_healthy().await {
                info!("✅ Service is healthy!");
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        Err(format!(
            "Timeout waiting for service at {} to be healthy",
            self.base_url
        )
        .into())
    }
}

/// Builder for HTTP requests
pub struct RequestBuilder {
    client: HttpClient,
    method: String,
    path: String,
    body: Option<Value>,
}

impl RequestBuilder {
    /// Create a new request builder
    pub fn new(client: HttpClient, method: &str, path: &str) -> Self {
        Self {
            client,
            method: method.to_string(),
            path: path.to_string(),
            body: None,
        }
    }

    /// Add JSON body to request
    pub fn json(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Execute the request
    pub async fn execute(self) -> TestResult<Value> {
        match self.method.as_str() {
            "GET" => self.client.get(&self.path).await,
            "POST" => {
                if let Some(body) = self.body {
                    self.client.post(&self.path, body).await
                } else {
                    Err("POST request requires a body".into())
                }
            }
            "DELETE" => self.client.delete(&self.path).await,
            _ => Err(format!("Unsupported method: {}", self.method).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_creation() {
        let client = HttpClient::new("http://localhost:8080".to_string(), 3);
        assert_eq!(client.base_url, "http://localhost:8080");
        assert_eq!(client.max_retries, 3);
    }
}
