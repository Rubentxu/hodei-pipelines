//! Cooldown Management Module
//!
//! This module provides comprehensive cooldown management to prevent scaling
//! oscillation while allowing overrides for critical situations.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::auto_scaling_engine::ScaleDirection;
use hodei_core::{DomainError, Result};

/// Cooldown type for different scaling scenarios
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CooldownType {
    ScaleUp,
    ScaleDown,
    Any,
}

/// Cooldown configuration
#[derive(Debug, Clone)]
pub struct CooldownConfig {
    /// Duration for scale up cooldown
    pub scale_up_duration: Duration,
    /// Duration for scale down cooldown
    pub scale_down_duration: Duration,
    /// Default cooldown duration for any direction
    pub default_duration: Duration,
    /// Enable cooldown tracking
    pub enabled: bool,
    /// Maximum number of cooldown events to track per pool
    pub max_history: usize,
}

/// Override reason for bypassing cooldown
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverrideReason {
    Critical,
    Emergency,
    Manual,
    SLAViolation,
    Custom(String),
}

/// Override configuration
#[derive(Debug, Clone)]
pub struct OverrideConfig {
    pub enabled: bool,
    pub allowed_reasons: Vec<OverrideReason>,
    pub max_overrides_per_hour: u32,
}

/// Cooldown event record
#[derive(Debug, Clone)]
pub struct CooldownEvent {
    pub pool_id: String,
    pub direction: ScaleDirection,
    pub cooldown_type: CooldownType,
    pub timestamp: DateTime<Utc>,
    pub duration: Duration,
    pub override_reason: Option<OverrideReason>,
}

/// Cooldown statistics
#[derive(Debug, Clone)]
pub struct CooldownStats {
    pub total_cooldown_events: u64,
    pub scale_up_events: u64,
    pub scale_down_events: u64,
    pub override_events: u64,
    pub currently_in_cooldown: bool,
    pub remaining_cooldown: Option<Duration>,
}

/// Advanced cooldown manager
#[derive(Debug, Clone)]
pub struct AdvancedCooldownManager {
    config: CooldownConfig,
    override_config: OverrideConfig,
    cooldown_tracking: Arc<RwLock<HashMap<String, PoolCooldownState>>>,
    event_history: Arc<RwLock<Vec<CooldownEvent>>>,
}

/// Per-pool cooldown state
#[derive(Debug, Clone)]
struct PoolCooldownState {
    pub pool_id: String,
    pub last_scale_up: Option<DateTime<Utc>>,
    pub last_scale_down: Option<DateTime<Utc>>,
    pub last_any: Option<DateTime<Utc>>,
    pub override_expiry: Option<DateTime<Utc>>,
    pub override_reason: Option<OverrideReason>,
    pub event_count: HashMap<CooldownType, u32>,
}

/// Cooldown error types
#[derive(Debug, thiserror::Error)]
pub enum CooldownError {
    #[error("Cooldown violation: {0}")]
    CooldownViolation(String),
    #[error("Override not allowed: {0}")]
    OverrideNotAllowed(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

impl AdvancedCooldownManager {
    /// Create a new cooldown manager
    pub fn new(config: CooldownConfig, override_config: OverrideConfig) -> Self {
        Self {
            config,
            override_config,
            cooldown_tracking: Arc::new(RwLock::new(HashMap::new())),
            event_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check if a pool is currently in cooldown for a specific direction
    pub async fn is_in_cooldown(
        &self,
        pool_id: &str,
        direction: &ScaleDirection,
        cooldown_type: CooldownType,
    ) -> bool {
        let tracking = self.cooldown_tracking.read().await;
        if let Some(state) = tracking.get(pool_id) {
            self.check_cooldown_state(state, &direction, cooldown_type)
        } else {
            false
        }
    }

    /// Get remaining cooldown time
    pub async fn get_remaining_cooldown(
        &self,
        pool_id: &str,
        direction: &ScaleDirection,
    ) -> Option<Duration> {
        let tracking = self.cooldown_tracking.read().await;
        if let Some(state) = tracking.get(pool_id) {
            self.calculate_remaining_cooldown(state, &direction)
        } else {
            None
        }
    }

    /// Record a scaling event and start cooldown
    pub async fn record_scaling(
        &self,
        pool_id: &str,
        direction: &ScaleDirection,
        cooldown_type: CooldownType,
    ) -> Result<()> {
        // Check if in cooldown (unless explicitly overridden)
        if self.is_in_cooldown(pool_id, direction, cooldown_type).await {
            return Err(CooldownError::CooldownViolation(format!(
                "Pool {} is in cooldown for {:?} direction",
                pool_id, direction
            ))
            .into());
        }

        let now = Utc::now();
        let duration = self.get_cooldown_duration(&direction);

        let mut tracking = self.cooldown_tracking.write().await;
        let state = tracking
            .entry(pool_id.to_string())
            .or_insert_with(|| PoolCooldownState::new(pool_id.to_string()));

        // Update last scaling timestamp
        match &direction {
            ScaleDirection::ScaleUp => state.last_scale_up = Some(now),
            ScaleDirection::ScaleDown => state.last_scale_down = Some(now),
        }

        if matches!(cooldown_type, CooldownType::Any) {
            state.last_any = Some(now);
        }

        // Update event count
        *state.event_count.entry(cooldown_type).or_insert(0) += 1;

        // Record event
        let event = CooldownEvent {
            pool_id: pool_id.to_string(),
            direction: direction.clone(),
            cooldown_type,
            timestamp: now,
            duration,
            override_reason: None,
        };

        let mut history = self.event_history.write().await;
        history.push(event);

        // Keep history within limits (keep last 1000 events)
        let overflow = history.len().saturating_sub(1000);
        if overflow > 0 {
            history.drain(0..overflow);
        }

        info!(
            "Recorded scaling event for pool {}: {:?}",
            pool_id, direction
        );
        Ok(())
    }

    /// Override cooldown for critical situations
    pub async fn override_cooldown(
        &self,
        pool_id: &str,
        direction: &ScaleDirection,
        reason: OverrideReason,
    ) -> Result<()> {
        // Check if override is allowed
        if !self.override_config.enabled {
            return Err(
                CooldownError::OverrideNotAllowed("Overrides are disabled".to_string()).into(),
            );
        }

        if !self.override_config.allowed_reasons.contains(&reason) {
            return Err(CooldownError::OverrideNotAllowed(format!(
                "Override reason {:?} is not allowed",
                reason
            ))
            .into());
        }

        // Check override rate limit
        let recent_overrides = self
            .count_recent_overrides(pool_id, Duration::from_secs(3600))
            .await;
        if recent_overrides >= self.override_config.max_overrides_per_hour {
            return Err(CooldownError::OverrideNotAllowed(
                "Maximum override rate exceeded".to_string(),
            )
            .into());
        }

        let now = Utc::now();
        let expiry = now + chrono::Duration::minutes(5); // Override expires after 5 minutes

        let mut tracking = self.cooldown_tracking.write().await;
        let state = tracking
            .entry(pool_id.to_string())
            .or_insert_with(|| PoolCooldownState::new(pool_id.to_string()));

        state.override_expiry = Some(expiry);
        state.override_reason = Some(reason.clone());

        // Record override event
        let event = CooldownEvent {
            pool_id: pool_id.to_string(),
            direction: direction.clone(),
            cooldown_type: CooldownType::Any,
            timestamp: now,
            duration: Duration::from_secs(300), // 5 minutes
            override_reason: Some(reason.clone()),
        };

        let mut history = self.event_history.write().await;
        history.push(event);

        warn!("Cooldown overridden for pool {}: {:?}", pool_id, reason);
        Ok(())
    }

    /// Clear cooldown for a pool
    pub async fn clear_cooldown(&self, pool_id: &str) {
        let mut tracking = self.cooldown_tracking.write().await;
        tracking.remove(pool_id);
        info!("Cleared cooldown for pool {}", pool_id);
    }

    /// Get cooldown statistics for a pool
    pub async fn get_stats(&self, pool_id: &str) -> Option<CooldownStats> {
        let tracking = self.cooldown_tracking.read().await;
        let state = tracking.get(pool_id)?;

        let history = self.event_history.read().await;
        let pool_events: Vec<_> = history.iter().filter(|e| e.pool_id == pool_id).collect();

        let total = pool_events.len() as u64;
        let scale_up = pool_events
            .iter()
            .filter(|e| e.direction == ScaleDirection::ScaleUp)
            .count() as u64;
        let scale_down = pool_events
            .iter()
            .filter(|e| e.direction == ScaleDirection::ScaleDown)
            .count() as u64;
        let overrides = pool_events
            .iter()
            .filter(|e| e.override_reason.is_some())
            .count() as u64;

        let currently_in_cooldown = self
            .is_in_cooldown(pool_id, &ScaleDirection::ScaleUp, CooldownType::Any)
            .await
            || self
                .is_in_cooldown(pool_id, &ScaleDirection::ScaleDown, CooldownType::Any)
                .await;

        let remaining_cooldown_up = self
            .get_remaining_cooldown(pool_id, &ScaleDirection::ScaleUp)
            .await;
        let remaining_cooldown = if remaining_cooldown_up.is_some() {
            remaining_cooldown_up
        } else {
            self.get_remaining_cooldown(pool_id, &ScaleDirection::ScaleDown)
                .await
        };

        Some(CooldownStats {
            total_cooldown_events: total,
            scale_up_events: scale_up,
            scale_down_events: scale_down,
            override_events: overrides,
            currently_in_cooldown,
            remaining_cooldown,
        })
    }

    /// Get event history for a pool
    pub async fn get_history(&self, pool_id: &str) -> Vec<CooldownEvent> {
        let history = self.event_history.read().await;
        history
            .iter()
            .filter(|e| e.pool_id == pool_id)
            .cloned()
            .collect()
    }

    /// Cleanup old events
    pub async fn cleanup(&self, retention_period: Duration) -> Result<u64> {
        let cutoff = Utc::now()
            - chrono::Duration::from_std(retention_period)
                .map_err(|e| CooldownError::InvalidConfiguration(e.to_string()))?;
        let mut history = self.event_history.write().await;
        let before_len = history.len();
        history.retain(|e| e.timestamp > cutoff);
        let removed = before_len - history.len();
        Ok(removed as u64)
    }

    /// Internal: Check cooldown state
    fn check_cooldown_state(
        &self,
        state: &PoolCooldownState,
        direction: &ScaleDirection,
        cooldown_type: CooldownType,
    ) -> bool {
        // Check if override is active
        if let Some(expiry) = state.override_expiry {
            if Utc::now() < expiry {
                return false;
            }
        }

        // Get last scaling time based on type
        let last_time = match cooldown_type {
            CooldownType::ScaleUp => state.last_scale_up,
            CooldownType::ScaleDown => state.last_scale_down,
            CooldownType::Any => state
                .last_any
                .or(state.last_scale_up)
                .or(state.last_scale_down),
        };

        if let Some(last_time) = last_time {
            let elapsed = Utc::now().signed_duration_since(last_time);
            let duration =
                chrono::Duration::from_std(self.get_cooldown_duration(&direction)).unwrap();
            elapsed < duration
        } else {
            false
        }
    }

    /// Internal: Calculate remaining cooldown
    fn calculate_remaining_cooldown(
        &self,
        state: &PoolCooldownState,
        direction: &ScaleDirection,
    ) -> Option<Duration> {
        let last_time = state.last_scale_up.or(state.last_scale_down);
        if let Some(last_time) = last_time {
            let elapsed = Utc::now().signed_duration_since(last_time);
            let duration =
                chrono::Duration::from_std(self.get_cooldown_duration(&direction)).unwrap();
            if elapsed < duration {
                Some(duration.to_std().unwrap() - elapsed.to_std().unwrap())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Internal: Get cooldown duration for direction
    fn get_cooldown_duration(&self, direction: &ScaleDirection) -> Duration {
        match direction {
            ScaleDirection::ScaleUp => self.config.scale_up_duration,
            ScaleDirection::ScaleDown => self.config.scale_down_duration,
        }
    }

    /// Internal: Count recent overrides
    async fn count_recent_overrides(&self, pool_id: &str, window: Duration) -> u32 {
        let history = self.event_history.read().await;
        let cutoff = Instant::now() - window;
        history
            .iter()
            .filter(|e| {
                e.pool_id == pool_id
                    && e.override_reason.is_some()
                    && e.timestamp > Utc::now() - chrono::Duration::from_std(window).unwrap()
            })
            .count() as u32
    }
}

impl PoolCooldownState {
    fn new(pool_id: String) -> Self {
        Self {
            pool_id,
            last_scale_up: None,
            last_scale_down: None,
            last_any: None,
            override_expiry: None,
            override_reason: None,
            event_count: HashMap::new(),
        }
    }
}

/// Create default cooldown configuration
impl Default for CooldownConfig {
    fn default() -> Self {
        Self {
            scale_up_duration: Duration::from_secs(60),
            scale_down_duration: Duration::from_secs(120),
            default_duration: Duration::from_secs(60),
            enabled: true,
            max_history: 1000,
        }
    }
}

/// Create default override configuration
impl Default for OverrideConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_reasons: vec![
                OverrideReason::Critical,
                OverrideReason::Emergency,
                OverrideReason::SLAViolation,
            ],
            max_overrides_per_hour: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cooldown_manager_creation() {
        let config = CooldownConfig::default();
        let override_config = OverrideConfig::default();
        let manager = AdvancedCooldownManager::new(config, override_config);

        assert!(manager.config.enabled);
        assert_eq!(manager.config.scale_up_duration, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_record_scaling_and_check_cooldown() {
        let config = CooldownConfig::default();
        let override_config = OverrideConfig::default();
        let manager = AdvancedCooldownManager::new(config, override_config);

        // Record a scale up event
        let result = manager
            .record_scaling("pool-1", &ScaleDirection::ScaleUp, CooldownType::ScaleUp)
            .await;
        assert!(result.is_ok());

        // Should be in cooldown now
        let in_cooldown = manager
            .is_in_cooldown("pool-1", &ScaleDirection::ScaleUp, CooldownType::ScaleUp)
            .await;
        assert!(in_cooldown);

        // Check remaining cooldown
        let remaining = manager
            .get_remaining_cooldown("pool-1", &ScaleDirection::ScaleUp)
            .await;
        assert!(remaining.is_some());
        assert!(remaining.unwrap() > Duration::from_secs(0));
    }

    #[tokio::test]
    async fn test_override_cooldown() {
        let config = CooldownConfig::default();
        let override_config = OverrideConfig::default();
        let manager = AdvancedCooldownManager::new(config, override_config.clone());

        // Record a scale up event
        manager
            .record_scaling("pool-1", &ScaleDirection::ScaleUp, CooldownType::ScaleUp)
            .await
            .unwrap();

        // Should be in cooldown
        assert!(
            manager
                .is_in_cooldown("pool-1", &ScaleDirection::ScaleUp, CooldownType::ScaleUp)
                .await
        );

        // Override cooldown
        let result = manager
            .override_cooldown("pool-1", &ScaleDirection::ScaleUp, OverrideReason::Critical)
            .await;
        assert!(result.is_ok());

        // Should not be in cooldown anymore
        let in_cooldown = manager
            .is_in_cooldown("pool-1", &ScaleDirection::ScaleUp, CooldownType::ScaleUp)
            .await;
        assert!(!in_cooldown);
    }

    #[tokio::test]
    async fn test_override_not_allowed() {
        let config = CooldownConfig::default();
        let override_config = OverrideConfig {
            enabled: false,
            allowed_reasons: vec![],
            max_overrides_per_hour: 0,
        };
        let manager = AdvancedCooldownManager::new(config, override_config);

        let result = manager
            .override_cooldown("pool-1", &ScaleDirection::ScaleUp, OverrideReason::Critical)
            .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(CooldownError::OverrideNotAllowed(_))));
    }

    #[tokio::test]
    async fn test_cooldown_violation() {
        let config = CooldownConfig {
            scale_up_duration: Duration::from_secs(10),
            ..Default::default()
        };
        let override_config = OverrideConfig::default();
        let manager = AdvancedCooldownManager::new(config, override_config);

        // Record a scale up event
        manager
            .record_scaling("pool-1", &ScaleDirection::ScaleUp, CooldownType::ScaleUp)
            .await
            .unwrap();

        // Try to scale up again immediately (should fail)
        let result = manager
            .record_scaling("pool-1", &ScaleDirection::ScaleUp, CooldownType::ScaleUp)
            .await;
        assert!(result.is_err());
        assert!(matches!(result, Err(CooldownError::CooldownViolation(_))));
    }

    #[tokio::test]
    async fn test_clear_cooldown() {
        let config = CooldownConfig::default();
        let override_config = OverrideConfig::default();
        let manager = AdvancedCooldownManager::new(config, override_config);

        // Record a scale up event
        manager
            .record_scaling("pool-1", &ScaleDirection::ScaleUp, CooldownType::ScaleUp)
            .await
            .unwrap();

        // Should be in cooldown
        assert!(
            manager
                .is_in_cooldown("pool-1", &ScaleDirection::ScaleUp, CooldownType::ScaleUp)
                .await
        );

        // Clear cooldown
        manager.clear_cooldown("pool-1").await;

        // Should not be in cooldown
        let in_cooldown = manager
            .is_in_cooldown("pool-1", &ScaleDirection::ScaleUp, CooldownType::ScaleUp)
            .await;
        assert!(!in_cooldown);
    }

    #[tokio::test]
    async fn test_cooldown_stats() {
        let config = CooldownConfig::default();
        let override_config = OverrideConfig::default();
        let manager = AdvancedCooldownManager::new(config, override_config);

        // Record multiple events
        manager
            .record_scaling("pool-1", &ScaleDirection::ScaleUp, CooldownType::ScaleUp)
            .await
            .unwrap();
        manager
            .record_scaling(
                "pool-1",
                &ScaleDirection::ScaleDown,
                CooldownType::ScaleDown,
            )
            .await
            .unwrap();

        let stats = manager.get_stats("pool-1").await.unwrap();
        assert_eq!(stats.total_cooldown_events, 2);
        assert_eq!(stats.scale_up_events, 1);
        assert_eq!(stats.scale_down_events, 1);
        assert_eq!(stats.override_events, 0);
    }

    #[tokio::test]
    async fn test_event_history() {
        let config = CooldownConfig::default();
        let override_config = OverrideConfig::default();
        let manager = AdvancedCooldownManager::new(config, override_config);

        // Record events
        manager
            .record_scaling("pool-1", &ScaleDirection::ScaleUp, CooldownType::ScaleUp)
            .await
            .unwrap();
        manager
            .record_scaling(
                "pool-1",
                &ScaleDirection::ScaleDown,
                CooldownType::ScaleDown,
            )
            .await
            .unwrap();

        let history = manager.get_history("pool-1").await;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].direction, ScaleDirection::ScaleUp);
        assert_eq!(history[1].direction, ScaleDirection::ScaleDown);
    }

    #[tokio::test]
    async fn test_cleanup_old_events() {
        let config = CooldownConfig::default();
        let override_config = OverrideConfig::default();
        let manager = AdvancedCooldownManager::new(config, override_config);

        // Record an event
        manager
            .record_scaling("pool-1", &ScaleDirection::ScaleUp, CooldownType::ScaleUp)
            .await
            .unwrap();

        // Cleanup with 0 retention
        let removed = manager.cleanup(Duration::from_secs(0)).await.unwrap();
        assert_eq!(removed, 1);

        // History should be empty
        let history = manager.get_history("pool-1").await;
        assert!(history.is_empty());
    }
}

// Error conversion to DomainError
impl From<CooldownError> for DomainError {
    fn from(err: CooldownError) -> Self {
        DomainError::Infrastructure(err.to_string())
    }
}
