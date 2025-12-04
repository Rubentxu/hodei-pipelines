//! Pool Lifecycle Management API
//!
//! This module provides endpoints for managing the complete lifecycle of resource pools,
//! including initialization, activation, suspension, termination, and cleanup operations.

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Pool lifecycle state enum
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PoolLifecycleState {
    Uninitialized,
    Initializing,
    Active,
    Suspended,
    Terminating,
    Terminated,
    Error,
}

/// Pool lifecycle configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolLifecycleConfig {
    pub auto_start: bool,
    pub auto_cleanup: bool,
    pub health_check_interval: Duration,
    pub termination_timeout: Duration,
    pub graceful_shutdown_timeout: Duration,
    pub retry_max_attempts: u32,
}

/// Pool lifecycle event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleEvent {
    pub event_id: String,
    pub pool_id: String,
    pub state: PoolLifecycleState,
    pub timestamp: SystemTime,
    pub message: String,
}

/// Pool lifecycle status response
#[derive(Debug, Serialize, Deserialize)]
pub struct PoolLifecycleStatusResponse {
    pub pool_id: String,
    pub current_state: PoolLifecycleState,
    pub previous_state: Option<PoolLifecycleState>,
    pub last_transition: SystemTime,
    pub uptime: Duration,
    pub health_status: String,
    pub active_operations: Vec<String>,
}

/// Pool lifecycle statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct PoolLifecycleStats {
    pub pool_id: String,
    pub total_transitions: u64,
    pub time_in_states: HashMap<PoolLifecycleState, Duration>,
    pub errors_count: u64,
    pub last_error: Option<String>,
}

/// Initialize pool lifecycle request
#[derive(Debug, Deserialize)]
pub struct InitializePoolRequest {
    pub pool_id: String,
    pub config: Option<PoolLifecycleConfig>,
}

/// Activate pool request
#[derive(Debug, Deserialize)]
pub struct ActivatePoolRequest {
    pub pool_id: String,
    pub validate_resources: bool,
}

/// Suspend pool request
#[derive(Debug, Deserialize)]
pub struct SuspendPoolRequest {
    pub pool_id: String,
    pub force: bool,
    pub timeout_seconds: u64,
}

/// Terminate pool request
#[derive(Debug, Deserialize)]
pub struct TerminatePoolRequest {
    pub pool_id: String,
    pub graceful: bool,
    pub cleanup: bool,
}

/// Message response
#[derive(Debug, Serialize)]
pub struct LifecycleMessageResponse {
    pub message: String,
}

/// Pool Lifecycle Service
#[derive(Debug, Clone)]
pub struct PoolLifecycleService {
    /// Pool lifecycle states
    pool_states: Arc<RwLock<HashMap<String, PoolLifecycleState>>>,
    /// Pool configurations
    pool_configs: Arc<RwLock<HashMap<String, PoolLifecycleConfig>>>,
    /// Lifecycle event history
    lifecycle_events: Arc<RwLock<HashMap<String, Vec<LifecycleEvent>>>>,
    /// Last transition timestamps
    last_transitions: Arc<RwLock<HashMap<String, SystemTime>>>,
}

impl PoolLifecycleService {
    /// Create new Pool Lifecycle Service
    pub fn new() -> Self {
        info!("Initializing Pool Lifecycle Service");
        Self {
            pool_states: Arc::new(RwLock::new(HashMap::new())),
            pool_configs: Arc::new(RwLock::new(HashMap::new())),
            lifecycle_events: Arc::new(RwLock::new(HashMap::new())),
            last_transitions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize a pool
    pub async fn initialize_pool(
        &self,
        pool_id: &str,
        config: Option<PoolLifecycleConfig>,
    ) -> Result<(), String> {
        let mut states = self.pool_states.write().await;
        let mut configs = self.pool_configs.write().await;

        if states.contains_key(pool_id) {
            return Err(format!("Pool {} already exists", pool_id));
        }

        let final_config = config.unwrap_or_else(|| PoolLifecycleConfig {
            auto_start: false,
            auto_cleanup: true,
            health_check_interval: Duration::from_secs(30),
            termination_timeout: Duration::from_secs(300),
            graceful_shutdown_timeout: Duration::from_secs(60),
            retry_max_attempts: 3,
        });

        states.insert(pool_id.to_string(), PoolLifecycleState::Initializing);
        configs.insert(pool_id.to_string(), final_config.clone());

        // Record event
        self.record_event(
            pool_id,
            PoolLifecycleState::Initializing,
            "Pool initialized".to_string(),
        )
        .await;

        info!("Pool {} initialized", pool_id);

        Ok(())
    }

    /// Activate a pool
    pub async fn activate_pool(
        &self,
        pool_id: &str,
        _validate_resources: bool,
    ) -> Result<(), String> {
        let mut states = self.pool_states.write().await;

        let current_state = states
            .get(pool_id)
            .ok_or_else(|| format!("Pool {} not found", pool_id))?;

        if *current_state != PoolLifecycleState::Initializing
            && *current_state != PoolLifecycleState::Suspended
        {
            return Err(format!(
                "Cannot activate pool from state {:?}",
                current_state
            ));
        }

        states.insert(pool_id.to_string(), PoolLifecycleState::Active);
        self.record_event(
            pool_id,
            PoolLifecycleState::Active,
            "Pool activated".to_string(),
        )
        .await;

        info!("Pool {} activated", pool_id);

        Ok(())
    }

    /// Suspend a pool
    pub async fn suspend_pool(
        &self,
        pool_id: &str,
        force: bool,
        _timeout_seconds: u64,
    ) -> Result<(), String> {
        let mut states = self.pool_states.write().await;

        let current_state = states
            .get(pool_id)
            .ok_or_else(|| format!("Pool {} not found", pool_id))?;

        if *current_state != PoolLifecycleState::Active {
            return Err(format!(
                "Cannot suspend pool from state {:?}",
                current_state
            ));
        }

        if force {
            warn!("Force suspending pool {}", pool_id);
        }

        states.insert(pool_id.to_string(), PoolLifecycleState::Suspended);
        self.record_event(
            pool_id,
            PoolLifecycleState::Suspended,
            "Pool suspended".to_string(),
        )
        .await;

        info!("Pool {} suspended", pool_id);

        Ok(())
    }

    /// Terminate a pool
    pub async fn terminate_pool(
        &self,
        pool_id: &str,
        graceful: bool,
        cleanup: bool,
    ) -> Result<(), String> {
        let mut states = self.pool_states.write().await;

        let current_state = states
            .get(pool_id)
            .ok_or_else(|| format!("Pool {} not found", pool_id))?;

        if !graceful && *current_state == PoolLifecycleState::Active {
            return Err("Must suspend or deactivate before termination".to_string());
        }

        states.insert(pool_id.to_string(), PoolLifecycleState::Terminating);
        self.record_event(
            pool_id,
            PoolLifecycleState::Terminating,
            "Pool termination started".to_string(),
        )
        .await;

        // Simulate termination process
        tokio::time::sleep(Duration::from_millis(100)).await;

        if cleanup {
            // Clean up resources
            self.cleanup_pool(pool_id).await;
        }

        states.insert(pool_id.to_string(), PoolLifecycleState::Terminated);
        self.record_event(
            pool_id,
            PoolLifecycleState::Terminated,
            "Pool terminated".to_string(),
        )
        .await;

        info!(
            "Pool {} terminated (graceful: {}, cleanup: {})",
            pool_id, graceful, cleanup
        );

        Ok(())
    }

    /// Get pool lifecycle status
    pub async fn get_pool_status(
        &self,
        pool_id: &str,
    ) -> Result<PoolLifecycleStatusResponse, String> {
        let states = self.pool_states.read().await;
        let transitions = self.last_transitions.read().await;

        let current_state = states
            .get(pool_id)
            .ok_or_else(|| format!("Pool {} not found", pool_id))?
            .clone();
        let last_transition = transitions.get(pool_id).copied().unwrap_or(UNIX_EPOCH);

        let now = SystemTime::now();
        let uptime = if current_state == PoolLifecycleState::Active {
            now.duration_since(last_transition).unwrap_or_default()
        } else {
            Duration::from_secs(0)
        };

        let previous_state = None; // Could be tracked if needed
        let health_status = match current_state {
            PoolLifecycleState::Active => "healthy".to_string(),
            PoolLifecycleState::Suspended => "suspended".to_string(),
            PoolLifecycleState::Terminated => "terminated".to_string(),
            PoolLifecycleState::Error => "error".to_string(),
            _ => "unknown".to_string(),
        };

        Ok(PoolLifecycleStatusResponse {
            pool_id: pool_id.to_string(),
            current_state,
            previous_state,
            last_transition,
            uptime,
            health_status,
            active_operations: Vec::new(),
        })
    }

    /// List all pools
    pub async fn list_pools(&self) -> Vec<String> {
        let states = self.pool_states.read().await;
        states.keys().cloned().collect()
    }

    /// Get lifecycle statistics
    pub async fn get_pool_stats(&self, pool_id: &str) -> Result<PoolLifecycleStats, String> {
        let events = self.lifecycle_events.read().await;
        let pool_events = events
            .get(pool_id)
            .ok_or_else(|| format!("Pool {} not found", pool_id))?;

        let total_transitions = pool_events.len() as u64;
        let errors_count = pool_events
            .iter()
            .filter(|e| e.state == PoolLifecycleState::Error)
            .count() as u64;
        let last_error = pool_events
            .iter()
            .rev()
            .find(|e| e.state == PoolLifecycleState::Error)
            .map(|e| e.message.clone());

        // Calculate time in states (simplified)
        let mut time_in_states = HashMap::new();
        for event in pool_events {
            *time_in_states
                .entry(event.state.clone())
                .or_insert(Duration::from_secs(0)) += Duration::from_secs(1);
        }

        Ok(PoolLifecycleStats {
            pool_id: pool_id.to_string(),
            total_transitions,
            time_in_states,
            errors_count,
            last_error,
        })
    }

    /// Record lifecycle event
    async fn record_event(&self, pool_id: &str, state: PoolLifecycleState, message: String) {
        let mut events = self.lifecycle_events.write().await;
        let mut transitions = self.last_transitions.write().await;

        let event = LifecycleEvent {
            event_id: format!(
                "{}-{}",
                pool_id,
                SystemTime::now()
                    .elapsed()
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            ),
            pool_id: pool_id.to_string(),
            state: state.clone(),
            timestamp: SystemTime::now(),
            message: message.clone(),
        };

        events
            .entry(pool_id.to_string())
            .or_insert_with(Vec::new)
            .push(event);
        transitions.insert(pool_id.to_string(), SystemTime::now());

        info!(
            "Lifecycle event for pool {}: {:?} - {}",
            pool_id, state, message
        );
    }

    /// Clean up pool resources
    async fn cleanup_pool(&self, pool_id: &str) {
        let mut configs = self.pool_configs.write().await;
        configs.remove(pool_id);
        info!("Cleaned up resources for pool {}", pool_id);
    }
}

/// Application state for Pool Lifecycle
#[derive(Clone)]
pub struct PoolLifecycleAppState {
    pub service: Arc<PoolLifecycleService>,
}

/// Create router for Pool Lifecycle API
pub fn pool_lifecycle_routes() -> Router<PoolLifecycleAppState> {
    Router::new()
        .route("/pools", post(initialize_pool_handler))
        .route("/pools", get(list_pools_handler))
        .route("/pools/:pool_id", get(get_pool_status_handler))
        .route("/pools/:pool_id/activate", post(activate_pool_handler))
        .route("/pools/:pool_id/suspend", post(suspend_pool_handler))
        .route("/pools/:pool_id/terminate", post(terminate_pool_handler))
        .route("/pools/:pool_id/stats", get(get_pool_stats_handler))
        .route("/pools/:pool_id", delete(delete_pool_handler))
}

/// Initialize pool handler
async fn initialize_pool_handler(
    State(state): State<PoolLifecycleAppState>,
    Json(payload): Json<InitializePoolRequest>,
) -> Result<Json<LifecycleMessageResponse>, StatusCode> {
    match state
        .service
        .initialize_pool(&payload.pool_id, payload.config)
        .await
    {
        Ok(_) => Ok(Json(LifecycleMessageResponse {
            message: format!("Pool {} initialized successfully", payload.pool_id),
        })),
        Err(e) => {
            error!("Failed to initialize pool: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// List pools handler
async fn list_pools_handler(
    State(state): State<PoolLifecycleAppState>,
) -> Result<Json<Vec<String>>, StatusCode> {
    Ok(Json(state.service.list_pools().await))
}

/// Get pool status handler
async fn get_pool_status_handler(
    State(state): State<PoolLifecycleAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
) -> Result<Json<PoolLifecycleStatusResponse>, StatusCode> {
    match state.service.get_pool_status(&pool_id).await {
        Ok(status) => Ok(Json(status)),
        Err(e) => {
            error!("Failed to get pool status: {}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Activate pool handler
async fn activate_pool_handler(
    State(state): State<PoolLifecycleAppState>,
    Json(payload): Json<ActivatePoolRequest>,
) -> Result<Json<LifecycleMessageResponse>, StatusCode> {
    match state
        .service
        .activate_pool(&payload.pool_id, payload.validate_resources)
        .await
    {
        Ok(_) => Ok(Json(LifecycleMessageResponse {
            message: format!("Pool {} activated successfully", payload.pool_id),
        })),
        Err(e) => {
            error!("Failed to activate pool: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Suspend pool handler
async fn suspend_pool_handler(
    State(state): State<PoolLifecycleAppState>,
    Json(payload): Json<SuspendPoolRequest>,
) -> Result<Json<LifecycleMessageResponse>, StatusCode> {
    match state
        .service
        .suspend_pool(&payload.pool_id, payload.force, payload.timeout_seconds)
        .await
    {
        Ok(_) => Ok(Json(LifecycleMessageResponse {
            message: format!("Pool {} suspended successfully", payload.pool_id),
        })),
        Err(e) => {
            error!("Failed to suspend pool: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Terminate pool handler
async fn terminate_pool_handler(
    State(state): State<PoolLifecycleAppState>,
    Json(payload): Json<TerminatePoolRequest>,
) -> Result<Json<LifecycleMessageResponse>, StatusCode> {
    match state
        .service
        .terminate_pool(&payload.pool_id, payload.graceful, payload.cleanup)
        .await
    {
        Ok(_) => Ok(Json(LifecycleMessageResponse {
            message: format!("Pool {} terminated successfully", payload.pool_id),
        })),
        Err(e) => {
            error!("Failed to terminate pool: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Get pool statistics handler
async fn get_pool_stats_handler(
    State(state): State<PoolLifecycleAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
) -> Result<Json<PoolLifecycleStats>, StatusCode> {
    match state.service.get_pool_stats(&pool_id).await {
        Ok(stats) => Ok(Json(stats)),
        Err(e) => {
            error!("Failed to get pool stats: {}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Delete pool handler
async fn delete_pool_handler(
    State(state): State<PoolLifecycleAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
) -> Result<Json<LifecycleMessageResponse>, StatusCode> {
    // Verify pool exists
    let mut states = state.service.pool_states.write().await;

    if !states.contains_key(&pool_id) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Remove pool
    states.remove(&pool_id);

    // Clean up resources
    let mut configs = state.service.pool_configs.write().await;
    configs.remove(&pool_id);

    info!("Pool {} deleted", pool_id);

    Ok(Json(LifecycleMessageResponse {
        message: format!("Pool {} deleted successfully", pool_id),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_lifecycle_service_initialization() {
        let service = PoolLifecycleService::new();

        let result = service.initialize_pool("pool-1", None).await;
        assert!(result.is_ok());

        let pools = service.list_pools().await;
        assert_eq!(pools.len(), 1);
        assert_eq!(pools[0], "pool-1");
    }

    #[tokio::test]
    async fn test_pool_activation() {
        let service = PoolLifecycleService::new();

        service.initialize_pool("pool-1", None).await.unwrap();

        let result = service.activate_pool("pool-1", false).await;
        assert!(result.is_ok());

        let status = service.get_pool_status("pool-1").await.unwrap();
        assert_eq!(status.current_state, PoolLifecycleState::Active);
    }

    #[tokio::test]
    async fn test_pool_suspension() {
        let service = PoolLifecycleService::new();

        service.initialize_pool("pool-1", None).await.unwrap();
        service.activate_pool("pool-1", false).await.unwrap();

        let result = service.suspend_pool("pool-1", false, 30).await;
        assert!(result.is_ok());

        let status = service.get_pool_status("pool-1").await.unwrap();
        assert_eq!(status.current_state, PoolLifecycleState::Suspended);
    }

    #[tokio::test]
    async fn test_pool_termination() {
        let service = PoolLifecycleService::new();

        service.initialize_pool("pool-1", None).await.unwrap();
        service.activate_pool("pool-1", false).await.unwrap();

        let result = service.terminate_pool("pool-1", true, true).await;
        assert!(result.is_ok());

        let status = service.get_pool_status("pool-1").await.unwrap();
        assert_eq!(status.current_state, PoolLifecycleState::Terminated);
    }

    #[tokio::test]
    async fn test_pool_statistics() {
        let service = PoolLifecycleService::new();

        service.initialize_pool("pool-1", None).await.unwrap();
        service.activate_pool("pool-1", false).await.unwrap();
        service.suspend_pool("pool-1", false, 30).await.unwrap();

        let stats = service.get_pool_stats("pool-1").await.unwrap();
        assert!(stats.total_transitions >= 3);
    }

    #[tokio::test]
    async fn test_duplicate_pool_initialization() {
        let service = PoolLifecycleService::new();

        service.initialize_pool("pool-1", None).await.unwrap();

        let result = service.initialize_pool("pool-1", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[tokio::test]
    async fn test_activate_nonexistent_pool() {
        let service = PoolLifecycleService::new();

        let result = service.activate_pool("pool-1", false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_get_status_nonexistent_pool() {
        let service = PoolLifecycleService::new();

        let result = service.get_pool_status("pool-1").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}
