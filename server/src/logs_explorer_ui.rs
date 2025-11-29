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
use hodei_core::pipeline_execution::ExecutionId;
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug)]
pub struct LogsExplorerService {
    /// Mock log entries for demonstration
    mock_logs: Arc<Vec<LogEntry>>,
    /// Search index cache (simulated)
    search_cache: Arc<HashMap<String, Vec<usize>>>,
}

impl LogsExplorerService {
    /// Create new logs explorer service
    pub fn new() -> Self {
        let mock_logs = Self::generate_mock_logs();
        let search_cache = Self::build_search_cache(&mock_logs);

        Self {
            mock_logs: Arc::new(mock_logs),
            search_cache: Arc::new(search_cache),
        }
    }

    /// Query logs based on filters
    pub async fn query_logs(&self, request: &LogQueryRequest) -> LogQueryResponse {
        let mut logs = self.mock_logs.as_ref().clone();

        // Apply filters
        if let Some(ref tenant_id) = request.tenant_id {
            logs.retain(|log| log.tenant_id == *tenant_id);
        }

        if let Some(ref exec_id) = request.execution_id {
            logs.retain(|log| log.execution_id.to_string() == *exec_id);
        }

        if let Some(ref pipeline_id) = request.pipeline_id {
            logs.retain(|log| log.pipeline_id.as_ref() == Some(pipeline_id));
        }

        if let Some(ref log_level) = request.log_level {
            logs.retain(|log| log.log_level == *log_level);
        }

        if let Some(ref search_query) = request.search_query {
            let search_lower = search_query.to_lowercase();
            logs.retain(|log| {
                log.message.to_lowercase().contains(&search_lower)
                    || log
                        .step
                        .as_ref()
                        .map_or(false, |s| s.to_lowercase().contains(&search_lower))
            });
        }

        if let Some(start_time) = request.start_time {
            logs.retain(|log| log.timestamp >= start_time);
        }

        if let Some(end_time) = request.end_time {
            logs.retain(|log| log.timestamp <= end_time);
        }

        // Get total count before pagination
        let total = logs.len() as u64;

        // Apply pagination
        let offset = request.offset.unwrap_or(0);
        let limit = request.limit.unwrap_or(100);

        let paginated_logs = logs
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        let has_more = u64::from(offset + limit) < total;

        LogQueryResponse {
            logs: paginated_logs,
            total,
            offset,
            limit,
            has_more,
            query: request.clone(),
        }
    }

    /// Get log statistics
    pub async fn get_statistics(&self, tenant_id: Option<&str>) -> LogStatistics {
        let logs = self.mock_logs.clone();

        // Filter by tenant if specified
        let filtered_logs: Vec<_> = if let Some(tenant) = tenant_id {
            logs.iter()
                .filter(|log| log.tenant_id == tenant)
                .cloned()
                .collect()
        } else {
            logs.iter().cloned().collect()
        };

        let total_logs = filtered_logs.len() as u64;

        // Calculate logs by level
        let mut by_log_level = HashMap::new();
        for log in &filtered_logs {
            *by_log_level.entry(log.log_level.clone()).or_insert(0) += 1;
        }

        // Calculate logs by time period
        let mut by_time_period = HashMap::new();
        let now = Utc::now();

        let last_hour = filtered_logs
            .iter()
            .filter(|log| now.signed_duration_since(log.timestamp) < chrono::Duration::hours(1))
            .count() as u64;
        by_time_period.insert("last_hour".to_string(), last_hour);

        let last_day = filtered_logs
            .iter()
            .filter(|log| now.signed_duration_since(log.timestamp) < chrono::Duration::days(1))
            .count() as u64;
        by_time_period.insert("last_day".to_string(), last_day);

        let last_week = filtered_logs
            .iter()
            .filter(|log| now.signed_duration_since(log.timestamp) < chrono::Duration::days(7))
            .count() as u64;
        by_time_period.insert("last_week".to_string(), last_week);

        // Calculate top search terms (simulated)
        let top_search_terms = vec![
            "error".to_string(),
            "timeout".to_string(),
            "deployment".to_string(),
            "failed".to_string(),
            "completed".to_string(),
        ];

        // Calculate error rate
        let error_count = filtered_logs
            .iter()
            .filter(|log| log.log_level == "ERROR")
            .count() as f64;
        let error_rate = if total_logs > 0 {
            (error_count / total_logs as f64) * 100.0
        } else {
            0.0
        };

        LogStatistics {
            total_logs,
            by_log_level,
            by_time_period,
            top_search_terms,
            error_rate,
            timestamp: Utc::now(),
        }
    }

    /// Generate mock log entries for demonstration
    fn generate_mock_logs() -> Vec<LogEntry> {
        use rand::Rng;
        let mut rng = rand::rng();

        let tenants = vec!["tenant-123", "tenant-456", "tenant-789"];
        let log_levels = vec!["INFO", "WARN", "ERROR", "DEBUG"];
        let steps = vec![
            Some("checkout".to_string()),
            Some("build".to_string()),
            Some("test".to_string()),
            Some("deploy".to_string()),
            Some("verify".to_string()),
            None,
        ];
        let messages = vec![
            "Cloning repository...",
            "Installing dependencies...",
            "Running tests...",
            "Build completed successfully",
            "Build failed with error",
            "Deploying to staging...",
            "Deployment successful",
            "Deployment failed",
            "Running verification checks...",
            "All checks passed",
            "Workflow completed",
            "Timeout waiting for resource",
            "Database connection established",
            "Cache initialized",
            "Configuration loaded",
        ];

        let mut logs = Vec::new();
        let now = Utc::now();

        for i in 0..200 {
            let tenant_idx = rng.random_range(0..tenants.len());
            let level_idx = rng.random_range(0..log_levels.len());
            let step_idx = rng.random_range(0..steps.len());
            let message_idx = rng.random_range(0..messages.len());

            let exec_id = ExecutionId::new();
            let pipeline_id = format!("pipeline-{}", rng.random_range(1..10));
            let worker_id = if rng.random_bool(0.7) {
                Some(format!("worker-{}", rng.random_range(1..20)))
            } else {
                None
            };

            let timestamp = now - chrono::Duration::hours(rng.random_range(0..168));

            logs.push(LogEntry {
                id: format!("log-{}", i),
                timestamp,
                execution_id: exec_id,
                pipeline_id: Some(pipeline_id),
                tenant_id: tenants[tenant_idx].to_string(),
                log_level: log_levels[level_idx].to_string(),
                step: steps[step_idx].clone(),
                message: messages[message_idx].to_string(),
                worker_id,
                metadata: if rng.random_bool(0.3) {
                    Some(serde_json::Value::String("metadata".to_string()))
                } else {
                    None
                },
            });
        }

        logs
    }

    /// Build search cache for faster queries (simulated)
    fn build_search_cache(logs: &[LogEntry]) -> HashMap<String, Vec<usize>> {
        let mut cache = HashMap::new();

        // Index common search terms
        let common_terms = vec!["error", "timeout", "deployment", "failed", "completed"];
        for term in common_terms {
            let mut indices = Vec::new();
            for (idx, log) in logs.iter().enumerate() {
                if log.message.to_lowercase().contains(term)
                    || log
                        .step
                        .as_ref()
                        .map_or(false, |s| s.to_lowercase().contains(term))
                {
                    indices.push(idx);
                }
            }
            cache.insert(term.to_string(), indices);
        }

        cache
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
        assert!(!service.mock_logs.is_empty());
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

        assert_eq!(response.logs.len(), 10);
        assert!(response.has_more);
    }

    #[tokio::test]
    async fn test_logs_explorer_service_statistics() {
        let service = LogsExplorerService::new();

        let stats = service.get_statistics(Some("tenant-123")).await;

        assert!(stats.total_logs > 0);
        assert!(!stats.by_log_level.is_empty());
        assert!(!stats.by_time_period.is_empty());
        assert!(!stats.top_search_terms.is_empty());
        assert!(stats.error_rate >= 0.0);
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
