//! Cooldown Management API
//!
//! This module provides endpoints for managing cooldown periods during scaling operations
//! to prevent thrashing and ensure stable resource allocation.

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
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

/// Cooldown types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CooldownType {
    ScaleIn,
    ScaleOut,
    Global,
    Policy,
}

/// Cooldown status
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CooldownStatus {
    Active,
    Waiting,
    Expired,
}

/// Cooldown configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooldownConfig {
    pub cooldown_type: CooldownType,
    pub duration: Duration,
    pub scope: String,             // pool, policy, global
    pub scope_id: String,          // pool_id or policy_id (empty for global)
    pub max_cooldowns: u32,        // Maximum number of cooldowns
    pub cooldown_strategy: String, // strict, rolling, sliding
}

/// Cooldown period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooldownPeriod {
    pub cooldown_id: String,
    pub pool_id: String,
    pub policy_id: Option<String>,
    pub cooldown_type: CooldownType,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status: CooldownStatus,
    pub triggered_by: String,
    pub remaining_time: Option<Duration>,
}

/// Create cooldown request
#[derive(Debug, Deserialize)]
pub struct CreateCooldownRequest {
    pub pool_id: String,
    pub policy_id: Option<String>,
    pub cooldown_type: String, // scale_in, scale_out, global, policy
    pub duration_seconds: u64,
    pub triggered_by: String,
}

/// Cooldown status response
#[derive(Debug, Serialize, Deserialize)]
pub struct CooldownStatusResponse {
    pub cooldown_id: String,
    pub pool_id: String,
    pub policy_id: Option<String>,
    pub cooldown_type: String,
    pub status: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub remaining_time_seconds: Option<u64>,
    pub triggered_by: String,
}

/// Message response
#[derive(Debug, Serialize)]
pub struct CooldownMessageResponse {
    pub message: String,
}

/// Cooldown statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct CooldownStats {
    pub pool_id: String,
    pub total_cooldowns: u64,
    pub active_cooldowns: u64,
    pub average_cooldown_duration: Duration,
    pub cooldowns_prevented: u64,
    pub last_cooldown: Option<DateTime<Utc>>,
}

/// Cooldown history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooldownHistoryEntry {
    pub cooldown_id: String,
    pub pool_id: String,
    pub cooldown_type: CooldownType,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration: Duration,
    pub triggered_by: String,
    pub prevented_scaling: bool,
}

/// Cooldowns Service
#[derive(Debug, Clone)]
pub struct CooldownsService {
    /// Active cooldowns
    cooldowns: Arc<RwLock<HashMap<String, CooldownPeriod>>>,
    /// Cooldown history
    cooldown_history: Arc<RwLock<Vec<CooldownHistoryEntry>>>,
    /// Cooldown configurations
    cooldown_configs: Arc<RwLock<HashMap<String, CooldownConfig>>>,
}

impl CooldownsService {
    /// Create new Cooldowns Service
    pub fn new() -> Self {
        info!("Initializing Cooldowns Service");
        Self {
            cooldowns: Arc::new(RwLock::new(HashMap::new())),
            cooldown_history: Arc::new(RwLock::new(Vec::new())),
            cooldown_configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create cooldown
    pub async fn create_cooldown(
        &self,
        request: CreateCooldownRequest,
    ) -> Result<CooldownPeriod, String> {
        let cooldown_type = match request.cooldown_type.to_lowercase().as_str() {
            "scale_in" => CooldownType::ScaleIn,
            "scale_out" => CooldownType::ScaleOut,
            "global" => CooldownType::Global,
            "policy" => CooldownType::Policy,
            _ => return Err("Invalid cooldown type".to_string()),
        };

        let start_time = Utc::now();
        let duration = Duration::from_secs(request.duration_seconds);
        let end_time = start_time + chrono::Duration::from_std(duration).unwrap();

        let cooldown = CooldownPeriod {
            cooldown_id: uuid::Uuid::new_v4().to_string(),
            pool_id: request.pool_id.clone(),
            policy_id: request.policy_id,
            cooldown_type: cooldown_type.clone(),
            start_time,
            end_time,
            status: CooldownStatus::Active,
            triggered_by: request.triggered_by,
            remaining_time: Some(duration),
        };

        let mut cooldowns = self.cooldowns.write().await;
        cooldowns.insert(cooldown.cooldown_id.clone(), cooldown.clone());

        // Add to history
        let history_entry = CooldownHistoryEntry {
            cooldown_id: cooldown.cooldown_id.clone(),
            pool_id: cooldown.pool_id.clone(),
            cooldown_type: cooldown_type.clone(),
            start_time: cooldown.start_time,
            end_time: cooldown.end_time,
            duration,
            triggered_by: cooldown.triggered_by.clone(),
            prevented_scaling: false, // Will be updated if scaling is prevented
        };

        let mut history = self.cooldown_history.write().await;
        history.push(history_entry);

        info!(
            "Created cooldown: {} for pool {} (type: {:?}, duration: {:?})",
            cooldown.cooldown_id, cooldown.pool_id, cooldown_type, duration
        );

        Ok(cooldown)
    }

    /// Get cooldown
    pub async fn get_cooldown(&self, cooldown_id: &str) -> Result<CooldownPeriod, String> {
        let cooldowns = self.cooldowns.read().await;
        let mut cooldown = cooldowns
            .get(cooldown_id)
            .cloned()
            .ok_or_else(|| "Cooldown not found".to_string())?;

        // Update remaining time if cooldown is active
        if cooldown.status == CooldownStatus::Active {
            let now = Utc::now();
            if let Ok(remaining) = cooldown.end_time.signed_duration_since(now).to_std() {
                cooldown.remaining_time = Some(remaining);
                if remaining.as_secs() == 0 {
                    cooldown.status = CooldownStatus::Expired;
                }
            }
        }

        Ok(cooldown)
    }

    /// List cooldowns by pool
    pub async fn list_cooldowns_by_pool(&self, pool_id: &str) -> Vec<CooldownPeriod> {
        let cooldowns = self.cooldowns.read().await;

        let mut cooldown_list: Vec<CooldownPeriod> = cooldowns
            .values()
            .filter(|c| c.pool_id == pool_id)
            .cloned()
            .collect();

        // Update remaining times
        for cooldown in &mut cooldown_list {
            if cooldown.status == CooldownStatus::Active {
                let now = Utc::now();
                if let Ok(remaining) = cooldown.end_time.signed_duration_since(now).to_std() {
                    cooldown.remaining_time = Some(remaining);
                    if remaining.as_secs() == 0 {
                        cooldown.status = CooldownStatus::Expired;
                    }
                }
            }
        }

        cooldown_list
    }

    /// List active cooldowns
    pub async fn list_active_cooldowns(&self) -> Vec<CooldownPeriod> {
        let cooldowns = self.cooldowns.read().await;
        let now = Utc::now();

        let mut active_list: Vec<CooldownPeriod> = cooldowns
            .values()
            .filter(|c| c.status == CooldownStatus::Active && c.end_time > now)
            .cloned()
            .collect();

        // Update remaining times
        for cooldown in &mut active_list {
            if let Ok(remaining) = cooldown.end_time.signed_duration_since(now).to_std() {
                cooldown.remaining_time = Some(remaining);
            }
        }

        active_list
    }

    /// Delete cooldown
    pub async fn delete_cooldown(&self, cooldown_id: &str) -> Result<(), String> {
        let mut cooldowns = self.cooldowns.write().await;

        if !cooldowns.contains_key(cooldown_id) {
            return Err("Cooldown not found".to_string());
        }

        cooldowns.remove(cooldown_id);

        info!("Deleted cooldown: {}", cooldown_id);

        Ok(())
    }

    /// Check if cooldown is active
    pub async fn is_cooldown_active(&self, pool_id: &str, cooldown_type: &str) -> bool {
        let cooldowns = self.cooldowns.read().await;
        let now = Utc::now();

        let cooldown_type_enum = match cooldown_type.to_lowercase().as_str() {
            "scale_in" => CooldownType::ScaleIn,
            "scale_out" => CooldownType::ScaleOut,
            "global" => CooldownType::Global,
            "policy" => CooldownType::Policy,
            _ => return false,
        };

        cooldowns.values().any(|c| {
            c.pool_id == pool_id
                && c.cooldown_type == cooldown_type_enum
                && c.status == CooldownStatus::Active
                && c.end_time > now
        })
    }

    /// Get cooldown statistics for pool
    pub async fn get_cooldown_stats(&self, pool_id: &str) -> Result<CooldownStats, String> {
        let history = self.cooldown_history.read().await;
        let cooldowns = self.cooldowns.read().await;

        let pool_history: Vec<_> = history.iter().filter(|h| h.pool_id == pool_id).collect();

        let total_cooldowns = pool_history.len() as u64;
        let active_cooldowns = cooldowns
            .values()
            .filter(|c| c.pool_id == pool_id && c.status == CooldownStatus::Active)
            .count() as u64;

        let average_duration = if pool_history.is_empty() {
            Duration::from_secs(0)
        } else {
            let total_duration: Duration = pool_history
                .iter()
                .map(|h| h.duration)
                .fold(Duration::from_secs(0), |acc, d| acc + d);
            total_duration / pool_history.len() as u32
        };

        let last_cooldown = pool_history.iter().map(|h| h.start_time).max();

        Ok(CooldownStats {
            pool_id: pool_id.to_string(),
            total_cooldowns,
            active_cooldowns,
            average_cooldown_duration: average_duration,
            cooldowns_prevented: 0, // Mock value
            last_cooldown,
        })
    }

    /// Record cooldown history entry
    pub async fn record_cooldown_end(&self, cooldown_id: &str, prevented_scaling: bool) {
        if let Some(_cooldown) = self.cooldowns.read().await.get(cooldown_id) {
            let mut history = self.cooldown_history.write().await;

            // Find and update the history entry
            for entry in history.iter_mut() {
                if entry.cooldown_id == cooldown_id {
                    entry.prevented_scaling = prevented_scaling;
                    break;
                }
            }
        }
    }

    /// Clean up expired cooldowns
    pub async fn cleanup_expired_cooldowns(&self) {
        let mut cooldowns = self.cooldowns.write().await;
        let now = Utc::now();

        let mut expired_ids = Vec::new();

        for (id, cooldown) in cooldowns.iter() {
            if cooldown.status == CooldownStatus::Active && cooldown.end_time <= now {
                expired_ids.push(id.clone());
            }
        }

        for id in expired_ids {
            if let Some(mut cooldown) = cooldowns.remove(&id) {
                cooldown.status = CooldownStatus::Expired;
                cooldown.remaining_time = Some(Duration::from_secs(0));

                info!("Cleaned up expired cooldown: {}", id);
            }
        }
    }
}

/// Application state for Cooldowns
#[derive(Clone)]
pub struct CooldownsAppState {
    pub service: Arc<CooldownsService>,
}

/// Create router for Cooldowns API
pub fn cooldowns_routes() -> Router<CooldownsAppState> {
    Router::new()
        .route("/cooldowns", post(create_cooldown_handler))
        .route("/cooldowns/active", get(list_active_cooldowns_handler))
        .route(
            "/cooldowns/pool/:pool_id",
            get(list_cooldowns_by_pool_handler),
        )
        .route("/cooldowns/:cooldown_id", get(get_cooldown_handler))
        .route("/cooldowns/:cooldown_id", delete(delete_cooldown_handler))
        .route(
            "/cooldowns/pool/:pool_id/stats",
            get(get_cooldown_stats_handler),
        )
        .route(
            "/cooldowns/check/:pool_id/:cooldown_type",
            get(check_cooldown_handler),
        )
}

/// Create cooldown handler
async fn create_cooldown_handler(
    State(state): State<CooldownsAppState>,
    Json(payload): Json<CreateCooldownRequest>,
) -> Result<Json<CooldownMessageResponse>, StatusCode> {
    match state.service.create_cooldown(payload).await {
        Ok(cooldown) => Ok(Json(CooldownMessageResponse {
            message: format!(
                "Cooldown created: {} (expires at {})",
                cooldown.cooldown_id, cooldown.end_time
            ),
        })),
        Err(e) => {
            error!("Failed to create cooldown: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Get cooldown handler
async fn get_cooldown_handler(
    State(state): State<CooldownsAppState>,
    axum::extract::Path(cooldown_id): axum::extract::Path<String>,
) -> Result<Json<CooldownStatusResponse>, StatusCode> {
    match state.service.get_cooldown(&cooldown_id).await {
        Ok(cooldown) => {
            let remaining_seconds = cooldown.remaining_time.map(|d| d.as_secs());

            Ok(Json(CooldownStatusResponse {
                cooldown_id: cooldown.cooldown_id,
                pool_id: cooldown.pool_id,
                policy_id: cooldown.policy_id,
                cooldown_type: format!("{:?}", cooldown.cooldown_type),
                status: format!("{:?}", cooldown.status),
                start_time: cooldown.start_time,
                end_time: cooldown.end_time,
                remaining_time_seconds: remaining_seconds,
                triggered_by: cooldown.triggered_by,
            }))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// List active cooldowns handler
async fn list_active_cooldowns_handler(
    State(state): State<CooldownsAppState>,
) -> Result<Json<Vec<CooldownStatusResponse>>, StatusCode> {
    let cooldowns = state.service.list_active_cooldowns().await;

    let responses: Vec<CooldownStatusResponse> = cooldowns
        .into_iter()
        .map(|cooldown| {
            let remaining_seconds = cooldown.remaining_time.map(|d| d.as_secs());

            CooldownStatusResponse {
                cooldown_id: cooldown.cooldown_id,
                pool_id: cooldown.pool_id,
                policy_id: cooldown.policy_id,
                cooldown_type: format!("{:?}", cooldown.cooldown_type),
                status: format!("{:?}", cooldown.status),
                start_time: cooldown.start_time,
                end_time: cooldown.end_time,
                remaining_time_seconds: remaining_seconds,
                triggered_by: cooldown.triggered_by,
            }
        })
        .collect();

    Ok(Json(responses))
}

/// List cooldowns by pool handler
async fn list_cooldowns_by_pool_handler(
    State(state): State<CooldownsAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
) -> Result<Json<Vec<CooldownStatusResponse>>, StatusCode> {
    let cooldowns = state.service.list_cooldowns_by_pool(&pool_id).await;

    let responses: Vec<CooldownStatusResponse> = cooldowns
        .into_iter()
        .map(|cooldown| {
            let remaining_seconds = cooldown.remaining_time.map(|d| d.as_secs());

            CooldownStatusResponse {
                cooldown_id: cooldown.cooldown_id,
                pool_id: cooldown.pool_id,
                policy_id: cooldown.policy_id,
                cooldown_type: format!("{:?}", cooldown.cooldown_type),
                status: format!("{:?}", cooldown.status),
                start_time: cooldown.start_time,
                end_time: cooldown.end_time,
                remaining_time_seconds: remaining_seconds,
                triggered_by: cooldown.triggered_by,
            }
        })
        .collect();

    Ok(Json(responses))
}

/// Delete cooldown handler
async fn delete_cooldown_handler(
    State(state): State<CooldownsAppState>,
    axum::extract::Path(cooldown_id): axum::extract::Path<String>,
) -> Result<Json<CooldownMessageResponse>, StatusCode> {
    match state.service.delete_cooldown(&cooldown_id).await {
        Ok(_) => Ok(Json(CooldownMessageResponse {
            message: format!("Cooldown {} deleted successfully", cooldown_id),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Get cooldown statistics handler
async fn get_cooldown_stats_handler(
    State(state): State<CooldownsAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
) -> Result<Json<CooldownStats>, StatusCode> {
    match state.service.get_cooldown_stats(&pool_id).await {
        Ok(stats) => Ok(Json(stats)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Check cooldown handler
async fn check_cooldown_handler(
    State(state): State<CooldownsAppState>,
    axum::extract::Path((pool_id, cooldown_type)): axum::extract::Path<(String, String)>,
) -> Result<Json<CooldownMessageResponse>, StatusCode> {
    let is_active = state
        .service
        .is_cooldown_active(&pool_id, &cooldown_type)
        .await;

    Ok(Json(CooldownMessageResponse {
        message: if is_active {
            format!(
                "Cooldown is ACTIVE for pool {} (type: {})",
                pool_id, cooldown_type
            )
        } else {
            format!(
                "No active cooldown for pool {} (type: {})",
                pool_id, cooldown_type
            )
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_cooldown() {
        let service = CooldownsService::new();

        let request = CreateCooldownRequest {
            pool_id: "pool-1".to_string(),
            policy_id: None,
            cooldown_type: "scale_in".to_string(),
            duration_seconds: 300,
            triggered_by: "policy-1".to_string(),
        };

        let result = service.create_cooldown(request).await;
        assert!(result.is_ok());

        let cooldown = result.unwrap();
        assert_eq!(cooldown.pool_id, "pool-1");
        assert_eq!(cooldown.cooldown_type, CooldownType::ScaleIn);
    }

    #[tokio::test]
    async fn test_list_cooldowns_by_pool() {
        let service = CooldownsService::new();

        let request = CreateCooldownRequest {
            pool_id: "pool-1".to_string(),
            policy_id: None,
            cooldown_type: "scale_in".to_string(),
            duration_seconds: 300,
            triggered_by: "policy-1".to_string(),
        };

        service.create_cooldown(request).await.unwrap();

        let cooldowns = service.list_cooldowns_by_pool("pool-1").await;
        assert_eq!(cooldowns.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_cooldown() {
        let service = CooldownsService::new();

        let request = CreateCooldownRequest {
            pool_id: "pool-1".to_string(),
            policy_id: None,
            cooldown_type: "scale_in".to_string(),
            duration_seconds: 300,
            triggered_by: "policy-1".to_string(),
        };

        let cooldown = service.create_cooldown(request).await.unwrap();

        let result = service.delete_cooldown(&cooldown.cooldown_id).await;
        assert!(result.is_ok());

        let cooldowns = service.list_cooldowns_by_pool("pool-1").await;
        assert_eq!(cooldowns.len(), 0);
    }

    #[tokio::test]
    async fn test_is_cooldown_active() {
        let service = CooldownsService::new();

        let request = CreateCooldownRequest {
            pool_id: "pool-1".to_string(),
            policy_id: None,
            cooldown_type: "scale_in".to_string(),
            duration_seconds: 300,
            triggered_by: "policy-1".to_string(),
        };

        service.create_cooldown(request).await.unwrap();

        let is_active = service.is_cooldown_active("pool-1", "scale_in").await;
        assert!(is_active);

        let is_active = service.is_cooldown_active("pool-1", "scale_out").await;
        assert!(!is_active);
    }

    #[tokio::test]
    async fn test_get_cooldown_stats() {
        let service = CooldownsService::new();

        let request = CreateCooldownRequest {
            pool_id: "pool-1".to_string(),
            policy_id: None,
            cooldown_type: "scale_in".to_string(),
            duration_seconds: 300,
            triggered_by: "policy-1".to_string(),
        };

        service.create_cooldown(request).await.unwrap();

        let stats = service.get_cooldown_stats("pool-1").await.unwrap();
        assert_eq!(stats.total_cooldowns, 1);
        assert!(stats.active_cooldowns >= 0);
    }

    #[tokio::test]
    async fn test_invalid_cooldown_type() {
        let service = CooldownsService::new();

        let request = CreateCooldownRequest {
            pool_id: "pool-1".to_string(),
            policy_id: None,
            cooldown_type: "invalid_type".to_string(),
            duration_seconds: 300,
            triggered_by: "policy-1".to_string(),
        };

        let result = service.create_cooldown(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid cooldown type"));
    }
}
