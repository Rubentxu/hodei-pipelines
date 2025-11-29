//! Traces & Distributed Tracing Module
//!
//! Provides APIs for tracing and distributed tracing across services.
//! Implements US-013 from the Epic Web Frontend Production Ready.

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Request parameters for trace queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceQueryRequest {
    /// Optional tenant ID filter
    pub tenant_id: Option<String>,
    /// Optional service name filter
    pub service_name: Option<String>,
    /// Optional operation name filter
    pub operation_name: Option<String>,
    /// Optional status filter (SUCCESS, ERROR, TIMEOUT)
    pub status: Option<String>,
    /// Start time for time range filter
    pub start_time: Option<DateTime<Utc>>,
    /// End time for time range filter
    pub end_time: Option<DateTime<Utc>>,
    /// Maximum number of traces to return
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

/// Span structure representing a single operation in a trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Unique span ID
    pub span_id: String,
    /// Trace ID this span belongs to
    pub trace_id: String,
    /// Parent span ID (optional, null for root spans)
    pub parent_span_id: Option<String>,
    /// Operation name
    pub operation_name: String,
    /// Service name where the span originated
    pub service_name: String,
    /// Start timestamp
    pub start_time: DateTime<Utc>,
    /// End timestamp
    pub end_time: DateTime<Utc>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Status code (OK, ERROR, TIMEOUT)
    pub status_code: String,
    /// Key-value tags for additional span metadata
    pub tags: HashMap<String, String>,
    /// Structured logs for this span
    pub logs: Vec<SpanLog>,
}

/// Span log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanLog {
    /// Timestamp of the log
    pub timestamp: DateTime<Utc>,
    /// Log level (INFO, WARN, ERROR)
    pub level: String,
    /// Log message
    pub message: String,
    /// Additional fields
    pub fields: HashMap<String, String>,
}

/// Trace structure representing a distributed trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    /// Unique trace ID
    pub trace_id: String,
    /// Root operation name
    pub operation_name: String,
    /// Service name where the trace originated
    pub service_name: String,
    /// Start timestamp
    pub start_time: DateTime<Utc>,
    /// End timestamp
    pub end_time: DateTime<Utc>,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Tenant ID
    pub tenant_id: String,
    /// Status (SUCCESS, ERROR, TIMEOUT)
    pub status: String,
    /// Error message (optional)
    pub error_message: Option<String>,
    /// List of spans in this trace
    pub spans: Vec<Span>,
    /// Key-value tags for additional trace metadata
    pub tags: HashMap<String, String>,
}

/// Trace query response with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceQueryResponse {
    /// List of traces
    pub traces: Vec<Trace>,
    /// Total number of traces matching the query
    pub total: u64,
    /// Offset used
    pub offset: u32,
    /// Limit used
    pub limit: u32,
    /// Whether there are more results
    pub has_more: bool,
    /// Query parameters used
    pub query: TraceQueryRequest,
}

/// Service for managing and querying traces
#[derive(Debug)]
pub struct TracesService {
    /// Mock traces for demonstration
    mock_traces: Arc<Vec<Trace>>,
    /// Trace statistics cache
    trace_stats: Arc<HashMap<String, u64>>,
}

impl TracesService {
    /// Create new traces service
    pub fn new() -> Self {
        let mock_traces = Self::generate_mock_traces();
        let trace_stats = Self::generate_trace_stats(&mock_traces);

        Self {
            mock_traces: Arc::new(mock_traces),
            trace_stats: Arc::new(trace_stats),
        }
    }

    /// Get a specific trace by ID
    pub async fn get_trace(&self, trace_id: &str) -> Option<Trace> {
        self.mock_traces
            .iter()
            .find(|trace| trace.trace_id == trace_id)
            .cloned()
    }

    /// Query traces based on filters
    pub async fn query_traces(&self, request: &TraceQueryRequest) -> TraceQueryResponse {
        let mut traces = self.mock_traces.as_ref().clone();

        // Apply filters
        if let Some(ref tenant_id) = request.tenant_id {
            traces.retain(|trace| trace.tenant_id == *tenant_id);
        }

        if let Some(ref service_name) = request.service_name {
            traces.retain(|trace| trace.service_name == *service_name);
        }

        if let Some(ref operation_name) = request.operation_name {
            traces.retain(|trace| trace.operation_name == *operation_name);
        }

        if let Some(ref status) = request.status {
            traces.retain(|trace| trace.status == *status);
        }

        if let Some(start_time) = request.start_time {
            traces.retain(|trace| trace.start_time >= start_time);
        }

        if let Some(end_time) = request.end_time {
            traces.retain(|trace| trace.end_time <= end_time);
        }

        // Get total count before pagination
        let total = traces.len() as u64;

        // Apply pagination
        let offset = request.offset.unwrap_or(0);
        let limit = request.limit.unwrap_or(50);

        let paginated_traces = traces
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        let has_more = u64::from(offset + limit) < total;

        TraceQueryResponse {
            traces: paginated_traces,
            total,
            offset,
            limit,
            has_more,
            query: request.clone(),
        }
    }

    /// Generate mock traces for demonstration
    fn generate_mock_traces() -> Vec<Trace> {
        use rand::Rng;
        let mut rng = rand::rng();

        let tenants = vec!["tenant-123", "tenant-456", "tenant-789"];
        let services = vec!["hodei-server", "hwp-agent", "worker-pool"];
        let operations = vec![
            "execute-pipeline",
            "execute-step",
            "resource-allocation",
            "job-scheduling",
            "log-aggregation",
            "metric-collection",
        ];
        let statuses = vec!["SUCCESS", "ERROR", "TIMEOUT"];

        let mut traces = Vec::new();
        let now = Utc::now();

        for i in 0..100 {
            let tenant_idx = rng.random_range(0..tenants.len());
            let service_idx = rng.random_range(0..services.len());
            let operation_idx = rng.random_range(0..operations.len());
            let status_idx = rng.random_range(0..statuses.len());

            let trace_id = format!("trace-{}", i);
            let service_name = services[service_idx].to_string();
            let operation_name = operations[operation_idx].to_string();
            let tenant_id = tenants[tenant_idx].to_string();
            let status = statuses[status_idx].to_string();

            let duration_ms = rng.random_range(100..30000);
            let start_time = now - chrono::Duration::milliseconds(duration_ms);
            let end_time = start_time + chrono::Duration::milliseconds(duration_ms);

            let error_message = if status == "ERROR" {
                Some("Operation failed with timeout".to_string())
            } else {
                None
            };

            // Generate spans for this trace
            let span_count = rng.random_range(1..10);
            let mut spans = Vec::new();

            for j in 0..span_count {
                let span_duration = duration_ms / span_count;
                let span_start =
                    start_time + chrono::Duration::milliseconds(j as i64 * span_duration as i64);
                let span_end = span_start + chrono::Duration::milliseconds(span_duration);

                let span = Span {
                    span_id: format!("{}-span-{}", trace_id, j),
                    trace_id: trace_id.clone(),
                    parent_span_id: if j > 0 {
                        Some(format!("{}-span-{}", trace_id, j - 1))
                    } else {
                        None
                    },
                    operation_name: format!("{}-step-{}", operation_name, j),
                    service_name: service_name.clone(),
                    start_time: span_start,
                    end_time: span_end,
                    duration_ms: span_duration as u64,
                    status_code: if status == "ERROR" && rng.random_bool(0.2) {
                        "ERROR".to_string()
                    } else {
                        "OK".to_string()
                    },
                    tags: {
                        let mut tags = HashMap::new();
                        tags.insert("step_number".to_string(), j.to_string());
                        tags.insert(
                            "worker_id".to_string(),
                            format!("worker-{}", rng.random_range(1..20)),
                        );
                        tags
                    },
                    logs: vec![
                        SpanLog {
                            timestamp: span_start,
                            level: "INFO".to_string(),
                            message: format!("Step {} started", j),
                            fields: HashMap::new(),
                        },
                        SpanLog {
                            timestamp: span_end,
                            level: "INFO".to_string(),
                            message: format!("Step {} completed", j),
                            fields: HashMap::new(),
                        },
                    ],
                };

                spans.push(span);
            }

            let tags = {
                let mut tags = HashMap::new();
                tags.insert(
                    "pipeline_id".to_string(),
                    format!("pipeline-{}", rng.random_range(1..20)),
                );
                tags.insert("environment".to_string(), "production".to_string());
                tags.insert("version".to_string(), "1.0.0".to_string());
                tags
            };

            traces.push(Trace {
                trace_id,
                operation_name,
                service_name,
                start_time,
                end_time,
                duration_ms: duration_ms as u64,
                tenant_id,
                status,
                error_message,
                spans,
                tags,
            });
        }

        traces
    }

    /// Generate trace statistics
    fn generate_trace_stats(traces: &[Trace]) -> HashMap<String, u64> {
        let mut stats = HashMap::new();

        stats.insert("total_traces".to_string(), traces.len() as u64);
        stats.insert(
            "success_traces".to_string(),
            traces.iter().filter(|t| t.status == "SUCCESS").count() as u64,
        );
        stats.insert(
            "error_traces".to_string(),
            traces.iter().filter(|t| t.status == "ERROR").count() as u64,
        );
        stats.insert(
            "timeout_traces".to_string(),
            traces.iter().filter(|t| t.status == "TIMEOUT").count() as u64,
        );

        // Calculate avg duration
        let total_duration: u64 = traces.iter().map(|t| t.duration_ms).sum();
        let avg_duration = if !traces.is_empty() {
            total_duration / traces.len() as u64
        } else {
            0
        };
        stats.insert("avg_duration_ms".to_string(), avg_duration);

        stats
    }
}

impl Default for TracesService {
    fn default() -> Self {
        Self::new()
    }
}

/// Application state for Traces API
#[derive(Clone)]
pub struct TracesApiAppState {
    pub service: Arc<TracesService>,
}

/// GET /api/v1/traces/:id - Get a specific trace by ID
#[allow(dead_code)]
pub async fn get_trace_handler(
    State(state): State<TracesApiAppState>,
    Path(trace_id): Path<String>,
) -> Result<Json<Trace>, StatusCode> {
    info!("🔍 Trace requested: {}", trace_id);

    match state.service.get_trace(&trace_id).await {
        Some(trace) => {
            info!("✅ Trace found: {}", trace_id);
            Ok(Json(trace))
        }
        None => {
            info!("❌ Trace not found: {}", trace_id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// GET /api/v1/traces - Query traces with filters
#[allow(dead_code)]
pub async fn query_traces_handler(
    State(state): State<TracesApiAppState>,
    Query(params): Query<TraceQueryRequest>,
) -> Result<Json<TraceQueryResponse>, StatusCode> {
    info!("🔍 Trace query requested: {:?}", params);

    let response = state.service.query_traces(&params).await;

    info!(
        "✅ Trace query returned {} of {} total traces",
        response.traces.len(),
        response.total
    );

    Ok(Json(response))
}

/// Traces API routes
pub fn traces_api_routes() -> Router<TracesApiAppState> {
    Router::new()
        .route("/traces/{id}", get(get_trace_handler))
        .route("/traces", get(query_traces_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_traces_service_new() {
        let service = TracesService::new();
        assert!(!service.mock_traces.is_empty());
    }

    #[tokio::test]
    async fn test_traces_service_get_trace() {
        let service = TracesService::new();

        // Get first trace
        if let Some(trace) = service.mock_traces.first() {
            let found_trace = service.get_trace(&trace.trace_id).await;
            assert!(found_trace.is_some());
            assert_eq!(found_trace.unwrap().trace_id, trace.trace_id);
        }
    }

    #[tokio::test]
    async fn test_traces_service_get_nonexistent_trace() {
        let service = TracesService::new();

        let found_trace = service.get_trace("nonexistent").await;
        assert!(found_trace.is_none());
    }

    #[tokio::test]
    async fn test_traces_service_query_with_filters() {
        let service = TracesService::new();

        let request = TraceQueryRequest {
            tenant_id: Some("tenant-123".to_string()),
            service_name: Some("hodei-server".to_string()),
            operation_name: None,
            status: None,
            start_time: None,
            end_time: None,
            limit: Some(50),
            offset: Some(0),
        };

        let response = service.query_traces(&request).await;

        // Verify results are filtered
        assert!(response.traces.len() <= 50);
        for trace in &response.traces {
            assert_eq!(trace.tenant_id, "tenant-123");
            assert_eq!(trace.service_name, "hodei-server");
        }
    }

    #[tokio::test]
    async fn test_traces_service_query_with_pagination() {
        let service = TracesService::new();

        let request = TraceQueryRequest {
            tenant_id: None,
            service_name: None,
            operation_name: None,
            status: None,
            start_time: None,
            end_time: None,
            limit: Some(10),
            offset: Some(0),
        };

        let response = service.query_traces(&request).await;

        assert_eq!(response.traces.len(), 10);
        assert!(response.has_more);
    }

    #[tokio::test]
    async fn test_trace_serialization() {
        let trace = Trace {
            trace_id: "trace-1".to_string(),
            operation_name: "execute-pipeline".to_string(),
            service_name: "hodei-server".to_string(),
            start_time: Utc::now() - chrono::Duration::seconds(10),
            end_time: Utc::now(),
            duration_ms: 10000,
            tenant_id: "tenant-123".to_string(),
            status: "SUCCESS".to_string(),
            error_message: None,
            spans: vec![],
            tags: {
                let mut map = HashMap::new();
                map.insert("pipeline_id".to_string(), "pipeline-456".to_string());
                map
            },
        };

        let json = serde_json::to_string(&trace).unwrap();
        assert!(json.contains("trace_id"));
        assert!(json.contains("operation_name"));

        let deserialized: Trace = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.trace_id, trace.trace_id);
    }

    #[tokio::test]
    async fn test_span_serialization() {
        let span = Span {
            span_id: "span-1".to_string(),
            trace_id: "trace-1".to_string(),
            parent_span_id: None,
            operation_name: "execute-step".to_string(),
            service_name: "hwp-agent".to_string(),
            start_time: Utc::now() - chrono::Duration::seconds(5),
            end_time: Utc::now() - chrono::Duration::seconds(2),
            duration_ms: 3000,
            status_code: "OK".to_string(),
            tags: {
                let mut map = HashMap::new();
                map.insert("step_name".to_string(), "build".to_string());
                map
            },
            logs: vec![],
        };

        let json = serde_json::to_string(&span).unwrap();
        assert!(json.contains("span_id"));
        assert!(json.contains("operation_name"));

        let deserialized: Span = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.span_id, span.span_id);
    }

    #[tokio::test]
    async fn test_trace_query_request_serialization() {
        let request = TraceQueryRequest {
            tenant_id: Some("tenant-123".to_string()),
            service_name: Some("hodei-server".to_string()),
            operation_name: Some("execute-pipeline".to_string()),
            status: Some("SUCCESS".to_string()),
            start_time: Some(Utc::now() - chrono::Duration::hours(1)),
            end_time: Some(Utc::now()),
            limit: Some(50),
            offset: Some(0),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("tenant_id"));
        assert!(json.contains("service_name"));

        let deserialized: TraceQueryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tenant_id, request.tenant_id);
    }
}
