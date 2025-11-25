
//! Scaling History API
//!
//! This module provides endpoints for querying and analyzing scaling operations history,
//! including detailed metrics and analytics for scaling events.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Scaling direction
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScalingDirection {
    ScaleIn,
    ScaleOut,
}

/// Scaling outcome
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScalingOutcome {
    Success,
    Failed,
    Cancelled,
    Partial,
}

/// Scaling history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingHistoryEntry {
    pub history_id: String,
    pub pool_id: String,
    pub policy_id: String,
    pub trigger_id: Option<String>,
    pub scaling_direction: ScalingDirection,
    pub previous_capacity: i32,
    pub new_capacity: i32,
    pub requested_capacity: i32,
    pub triggered_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub outcome: ScalingOutcome,
    pub reason: String,
    pub metadata: HashMap<String, String>,
}

/// Query scaling history request
#[derive(Debug, Deserialize)]
pub struct QueryScalingHistoryRequest {
    pub pool_id: Option<String>,
    pub policy_id: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub scaling_direction: Option<String>,
    pub outcome: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Scaling history response
#[derive(Debug, Serialize, Deserialize)]
pub struct ScalingHistoryEntryResponse {
    pub history_id: String,
    pub pool_id: String,
    pub policy_id: String,
    pub trigger_id: Option<String>,
    pub scaling_direction: String,
    pub previous_capacity: i32,
    pub new_capacity: i32,
    pub requested_capacity: i32,
    pub triggered_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub outcome: String,
    pub reason: String,
    pub metadata: HashMap<String, String>,
}

/// Scaling history aggregation
#[derive(Debug, Serialize, Deserialize)]
pub struct ScalingHistoryAggregation {
    pub pool_id: String,
    pub time_period: (DateTime<Utc>, DateTime<Utc>),
    pub total_events: u64,
    pub scale_in_events: u64,
    pub scale_out_events: u64,
    pub successful_events: u64,
    pub failed_events: u64,
    pub average_capacity_change: f64,
    pub max_capacity: i32,
    pub min_capacity: i32,
    pub average_duration_ms: Option<u64>,
}

/// Scaling statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct ScalingStatistics {
    pub pool_id: String,
    pub total_scaling_events: u64,
    pub average_events_per_hour: f64,
    pub most_active_policy: Option<String>,
    pub most_common_direction: String,
    pub success_rate: f64,
    pub average_time_between_events: Option<Duration>,
    pub peak_scaling_times: Vec<String>, // Hour strings like "09:00", "14:00"
}

/// Create scaling record request
#[derive(Debug, Deserialize)]
pub struct CreateScalingRecordRequest {
    pub pool_id: String,
    pub policy_id: String,
    pub trigger_id: Option<String>,
    pub scaling_direction: String, // scale_in, scale_out
    pub previous_capacity: i32,
    pub new_capacity: i32,
    pub requested_capacity: i32,
    pub outcome: String, // success, failed, cancelled, partial
    pub reason: String,
    pub metadata: Option<HashMap<String, String>>,
}

/// Message response
#[derive(Debug, Serialize)]
pub struct ScalingHistoryMessageResponse {
    pub message: String,
}

/// Scaling History Service
#[derive(Debug, Clone)]
pub struct ScalingHistoryService {
    /// Scaling history entries
    scaling_history: Arc<RwLock<Vec<ScalingHistoryEntry>>>,
    /// Index by pool_id for faster queries
    history_by_pool: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Index by policy_id for faster queries
    history_by_policy: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl ScalingHistoryService {
    /// Create new Scaling History Service
    pub fn new() -> Self {
        info!("Initializing Scaling History Service");
        Self {
            scaling_history: Arc::new(RwLock::new(Vec::new())),
            history_by_pool: Arc::new(RwLock::new(HashMap::new())),
            history_by_policy: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record scaling event
    pub async fn record_scaling_event(
        &self,
        request: CreateScalingRecordRequest,
    ) -> Result<ScalingHistoryEntry, String> {
        let scaling_direction = match request.scaling_direction.to_lowercase().as_str() {
            "scale_in" => ScalingDirection::ScaleIn,
            "scale_out" => ScalingDirection::ScaleOut,
            _ => return Err("Invalid scaling direction".to_string()),
        };

        let outcome = match request.outcome.to_lowercase().as_str() {
            "success" => ScalingOutcome::Success,
            "failed" => ScalingOutcome::Failed,
            "cancelled" => ScalingOutcome::Cancelled,
            "partial" => ScalingOutcome::Partial,
            _ => return Err("Invalid outcome".to_string()),
        };

        let now = Utc::now();

        let entry = ScalingHistoryEntry {
            history_id: uuid::Uuid::new_v4().to_string(),
            pool_id: request.pool_id.clone(),
            policy_id: request.policy_id.clone(),
            trigger_id: request.trigger_id,
            scaling_direction: scaling_direction.clone(),
            previous_capacity: request.previous_capacity,
            new_capacity: request.new_capacity,
            requested_capacity: request.requested_capacity,
            triggered_at: now,
            started_at: Some(now),
            completed_at: Some(now),
            duration_ms: Some(100), // Mock duration
            outcome: outcome.clone(),
            reason: request.reason,
            metadata: request.metadata.unwrap_or_else(HashMap::new),
        };

        let mut history = self.scaling_history.write().await;
        history.push(entry.clone());

        // Update indices
        let mut by_pool = self.history_by_pool.write().await;
        let mut by_policy = self.history_by_policy.write().await;

        by_pool.entry(request.pool_id.clone())
            .or_insert_with(Vec::new)
            .push(entry.history_id.clone());

        by_policy.entry(request.policy_id.clone())
            .or_insert_with(Vec::new)
            .push(entry.history_id.clone());

        info!("Recorded scaling event: {} (pool: {}, direction: {:?}, outcome: {:?})",
              entry.history_id, entry.pool_id, scaling_direction, outcome);

        Ok(entry)
    }

    /// Query scaling history
    pub async fn query_history(
        &self,
        query: QueryScalingHistoryRequest,
    ) -> Vec<ScalingHistoryEntry> {
        let history = self.scaling_history.read().await;

        let mut filtered: Vec<_> = history.iter().collect();

        // Apply filters
        if let Some(pool_id) = &query.pool_id {
            filtered.retain(|e| e.pool_id == *pool_id);
        }

        if let Some(policy_id) = &query.policy_id {
            filtered.retain(|e| e.policy_id == *policy_id);
        }

        if let Some(start_time) = query.start_time {
            filtered.retain(|e| e.triggered_at >= start_time);
        }

        if let Some(end_time) = query.end_time {
            filtered.retain(|e| e.triggered_at <= end_time);
        }

        if let Some(direction) = &query.scaling_direction {
            let direction_enum = match direction.to_lowercase().as_str() {
                "scale_in" => ScalingDirection::ScaleIn,
                "scale_out" => ScalingDirection::ScaleOut,
                _ => return Vec::new(),
            };
            filtered.retain(|e| e.scaling_direction == direction_enum);
        }

        if let Some(outcome) = &query.outcome {
            let outcome_enum = match outcome.to_lowercase().as_str() {
                "success" => ScalingOutcome::Success,
                "failed" => ScalingOutcome::Failed,
                "cancelled" => ScalingOutcome::Cancelled,
                "partial" => ScalingOutcome::Partial,
                _ => return Vec::new(),
            };
            filtered.retain(|e| e.outcome == outcome_enum);
        }

        // Sort by triggered_at descending
        filtered.sort_by(|a, b| b.triggered_at.cmp(&a.triggered_at));

        // Apply pagination
        let offset = query.offset.unwrap_or(0) as usize;
        let limit = query.limit.unwrap_or(100) as usize;

        filtered.into_iter().skip(offset).take(limit).cloned().collect()
    }

    /// Get scaling aggregation
    pub async fn get_scaling_aggregation(
        &self,
        pool_id: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<ScalingHistoryAggregation, String> {
        let history = self.scaling_history.read().await;

        let pool_history: Vec<_> = history.iter()
            .filter(|e| e.pool_id == pool_id && e.triggered_at >= start_time && e.triggered_at <= end_time)
            .collect();

        if pool_history.is_empty() {
            return Err("No scaling history found for specified criteria".to_string());
        }

        let total_events = pool_history.len() as u64;
        let scale_in_events = pool_history.iter().filter(|e| e.scaling_direction == ScalingDirection::ScaleIn).count() as u64;
        let scale_out_events = pool_history.iter().filter(|e| e.scaling_direction == ScalingDirection::ScaleOut).count() as u64;
        let successful_events = pool_history.iter().filter(|e| e.outcome == ScalingOutcome::Success).count() as u64;

        let capacity_changes: Vec<i32> = pool_history.iter()
            .map(|e| (e.new_capacity - e.previous_capacity).abs())
            .collect();

        let average_capacity_change = if capacity_changes.is_empty() {
            0.0
        } else {
            capacity_changes.iter().sum::<i32>() as f64 / capacity_changes.len() as f64
        };

        let capacities: Vec<i32> = pool_history.iter().map(|e| e.new_capacity).collect();
        let max_capacity = capacities.iter().max().copied().unwrap_or(0);
        let min_capacity = capacities.iter().min().copied().unwrap_or(0);

        let durations: Vec<u64> = pool_history.iter().filter_map(|e| e.duration_ms).collect();
        let average_duration_ms = if durations.is_empty() {
            None
        } else {
            Some(durations.iter().sum::<u64>() / durations.len() as u64)
        };

        Ok(ScalingHistoryAggregation {
            pool_id: pool_id.to_string(),
            time_period: (start_time, end_time),
            total_events,
            scale_in_events,
            scale_out_events,
            successful_events,
            failed_events: total_events - successful_events,
            average_capacity_change,
            max_capacity,
            min_capacity,
            average_duration_ms,
        })
    }

    /// Get scaling statistics
    pub async fn get_scaling_statistics(&self, pool_id: &str) -> Result<ScalingStatistics, String> {
        let history = self.scaling_history.read().await;

        let pool_history: Vec<_> = history.iter()
            .filter(|e| e.pool_id == pool_id)
            .collect();

        if pool_history.is_empty() {
            return Err("No scaling history found".to_string());
        }

        let total_events = pool_history.len() as u64;

        // Calculate most active policy
        let policy_counts: HashMap<String, usize> = pool_history.iter()
            .fold(HashMap::new(), |mut map, e| {
                *map.entry(e.policy_id.clone()).or_insert(0) += 1;
                map
            });

        let most_active_policy = policy_counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(policy, _)| policy);

        // Calculate most common direction
        let scale_in_count = pool_history.iter().filter(|e| e.scaling_direction == ScalingDirection::ScaleIn).count();
        let scale_out_count = pool_history.iter().filter(|e| e.scaling_direction == ScalingDirection::ScaleOut).count();
        let most_common_direction = if scale_in_count > scale_out_count {
            "scale_in".to_string()
        } else {
            "scale_out".to_string()
        };

        // Calculate success rate
        let successful_events = pool_history.iter().filter(|e| e.outcome == ScalingOutcome::Success).count();
        let success_rate = if total_events == 0 {
            0.0
        } else {
            (successful_events as f64 / total_events as f64) * 100.0
        };

        // Calculate average time between events (simplified)
        let mut sorted_history = pool_history.clone();
        sorted_history.sort_by(|a, b| a.triggered_at.cmp(&b.triggered_at));

        let average_time_between_events = if sorted_history.len() > 1 {
            let mut total_duration = Duration::from_secs(0);
            let mut count = 0;

            for i in 1..sorted_history.len() {
                if let Ok(duration) = sorted_history[i].triggered_at.signed_duration_since(sorted_history[i-1].triggered_at).to_std() {
                    total_duration += duration;
                    count += 1;
                }
            }

            if count > 0 {
                Some(total_duration / count as u32)
            } else {
                None
            }
        } else {
            None
        };

        // Calculate peak scaling times (simplified - by hour)
        let mut hour_counts: HashMap<String, usize> = HashMap::new();
        for entry in &pool_history {
            let hour = entry.triggered_at.format("%H:00").to_string();
            *hour_counts.entry(hour).or_insert(0) += 1;
        }

        let mut peak_scaling_times: Vec<_> = hour_counts.into_iter().collect();
        peak_scaling_times.sort_by(|a, b| b.1.cmp(&a.1));
        peak_scaling_times.truncate(3);
        let peak_times = peak_scaling_times.into_iter().map(|(hour, _)| hour).collect();

        Ok(ScalingStatistics {
            pool_id: pool_id.to_string(),
            total_scaling_events: total_events,
            average_events_per_hour: total_events as f64 / 24.0, // Simplified
            most_active_policy,
            most_common_direction,
            success_rate,
            average_time_between_events,
            peak_scaling_times: peak_times,
        })
    }

    /// Get history by ID
    pub async fn get_history_by_id(&self, history_id: &str) -> Result<ScalingHistoryEntry, String> {
        let history = self.scaling_history.read().await;
        history.iter()
            .find(|e| e.history_id == history_id)
            .cloned()
            .ok_or_else(|| "History entry not found".to_string())
    }

    /// Clean up old history entries
    pub async fn cleanup_old_history(&self, older_than: Duration) -> u64 {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(older_than).unwrap();

        let mut history = self.scaling_history.write().await;
        let old_count = history.iter().filter(|e| e.triggered_at < cutoff_time).count();

        history.retain(|e| e.triggered_at >= cutoff_time);

        info!("Cleaned up {} old scaling history entries", old_count);

        old_count as u64
    }

    /// Get total history count
    pub async fn get_history_count(&self) -> u64 {
        let history = self.scaling_history.read().await;
        history.len() as u64
    }
}

/// Application state for Scaling History
#[derive(Clone)]
pub struct ScalingHistoryAppState {
    pub service: Arc<ScalingHistoryService>,
}

/// Create router for Scaling History API
pub fn scaling_history_routes() -> Router<ScalingHistoryAppState> {
    Router::new()
        .route("/scaling-history", post(record_scaling_event_handler))
        .route("/scaling-history/query", post(query_scaling_history_handler))
        .route("/scaling-history/:history_id", get(get_scaling_history_handler))
        .route("/scaling-history/pool/:pool_id/aggregate", get(get_scaling_aggregation_handler))
        .route("/scaling-history/pool/:pool_id/statistics", get(get_scaling_statistics_handler))
        .route("/scaling-history/count", get(get_history_count_handler))
}

/// Record scaling event handler
async fn record_scaling_event_handler(
    State(state): State<ScalingHistoryAppState>,
    Json(payload): Json<CreateScalingRecordRequest>,
) -> Result<Json<ScalingHistoryMessageResponse>, StatusCode> {
    match state.service.record_scaling_event(payload).await {
        Ok(entry) => Ok(Json(ScalingHistoryMessageResponse {
            message: format!("Scaling event recorded: {}", entry.history_id),
        })),
        Err(e) => {
            error!("Failed to record scaling event: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Get scaling history handler
async fn get_scaling_history_handler(
    State(state): State<ScalingHistoryAppState>,
    axum::extract::Path(history_id): axum::extract::Path<String>,
) -> Result<Json<ScalingHistoryEntryResponse>, StatusCode> {
    match state.service.get_history_by_id(&history_id).await {
        Ok(entry) => Ok(Json(ScalingHistoryEntryResponse {
            history_id: entry.history_id,
            pool_id: entry.pool_id,
            policy_id: entry.policy_id,
            trigger_id: entry.trigger_id,
            scaling_direction: format!("{:?}", entry.scaling_direction),
            previous_capacity: entry.previous_capacity,
            new_capacity: entry.new_capacity,
            requested_capacity: entry.requested_capacity,
            triggered_at: entry.triggered_at,
            started_at: entry.started_at,
            completed_at: entry.completed_at,
            duration_ms: entry.duration_ms,
            outcome: format!("{:?}", entry.outcome),
            reason: entry.reason,
            metadata: entry.metadata,
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Query scaling history handler
async fn query_scaling_history_handler(
    State(state): State<ScalingHistoryAppState>,
    Json(query): Json<QueryScalingHistoryRequest>,
) -> Result<Json<Vec<ScalingHistoryEntryResponse>>, StatusCode> {
    let entries = state.service.query_history(query).await;

    let responses: Vec<ScalingHistoryEntryResponse> = entries
        .into_iter()
        .map(|entry| ScalingHistoryEntryResponse {
            history_id: entry.history_id,
            pool_id: entry.pool_id,
            policy_id: entry.policy_id,
            trigger_id: entry.trigger_id,
            scaling_direction: format!("{:?}", entry.scaling_direction),
            previous_capacity: entry.previous_capacity,
            new_capacity: entry.new_capacity,
            requested_capacity: entry.requested_capacity,
            triggered_at: entry.triggered_at,
            started_at: entry.started_at,
            completed_at: entry.completed_at,
            duration_ms: entry.duration_ms,
            outcome: format!("{:?}", entry.outcome),
            reason: entry.reason,
            metadata: entry.metadata,
        })
        .collect();

    Ok(Json(responses))
}

/// Get scaling aggregation handler
async fn get_scaling_aggregation_handler(
    State(state): State<ScalingHistoryAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<ScalingHistoryAggregation>, StatusCode> {
    let start_time = params.get("start_time")
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));

    let end_time = params.get("end_time")
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|| Utc::now());

    match state.service.get_scaling_aggregation(&pool_id, start_time, end_time).await {
        Ok(aggregation) => Ok(Json(aggregation)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Get scaling statistics handler
async fn get_scaling_statistics_handler(
    State(state): State<ScalingHistoryAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
) -> Result<Json<ScalingStatistics>, StatusCode> {
    match state.service.get_scaling_statistics(&pool_id).await {
        Ok(stats) => Ok(Json(stats)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Get history count handler
async fn get_history_count_handler(
    State(state): State<ScalingHistoryAppState>,
) -> Result<Json<u64>, StatusCode> {
    Ok(Json(state.service.get_history_count().await))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_scaling_event() {
        let service = ScalingHistoryService::new();

        let request = CreateScalingRecordRequest {
            pool_id: "pool-1".to_string(),
            policy_id: "policy-1".to_string(),
            trigger_id: None,
            scaling_direction: "scale_out".to_string(),
            previous_capacity: 5,
            new_capacity: 10,
            requested_capacity: 10,
            outcome: "success".to_string(),
            reason: "High CPU utilization".to_string(),
            metadata: None,
        };

        let result = service.record_scaling_event(request).await;
        assert!(result.is_ok());

        let entry = result.unwrap();
        assert_eq!(entry.pool_id, "pool-1");
        assert_eq!(entry.scaling_direction, ScalingDirection::ScaleOut);
    }

    #[tokio::test]
    async fn test_query_history() {
        let service = ScalingHistoryService::new();

        let request = CreateScalingRecordRequest {
            pool_id: "pool-1".to_string(),
            policy_id: "policy-1".to_string(),
            trigger_id: None,
            scaling_direction: "scale_out".to_string(),
            previous_capacity: 5,
            new_capacity: 10,
            requested_capacity: 10,
            outcome: "success".to_string(),
            reason: "High CPU utilization".to_string(),
            metadata: None,
        };

        service.record_scaling_event(request).await.unwrap();

        let query = QueryScalingHistoryRequest {
            pool_id: Some("pool-1".to_string()),
            policy_id: None,
            start_time: None,
            end_time: None,
            scaling_direction: None,
            outcome: None,
            limit: Some(10),
            offset: None,
        };

        let results = service.query_history(query).await;
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_get_scaling_statistics() {
        let service = ScalingHistoryService::new();

        let request = CreateScalingRecordRequest {
            pool_id: "pool-1".to_string(),
            policy_id: "policy-1".to_string(),
            trigger_id: None,
            scaling_direction: "scale_out".to_string(),
            previous_capacity: 5,
            new_capacity: 10,
            requested_capacity: 10,
            outcome: "success".to_string(),
            reason: "High CPU utilization".to_string(),
            metadata: None,
        };

        service.record_scaling_event(request).await.unwrap();

        let stats = service.get_scaling_statistics("pool-1").await.unwrap();
        assert_eq!(stats.total_scaling_events, 1);
        assert!(stats.success_rate > 0.0);
    }

    #[tokio::test]
    async fn test_get_history_by_id() {
        let service = ScalingHistoryService::new();

        let request = CreateScalingRecordRequest {
            pool_id: "pool-1".to_string(),
            policy_id: "policy-1".to_string(),
            trigger_id: None,
            scaling_direction: "scale_out".to_string(),
            previous_capacity: 5,
            new_capacity: 10,
            requested_capacity: 10,
            outcome: "success".to_string(),
            reason: "High CPU utilization".to_string(),
            metadata: None,
        };

        let entry = service.record_scaling_event(request).await.unwrap();

        let result = service.get_history_by_id(&entry.history_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().history_id, entry.history_id);
    }

    #[tokio::test]
    async fn test_invalid_scaling_direction() {
        let service = ScalingHistoryService::new();

        let request = CreateScalingRecordRequest {
            pool_id: "pool-1".to_string(),
            policy_id: "policy-1".to_string(),
            trigger_id: None,
            scaling_direction: "invalid_direction".to_string(),
            previous_capacity: 5,
            new_capacity: 10,
            requested_capacity: 10,
            outcome: "success".to_string(),
            reason: "High CPU utilization".to_string(),
            metadata: None,
        };

        let result = service.record_scaling_event(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid scaling direction"));
    }

    #[tokio::test]
    async fn test_get_scaling_aggregation() {
        let service = ScalingHistoryService::new();

        let request = CreateScalingRecordRequest {
            pool_id: "pool-1".to_string(),
            policy_id: "policy-1".to_string(),
            trigger_id: None,
            scaling_direction: "scale_out".to_string(),
            previous_capacity: 5,
            new_capacity: 10,
            requested_capacity: 10,
            outcome: "success".to_string(),
            reason: "High CPU utilization".to_string(),
            metadata: None,
        };

        service.record_scaling_event(request).await.unwrap();

        let start_time = Utc::now() - chrono::Duration::days(1);
        let end_time = Utc::now();

        let aggregation = service.get_scaling_aggregation("pool-1", start_time, end_time).await;
        assert!(aggregation.is_ok());

        let agg = aggregation.unwrap();
        assert_eq!(agg.pool_id, "pool-1");
        assert_eq!(agg.total_events, 1);
    }
}
