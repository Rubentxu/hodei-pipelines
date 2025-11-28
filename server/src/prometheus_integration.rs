use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::{IntoParams, ToSchema};

use crate::AppState;

/// Prometheus metric
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrometheusMetric {
    pub metric_name: String,
    pub metric_type: PrometheusMetricType,
    pub help: String,
    pub labels: HashMap<String, String>,
}

/// Type of Prometheus metric
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrometheusMetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Prometheus configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrometheusConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub scrape_interval_ms: u64,
    pub namespace: String,
    pub additional_labels: HashMap<String, String>,
}

/// Prometheus target for scraping
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrometheusTarget {
    pub job_name: String,
    pub static_configs: Vec<String>,
    pub scrape_interval: String,
    pub metrics_path: String,
    pub scheme: String,
    pub labels: HashMap<String, String>,
}

/// Prometheus scrape configuration response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrometheusScrapeConfig {
    pub global_config: GlobalScrapeConfig,
    pub scrape_configs: Vec<PrometheusTarget>,
}

/// Global scrape configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GlobalScrapeConfig {
    pub scrape_interval: String,
    pub scrape_timeout: String,
    pub evaluation_interval: String,
}

/// Prometheus query request
#[derive(Debug, Clone, Serialize, Deserialize, IntoParams)]
pub struct PrometheusQueryRequest {
    pub query: String,
    pub time: Option<String>,
    pub step: Option<String>,
}

/// Prometheus query response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrometheusQueryResponse {
    pub status: String,
    pub data: PrometheusQueryData,
}

/// Prometheus query data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrometheusQueryData {
    pub result_type: String,
    pub result: Vec<PrometheusQueryResult>,
}

/// Prometheus query result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrometheusQueryResult {
    pub metric: HashMap<String, String>,
    pub value: Vec<String>,
}

/// Prometheus series request
#[derive(Debug, Clone, Serialize, Deserialize, IntoParams)]
pub struct PrometheusSeriesRequest {
    pub match_: Vec<String>,
    pub start: Option<String>,
    pub end: Option<String>,
}

/// Prometheus series response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrometheusSeriesResponse {
    pub status: String,
    pub data: Vec<HashMap<String, String>>,
}

/// Service for Prometheus integration
#[derive(Debug)]
pub struct PrometheusIntegrationService {
    /// Prometheus configuration
    config: Arc<RwLock<PrometheusConfig>>,
    /// Registered metrics
    metrics: Arc<RwLock<Vec<PrometheusMetric>>>,
}

impl PrometheusIntegrationService {
    /// Create new Prometheus integration service
    pub fn new() -> Self {
        let default_config = PrometheusConfig {
            enabled: true,
            endpoint: "http://localhost:9090".to_string(),
            scrape_interval_ms: 15000,
            namespace: "hodei_jobs".to_string(),
            additional_labels: HashMap::new(),
        };

        Self {
            config: Arc::new(RwLock::new(default_config)),
            metrics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a new metric
    pub async fn register_metric(&self, metric: PrometheusMetric) {
        let mut metrics = self.metrics.write().await;
        metrics.push(metric);
    }

    /// Get configuration
    pub async fn get_config(&self) -> PrometheusConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Update configuration
    pub async fn update_config(&self, config: PrometheusConfig) {
        let mut config_lock = self.config.write().await;
        *config_lock = config;
    }

    /// List all registered metrics
    pub async fn list_metrics(&self) -> Vec<PrometheusMetric> {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Generate Prometheus scrape configuration
    pub async fn generate_scrape_config(&self) -> PrometheusScrapeConfig {
        let global_config = GlobalScrapeConfig {
            scrape_interval: "15s".to_string(),
            scrape_timeout: "10s".to_string(),
            evaluation_interval: "15s".to_string(),
        };

        let target = PrometheusTarget {
            job_name: "hodei-pipelines".to_string(),
            static_configs: vec!["localhost:8080".to_string()],
            scrape_interval: "15s".to_string(),
            metrics_path: "/metrics".to_string(),
            scheme: "http".to_string(),
            labels: HashMap::from([
                ("namespace".to_string(), "hodei-pipelines".to_string()),
                ("service".to_string(), "hodei-server".to_string()),
            ]),
        };

        PrometheusScrapeConfig {
            global_config,
            scrape_configs: vec![target],
        }
    }

    /// Execute Prometheus query
    pub async fn query(&self, query: &str, time: Option<DateTime<Utc>>) -> PrometheusQueryResponse {
        // Mock implementation - in real scenario, would query Prometheus
        let timestamp = time.unwrap_or_else(|| Utc::now());
        let timestamp_str = timestamp.timestamp().to_string();

        PrometheusQueryResponse {
            status: "success".to_string(),
            data: PrometheusQueryData {
                result_type: "vector".to_string(),
                result: vec![PrometheusQueryResult {
                    metric: HashMap::from([
                        ("__name__".to_string(), query.to_string()),
                        ("instance".to_string(), "localhost:8080".to_string()),
                    ]),
                    value: vec![
                        timestamp_str.clone(),
                        (rand::random::<f64>() * 100.0).to_string(),
                    ],
                }],
            },
        }
    }

    /// Get metric series
    pub async fn series(&self, matchers: &[String]) -> PrometheusSeriesResponse {
        // Mock implementation - in real scenario, would query Prometheus
        PrometheusSeriesResponse {
            status: "success".to_string(),
            data: vec![HashMap::from([
                ("__name__".to_string(), matchers.join(",")),
                ("instance".to_string(), "localhost:8080".to_string()),
            ])],
        }
    }

    /// Get metrics endpoint for Prometheus scraping
    pub async fn get_metrics_endpoint(&self) -> String {
        "# HELP hodei_jobs_info Hodei Pipelines information\n".to_string()
            + "# TYPE hodei_jobs_info gauge\n"
            + "hodei_jobs_info{version=\"0.1.0\",build=\"dev\"} 1\n\n"
            + "# HELP hodei_jobs_uptime Hodei Pipelines uptime in seconds\n"
            + "# TYPE hodei_jobs_uptime gauge\n"
            + "hodei_jobs_uptime 3600\n\n"
            + "# HELP hodei_jobs_jobs_total Total number of jobs\n"
            + "# TYPE hodei_jobs_jobs_total counter\n"
            + "hodei_jobs_jobs_total 42\n\n"
            + "# HELP hodei_jobs_workers_total Total number of workers\n"
            + "# TYPE hodei_jobs_workers_total gauge\n"
            + "hodei_jobs_workers_total 10\n\n"
            + "# HELP hodei_jobs_queue_size Current queue size\n"
            + "# TYPE hodei_jobs_queue_size gauge\n"
            + "hodei_jobs_queue_size 5\n"
    }
}

/// Get Prometheus configuration
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/prometheus/config",
    responses(
        (status = 200, description = "Prometheus configuration retrieved successfully", body = PrometheusConfig),
        (status = 500, description = "Internal server error")
    ),
    tag = "Prometheus Integration"
)]
pub async fn get_prometheus_config(
    State(state): State<PrometheusIntegrationAppState>,
) -> Result<Json<PrometheusConfig>, StatusCode> {
    let config = state.service.get_config().await;
    Ok(Json(config))
}

/// Update Prometheus configuration
#[allow(dead_code)] //#[utoipa::path(
    put,
    path = "/api/v1/prometheus/config",
    request_body = PrometheusConfig,
    responses(
        (status = 200, description = "Prometheus configuration updated successfully"),
        (status = 400, description = "Invalid configuration"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Prometheus Integration"
)]
pub async fn update_prometheus_config(
    State(state): State<PrometheusIntegrationAppState>,
    Json(config): Json<PrometheusConfig>,
) -> Result<Json<String>, StatusCode> {
    state.service.update_config(config).await;
    Ok(Json("Configuration updated successfully".to_string()))
}

/// Get list of Prometheus metrics
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/prometheus/metrics",
    responses(
        (status = 200, description = "Metrics retrieved successfully", body = Vec<PrometheusMetric>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Prometheus Integration"
)]
pub async fn list_prometheus_metrics(
    State(state): State<PrometheusIntegrationAppState>,
) -> Result<Json<Vec<PrometheusMetric>>, StatusCode> {
    let metrics = state.service.list_metrics().await;
    Ok(Json(metrics))
}

/// Register a new metric
#[allow(dead_code)] //#[utoipa::path(
    post,
    path = "/api/v1/prometheus/metrics",
    request_body = PrometheusMetric,
    responses(
        (status = 201, description = "Metric registered successfully"),
        (status = 400, description = "Invalid metric"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Prometheus Integration"
)]
pub async fn register_prometheus_metric(
    State(state): State<PrometheusIntegrationAppState>,
    Json(metric): Json<PrometheusMetric>,
) -> Result<Json<String>, StatusCode> {
    state.service.register_metric(metric).await;
    Ok(Json("Metric registered successfully".to_string()))
}

/// Get Prometheus scrape configuration
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/prometheus/scrape-config",
    responses(
        (status = 200, description = "Scrape configuration retrieved successfully", body = PrometheusScrapeConfig),
        (status = 500, description = "Internal server error")
    ),
    tag = "Prometheus Integration"
)]
pub async fn get_scrape_config(
    State(state): State<PrometheusIntegrationAppState>,
) -> Result<Json<PrometheusScrapeConfig>, StatusCode> {
    let config = state.service.generate_scrape_config().await;
    Ok(Json(config))
}

/// Execute Prometheus query
#[allow(dead_code)] //#[utoipa::path(
    post,
    path = "/api/v1/prometheus/query",
    request_body = PrometheusQueryRequest,
    responses(
        (status = 200, description = "Query executed successfully", body = PrometheusQueryResponse),
        (status = 400, description = "Invalid query"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Prometheus Integration"
)]
pub async fn prometheus_query(
    State(state): State<PrometheusIntegrationAppState>,
    Json(request): Json<PrometheusQueryRequest>,
) -> Result<Json<PrometheusQueryResponse>, StatusCode> {
    let time = request.time.and_then(|t| t.parse().ok());
    let response = state.service.query(&request.query, time).await;
    Ok(Json(response))
}

/// Get metric series
#[allow(dead_code)] //#[utoipa::path(
    post,
    path = "/api/v1/prometheus/series",
    request_body = PrometheusSeriesRequest,
    responses(
        (status = 200, description = "Series retrieved successfully", body = PrometheusSeriesResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Prometheus Integration"
)]
pub async fn prometheus_series(
    State(state): State<PrometheusIntegrationAppState>,
    Json(request): Json<PrometheusSeriesRequest>,
) -> Result<Json<PrometheusSeriesResponse>, StatusCode> {
    let response = state.service.series(&request.match_).await;
    Ok(Json(response))
}

/// Get Prometheus metrics endpoint (for scraping)
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/metrics",
    responses(
        (status = 200, description = "Metrics exported successfully"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Prometheus Integration"
)]
pub async fn metrics_endpoint(
    State(state): State<PrometheusIntegrationAppState>,
) -> Result<String, StatusCode> {
    let metrics = state.service.get_metrics_endpoint().await;
    Ok(metrics)
}

/// Application state for Prometheus Integration
#[derive(Clone)]
pub struct PrometheusIntegrationAppState {
    pub service: Arc<PrometheusIntegrationService>,
}

/// Prometheus integration routes
pub fn prometheus_integration_routes() -> Router<PrometheusIntegrationAppState> {
    Router::new()
        .route("/prometheus/config", get(get_prometheus_config))
        .route("/prometheus/config", put(update_prometheus_config))
        .route("/prometheus/metrics", get(list_prometheus_metrics))
        .route("/prometheus/metrics", post(register_prometheus_metric))
        .route("/prometheus/scrape-config", get(get_scrape_config))
        .route("/prometheus/query", post(prometheus_query))
        .route("/prometheus/series", post(prometheus_series))
        .route("/metrics", get(metrics_endpoint))
}
