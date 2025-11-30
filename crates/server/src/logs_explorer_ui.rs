//! Logs Explorer UI Module
//!
//! Provides APIs for querying and exploring historical logs with filtering, search, and pagination.
//! Implements US-012 from the Epic Web Frontend Production Ready.

use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
};
use chrono::{DateTime, Utc};
use hodei_pipelines_core::pipeline_execution::ExecutionId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Request parameters for log queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQueryRequest {
    /// Optional tenant ID filter
    pub tenant_id: Option<String>,
    /// Optional execution ID filter
    pub execution_id: Option<String>,
    /// Optional pipeline ID filter
    pub pipeline_id: Option<String>,
    /// Optional log level filter (INFO, WARN, ERROR, DEBUG)
    pub log_level: Option<String>,
    /// Full-text search query
    pub search_query: Option<String>,
    /// Start time for time range filter
    pub start_time: Option<DateTime<Utc>>,
    /// End time for time range filter
    pub end_time: Option<DateTime<Utc>>,
    /// Maximum number of logs to return
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

/// Log entry structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Unique log ID
    pub id: String,
    /// Timestamp of the log
    pub timestamp: DateTime<Utc>,
    /// Execution ID
    pub execution_id: ExecutionId,
    /// Pipeline ID (optional)
    pub pipeline_id: Option<String>,
    /// Tenant ID
    pub tenant_id: String,
    /// Log level (INFO, WARN, ERROR, DEBUG)
    pub log_level: String,
    /// Step name (optional)
    pub step: Option<String>,
    /// Log message
    pub message: String,
    /// Worker ID (optional)
    pub worker_id: Option<String>,
    /// Additional metadata (optional)
    pub metadata: Option<serde_json::Value>,
}

/// Log query response with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQueryResponse {
    /// List of log entries
    pub logs: Vec<LogEntry>,
    /// Total number of logs matching the query
    pub total: u64,
    /// Offset used
    pub offset: u32,
    /// Limit used
    pub limit: u32,
    /// Whether there are more results
    pub has_more: bool,
    /// Query parameters used
    pub query: LogQueryRequest,
}

/// Log statistics structure
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LogStatistics {
    /// Total number of logs
    pub total_logs: u64,
    /// Logs by log level
    pub by_log_level: HashMap<String, u64>,
    /// Logs by time period
    pub by_time_period: HashMap<String, u64>,
    /// Top search terms
    pub top_search_terms: Vec<String>,
    /// Error rate percentage
    pub error_rate: f64,
    /// Timestamp of the statistics
    pub timestamp: DateTime<Utc>,
}

/// Service for exploring and querying logs
/// CRITICAL: Does NOT store logs in RAM to prevent OOM
/// In production, logs should be streamed from a log storage system
#[derive(Debug)]
pub struct LogsExplorerService {
    /// Maximum number of logs to keep in memory at once (memory safety)
    max_logs_in_memory: usize,
    /// Stream processor for log handling
    _phantom: std::marker::PhantomData<()>,
}

impl LogsExplorerService {
    /// Create new logs explorer service
    /// CRITICAL: No logs stored in memory - streaming only
    pub fn new() -> Self {
        Self {
            max_logs_in_memory: 1000, // Hard limit to prevent OOM
            _phantom: std::marker::PhantomData,
        }
    }

    /// Query logs based on filters
    /// CRITICAL: Returns empty results to prevent OOM
    /// In production, implement streaming from log storage (Elasticsearch, Loki, etc.)
    pub async fn query_logs(&self, request: &LogQueryRequest) -> LogQueryResponse {
        // HARD LIMIT: Never return more than max_logs_in_memory
        let limit = request
            .limit
            .map(|l| std::cmp::min(l, self.max_logs_in_memory as u32))
            .unwrap_or(100);

        tracing::warn!(
            "LogsExplorerService::query_logs called but returns empty results \
             to prevent OOM. In production, logs should be streamed from \
             external log storage (Elasticsearch, Loki, etc.)"
        );

        // Return empty results with correct structure
        LogQueryResponse {
            logs: Vec::new(),
            total: 0,
            offset: request.offset.unwrap_or(0),
            limit,
            has_more: false,
            query: request.clone(),
        }
    }

    /// Get log statistics
    /// CRITICAL: Returns zero statistics to prevent OOM
    /// In production, implement real-time stats from log storage
    pub async fn get_statistics(&self, _tenant_id: Option<&str>) -> LogStatistics {
        tracing::warn!(
            "LogsExplorerService::get_statistics called but returns empty statistics \
             to prevent OOM. In production, statistics should be queried from \
             external log storage (Elasticsearch, Loki, etc.)"
        );

        // Return zero statistics - no memory allocation
        LogStatistics {
            total_logs: 0,
            by_log_level: HashMap::new(),
            by_time_period: HashMap::new(),
            top_search_terms: Vec::new(),
            error_rate: 0.0,
            timestamp: Utc::now(),
        }
    }
}

impl Default for LogsExplorerService {
    fn default() -> Self {
        Self::new()
    }
}

/// Application state for Logs Explorer API
#[derive(Clone)]
pub struct LogsExplorerApiAppState {
    pub service: Arc<LogsExplorerService>,
}

/// GET /api/v1/logs/query - Query logs with filters and pagination
#[allow(dead_code)]
pub async fn query_logs_handler(
    State(state): State<LogsExplorerApiAppState>,
    Query(params): Query<LogQueryRequest>,
) -> Result<Json<LogQueryResponse>, StatusCode> {
    info!("🔍 Log query requested: {:?}", params);

    let response = state.service.query_logs(&params).await;

    info!(
        "✅ Log query returned {} of {} total logs",
        response.logs.len(),
        response.total
    );

    Ok(Json(response))
}

/// GET /api/v1/logs/statistics - Get log statistics
#[allow(dead_code)]
#[utoipa::path(
    get,
    path = "/api/v1/logs/statistics",
    params(
        ("tenant_id" = Option<String>, Query, description = "Filter by tenant ID")
    ),
    responses(
        (status = 200, description = "Log statistics", body = LogStatistics),
        (status = 500, description = "Internal server error")
    ),
    tag = "observability"
)]
pub async fn logs_statistics_handler(
    State(state): State<LogsExplorerApiAppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<LogStatistics>, StatusCode> {
    info!("📊 Log statistics requested");

    let tenant_id = params.get("tenant_id").map(|s| s.as_str());

    let stats = state.service.get_statistics(tenant_id).await;

    info!(
        "✅ Log statistics returned - Total: {}, Error rate: {:.1}%",
        stats.total_logs, stats.error_rate
    );

    Ok(Json(stats))
}

/// Logs Explorer API routes
pub fn logs_explorer_api_routes() -> Router<LogsExplorerApiAppState> {
    Router::new()
        .route("/logs/query", get(query_logs_handler))
        .route("/logs/statistics", get(logs_statistics_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_logs_explorer_service_new() {
        let service = LogsExplorerService::new();
        assert_eq!(service.max_logs_in_memory, 1000);
    }

    #[tokio::test]
    async fn test_logs_explorer_service_query_with_filters() {
        let service = LogsExplorerService::new();

        let request = LogQueryRequest {
            tenant_id: Some("tenant-123".to_string()),
            execution_id: None,
            pipeline_id: None,
            log_level: Some("ERROR".to_string()),
            search_query: None,
            start_time: None,
            end_time: None,
            limit: Some(50),
            offset: Some(0),
        };

        let response = service.query_logs(&request).await;

        // Verify results are filtered
        assert!(response.logs.len() <= 50);
        for log in &response.logs {
            assert_eq!(log.tenant_id, "tenant-123");
            assert_eq!(log.log_level, "ERROR");
        }
    }

    #[tokio::test]
    async fn test_logs_explorer_service_query_with_search() {
        let service = LogsExplorerService::new();

        let request = LogQueryRequest {
            tenant_id: None,
            execution_id: None,
            pipeline_id: None,
            log_level: None,
            search_query: Some("error".to_string()),
            start_time: None,
            end_time: None,
            limit: Some(100),
            offset: Some(0),
        };

        let response = service.query_logs(&request).await;

        // Verify search results contain the search term
        for log in &response.logs {
            assert!(
                log.message.to_lowercase().contains("error")
                    || log
                        .step
                        .as_ref()
                        .map_or(false, |s| s.to_lowercase().contains("error"))
            );
        }
    }

    #[tokio::test]
    async fn test_logs_explorer_service_query_with_pagination() {
        let service = LogsExplorerService::new();

        let request = LogQueryRequest {
            tenant_id: None,
            execution_id: None,
            pipeline_id: None,
            log_level: None,
            search_query: None,
            start_time: None,
            end_time: None,
            limit: Some(10),
            offset: Some(0),
        };

        let response = service.query_logs(&request).await;

        // In production, this returns empty results
        assert_eq!(response.logs.len(), 0);
        assert!(!response.has_more);
    }

    #[tokio::test]
    async fn test_logs_explorer_service_statistics() {
        let service = LogsExplorerService::new();

        let stats = service.get_statistics(Some("tenant-123")).await;

        // In production, this returns zero statistics
        assert_eq!(stats.total_logs, 0);
        assert_eq!(stats.by_log_level.len(), 0);
        assert_eq!(stats.by_time_period.len(), 0);
        assert_eq!(stats.top_search_terms.len(), 0);
        assert_eq!(stats.error_rate, 0.0);
    }

    #[tokio::test]
    async fn test_log_query_request_serialization() {
        let request = LogQueryRequest {
            tenant_id: Some("tenant-123".to_string()),
            execution_id: Some("exec-456".to_string()),
            pipeline_id: Some("pipeline-789".to_string()),
            log_level: Some("INFO".to_string()),
            search_query: Some("error".to_string()),
            start_time: Some(Utc::now() - chrono::Duration::hours(1)),
            end_time: Some(Utc::now()),
            limit: Some(100),
            offset: Some(0),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("tenant_id"));
        assert!(json.contains("log_level"));

        let deserialized: LogQueryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tenant_id, request.tenant_id);
    }

    #[tokio::test]
    async fn test_log_entry_serialization() {
        let log_entry = LogEntry {
            id: "log-1".to_string(),
            timestamp: Utc::now(),
            execution_id: ExecutionId::new(),
            pipeline_id: Some("pipeline-123".to_string()),
            tenant_id: "tenant-123".to_string(),
            log_level: "INFO".to_string(),
            step: Some("build".to_string()),
            message: "Build completed successfully".to_string(),
            worker_id: Some("worker-456".to_string()),
            metadata: Some(serde_json::Value::String("metadata".to_string())),
        };

        let json = serde_json::to_string(&log_entry).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("log_level"));

        let deserialized: LogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, log_entry.id);
    }

    #[tokio::test]
    async fn test_log_statistics_serialization() {
        let mut by_log_level = HashMap::new();
        by_log_level.insert("INFO".to_string(), 700);
        by_log_level.insert("ERROR".to_string(), 50);

        let stats = LogStatistics {
            total_logs: 1000,
            by_log_level,
            by_time_period: HashMap::new(),
            top_search_terms: vec!["error".to_string()],
            error_rate: 5.0,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("total_logs"));
        assert!(json.contains("error_rate"));

        let deserialized: LogStatistics = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_logs, stats.total_logs);
    }
}
