use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::{IntoParams, ToSchema};

use crate::AppState;

/// Alerting rule
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertingRule {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub alert_type: AlertType,
    pub query: String,
    pub conditions: Vec<Condition>,
    pub severity: AlertSeverity,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub enabled: bool,
    pub for_duration: String,
    pub repeat_interval: String,
}

/// Type of alert
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    MetricThreshold,
    AnomalyDetection,
    ResourceExhaustion,
    PerformanceDegradation,
    Custom,
}

/// Alert severity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Condition for triggering an alert
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Condition {
    pub operator: ConditionOperator,
    pub threshold: f64,
    pub duration: String,
}

/// Condition operator
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// Alert instance
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertInstance {
    pub id: String,
    pub rule_id: String,
    pub status: AlertStatus,
    pub started_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub current_value: f64,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
}

/// Alert status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AlertStatus {
    Pending,
    Firing,
    Resolved,
}

/// Notification channel
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotificationChannel {
    pub id: String,
    pub name: String,
    pub channel_type: ChannelType,
    pub configuration: HashMap<String, String>,
    pub enabled: bool,
}

/// Type of notification channel
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Email,
    Slack,
    Webhook,
    PagerDuty,
    SMS,
}

/// Silence rule
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SilenceRule {
    pub id: String,
    pub matchers: HashMap<String, String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub created_by: String,
    pub comment: Option<String>,
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertingConfig {
    pub enabled: bool,
    pub evaluation_interval: String,
    pub external_url: String,
    pub notification_channels: Vec<NotificationChannel>,
}

/// Service for alerting rules management
#[derive(Debug)]
pub struct AlertingRulesService {
    /// Alerting configuration
    config: Arc<RwLock<AlertingConfig>>,
    /// Registered alerting rules
    rules: Arc<RwLock<HashMap<String, AlertingRule>>>,
    /// Active alert instances
    alert_instances: Arc<RwLock<HashMap<String, AlertInstance>>>,
    /// Silence rules
    silences: Arc<RwLock<HashMap<String, SilenceRule>>>,
}

impl AlertingRulesService {
    /// Create new alerting rules service
    pub fn new() -> Self {
        let default_config = AlertingConfig {
            enabled: true,
            evaluation_interval: "30s".to_string(),
            external_url: "http://localhost:9093".to_string(),
            notification_channels: vec![NotificationChannel {
                id: "default-email".to_string(),
                name: "Default Email".to_string(),
                channel_type: ChannelType::Email,
                configuration: HashMap::from([("to".to_string(), "admin@example.com".to_string())]),
                enabled: true,
            }],
        };

        Self {
            config: Arc::new(RwLock::new(default_config)),
            rules: Arc::new(RwLock::new(HashMap::new())),
            alert_instances: Arc::new(RwLock::new(HashMap::new())),
            silences: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create alerting rule
    pub async fn create_rule(&self, rule: AlertingRule) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        rules.insert(rule.id.clone(), rule);
        Ok(())
    }

    /// Get rule by ID
    pub async fn get_rule(&self, id: &str) -> Option<AlertingRule> {
        let rules = self.rules.read().await;
        rules.get(id).cloned()
    }

    /// List all rules
    pub async fn list_rules(&self) -> Vec<AlertingRule> {
        let rules = self.rules.read().await;
        rules.values().cloned().collect()
    }

    /// Update rule
    pub async fn update_rule(&self, rule: AlertingRule) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        rules.insert(rule.id.clone(), rule);
        Ok(())
    }

    /// Delete rule
    pub async fn delete_rule(&self, id: &str) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        rules.remove(id);
        Ok(())
    }

    /// Enable rule
    pub async fn enable_rule(&self, id: &str) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        if let Some(rule) = rules.get_mut(id) {
            rule.enabled = true;
            Ok(())
        } else {
            Err("Rule not found".to_string())
        }
    }

    /// Disable rule
    pub async fn disable_rule(&self, id: &str) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        if let Some(rule) = rules.get_mut(id) {
            rule.enabled = false;
            Ok(())
        } else {
            Err("Rule not found".to_string())
        }
    }

    /// Get active alert instances
    pub async fn get_alert_instances(&self) -> Vec<AlertInstance> {
        let instances = self.alert_instances.read().await;
        instances.values().cloned().collect()
    }

    /// Get alert instance by ID
    pub async fn get_alert_instance(&self, id: &str) -> Option<AlertInstance> {
        let instances = self.alert_instances.read().await;
        instances.get(id).cloned()
    }

    /// Acknowledge alert
    pub async fn acknowledge_alert(&self, id: &str) -> Result<(), String> {
        let mut instances = self.alert_instances.write().await;
        if let Some(instance) = instances.get_mut(id) {
            // In a real implementation, would update acknowledge status
            Ok(())
        } else {
            Err("Alert not found".to_string())
        }
    }

    /// Create silence rule
    pub async fn create_silence(&self, silence: SilenceRule) -> Result<(), String> {
        let mut silences = self.silences.write().await;
        silences.insert(silence.id.clone(), silence);
        Ok(())
    }

    /// Get silence by ID
    pub async fn get_silence(&self, id: &str) -> Option<SilenceRule> {
        let silences = self.silences.read().await;
        silences.get(id).cloned()
    }

    /// List all silences
    pub async fn list_silences(&self) -> Vec<SilenceRule> {
        let silences = self.silences.read().await;
        silences.values().cloned().collect()
    }

    /// Delete silence
    pub async fn delete_silence(&self, id: &str) -> Result<(), String> {
        let mut silences = self.silences.write().await;
        silences.remove(id);
        Ok(())
    }

    /// Get configuration
    pub async fn get_config(&self) -> AlertingConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Update configuration
    pub async fn update_config(&self, config: AlertingConfig) {
        let mut config_lock = self.config.write().await;
        *config_lock = config;
    }

    /// Get alert statistics
    pub async fn get_alert_statistics(&self) -> AlertStatistics {
        let instances = self.alert_instances.read().await;
        let mut stats = AlertStatistics {
            total: instances.len() as u64,
            pending: 0,
            firing: 0,
            resolved: 0,
            by_severity: HashMap::new(),
        };

        for instance in instances.values() {
            match instance.status {
                AlertStatus::Pending => stats.pending += 1,
                AlertStatus::Firing => stats.firing += 1,
                AlertStatus::Resolved => stats.resolved += 1,
            }
        }

        stats
    }
}

/// Alert statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertStatistics {
    pub total: u64,
    pub pending: u64,
    pub firing: u64,
    pub resolved: u64,
    pub by_severity: HashMap<String, u64>,
}

/// Create alerting rule
#[allow(dead_code)] //#[utoipa::path(
    post,
    path = "/api/v1/alerting/rules",
    request_body = AlertingRule,
    responses(
        (status = 201, description = "Alerting rule created successfully"),
        (status = 400, description = "Invalid rule")
    ),
    tag = "Alerting Rules"
)]
pub async fn create_alert_rule(
    State(state): State<AlertingRulesAppState>,
    Json(rule): Json<AlertingRule>,
) -> Result<Json<String>, StatusCode> {
    state
        .service
        .create_rule(rule)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json("Alerting rule created successfully".to_string()))
}

/// Get alerting rule by ID
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/alerting/rules/{id}",
    params(
        ("id" = String, Path, description = "Rule ID")
    ),
    responses(
        (status = 200, description = "Alerting rule retrieved successfully", body = AlertingRule),
        (status = 404, description = "Rule not found")
    ),
    tag = "Alerting Rules"
)]
pub async fn get_alert_rule(
    Path(id): Path<String>,
    State(state): State<AlertingRulesAppState>,
) -> Result<Json<AlertingRule>, StatusCode> {
    let rule = state
        .service
        .get_rule(&id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(rule))
}

/// List all alerting rules
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/alerting/rules",
    responses(
        (status = 200, description = "Alerting rules retrieved successfully", body = Vec<AlertingRule>)
    ),
    tag = "Alerting Rules"
)]
pub async fn list_alert_rules(
    State(state): State<AlertingRulesAppState>,
) -> Result<Json<Vec<AlertingRule>>, StatusCode> {
    let rules = state.service.list_rules().await;
    Ok(Json(rules))
}

/// Update alerting rule
#[allow(dead_code)] //#[utoipa::path(
    put,
    path = "/api/v1/alerting/rules/{id}",
    params(
        ("id" = String, Path, description = "Rule ID")
    ),
    request_body = AlertingRule,
    responses(
        (status = 200, description = "Alerting rule updated successfully"),
        (status = 404, description = "Rule not found")
    ),
    tag = "Alerting Rules"
)]
pub async fn update_alert_rule(
    Path(id): Path<String>,
    State(state): State<AlertingRulesAppState>,
    Json(rule): Json<AlertingRule>,
) -> Result<Json<String>, StatusCode> {
    if rule.id != id {
        return Err(StatusCode::BAD_REQUEST);
    }

    state
        .service
        .update_rule(rule)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json("Alerting rule updated successfully".to_string()))
}

/// Delete alerting rule
#[allow(dead_code)] //#[utoipa::path(
    delete,
    path = "/api/v1/alerting/rules/{id}",
    params(
        ("id" = String, Path, description = "Rule ID")
    ),
    responses(
        (status = 200, description = "Alerting rule deleted successfully"),
        (status = 404, description = "Rule not found")
    ),
    tag = "Alerting Rules"
)]
pub async fn delete_alert_rule(
    Path(id): Path<String>,
    State(state): State<AlertingRulesAppState>,
) -> Result<Json<String>, StatusCode> {
    state
        .service
        .delete_rule(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json("Alerting rule deleted successfully".to_string()))
}

/// Enable alerting rule
#[allow(dead_code)] //#[utoipa::path(
    post,
    path = "/api/v1/alerting/rules/{id}/enable",
    params(
        ("id" = String, Path, description = "Rule ID")
    ),
    responses(
        (status = 200, description = "Alerting rule enabled successfully"),
        (status = 404, description = "Rule not found")
    ),
    tag = "Alerting Rules"
)]
pub async fn enable_alert_rule(
    Path(id): Path<String>,
    State(state): State<AlertingRulesAppState>,
) -> Result<Json<String>, StatusCode> {
    state
        .service
        .enable_rule(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json("Alerting rule enabled successfully".to_string()))
}

/// Disable alerting rule
#[allow(dead_code)] //#[utoipa::path(
    post,
    path = "/api/v1/alerting/rules/{id}/disable",
    params(
        ("id" = String, Path, description = "Rule ID")
    ),
    responses(
        (status = 200, description = "Alerting rule disabled successfully"),
        (status = 404, description = "Rule not found")
    ),
    tag = "Alerting Rules"
)]
pub async fn disable_alert_rule(
    Path(id): Path<String>,
    State(state): State<AlertingRulesAppState>,
) -> Result<Json<String>, StatusCode> {
    state
        .service
        .disable_rule(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json("Alerting rule disabled successfully".to_string()))
}

/// Get alert instances
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/alerting/alerts",
    responses(
        (status = 200, description = "Alert instances retrieved successfully", body = Vec<AlertInstance>)
    ),
    tag = "Alerting Rules"
)]
pub async fn get_alert_instances(
    State(state): State<AlertingRulesAppState>,
) -> Result<Json<Vec<AlertInstance>>, StatusCode> {
    let instances = state.service.get_alert_instances().await;
    Ok(Json(instances))
}

/// Get alert instance by ID
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/alerting/alerts/{id}",
    params(
        ("id" = String, Path, description = "Alert ID")
    ),
    responses(
        (status = 200, description = "Alert instance retrieved successfully", body = AlertInstance),
        (status = 404, description = "Alert not found")
    ),
    tag = "Alerting Rules"
)]
pub async fn get_alert_instance(
    Path(id): Path<String>,
    State(state): State<AlertingRulesAppState>,
) -> Result<Json<AlertInstance>, StatusCode> {
    let instance = state
        .service
        .get_alert_instance(&id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(instance))
}

/// Acknowledge alert
#[allow(dead_code)] //#[utoipa::path(
    post,
    path = "/api/v1/alerting/alerts/{id}/acknowledge",
    params(
        ("id" = String, Path, description = "Alert ID")
    ),
    responses(
        (status = 200, description = "Alert acknowledged successfully"),
        (status = 404, description = "Alert not found")
    ),
    tag = "Alerting Rules"
)]
pub async fn acknowledge_alert(
    Path(id): Path<String>,
    State(state): State<AlertingRulesAppState>,
) -> Result<Json<String>, StatusCode> {
    state
        .service
        .acknowledge_alert(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json("Alert acknowledged successfully".to_string()))
}

/// Get alert statistics
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/alerting/statistics",
    responses(
        (status = 200, description = "Alert statistics retrieved successfully", body = AlertStatistics)
    ),
    tag = "Alerting Rules"
)]
pub async fn get_alert_statistics(
    State(state): State<AlertingRulesAppState>,
) -> Result<Json<AlertStatistics>, StatusCode> {
    let stats = state.service.get_alert_statistics().await;
    Ok(Json(stats))
}

/// Create silence rule
#[allow(dead_code)] //#[utoipa::path(
    post,
    path = "/api/v1/alerting/silences",
    request_body = SilenceRule,
    responses(
        (status = 201, description = "Silence rule created successfully"),
        (status = 400, description = "Invalid silence rule")
    ),
    tag = "Alerting Rules"
)]
pub async fn create_silence(
    State(state): State<AlertingRulesAppState>,
    Json(silence): Json<SilenceRule>,
) -> Result<Json<String>, StatusCode> {
    state
        .service
        .create_silence(silence)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json("Silence rule created successfully".to_string()))
}

/// List all silences
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/alerting/silences",
    responses(
        (status = 200, description = "Silence rules retrieved successfully", body = Vec<SilenceRule>)
    ),
    tag = "Alerting Rules"
)]
pub async fn list_silences(
    State(state): State<AlertingRulesAppState>,
) -> Result<Json<Vec<SilenceRule>>, StatusCode> {
    let silences = state.service.list_silences().await;
    Ok(Json(silences))
}

/// Delete silence rule
#[allow(dead_code)] //#[utoipa::path(
    delete,
    path = "/api/v1/alerting/silences/{id}",
    params(
        ("id" = String, Path, description = "Silence ID")
    ),
    responses(
        (status = 200, description = "Silence rule deleted successfully"),
        (status = 404, description = "Silence rule not found")
    ),
    tag = "Alerting Rules"
)]
pub async fn delete_silence(
    Path(id): Path<String>,
    State(state): State<AlertingRulesAppState>,
) -> Result<Json<String>, StatusCode> {
    state
        .service
        .delete_silence(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json("Silence rule deleted successfully".to_string()))
}

/// Get alerting configuration
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/alerting/config",
    responses(
        (status = 200, description = "Alerting configuration retrieved successfully", body = AlertingConfig)
    ),
    tag = "Alerting Rules"
)]
pub async fn get_alerting_config(
    State(state): State<AlertingRulesAppState>,
) -> Result<Json<AlertingConfig>, StatusCode> {
    let config = state.service.get_config().await;
    Ok(Json(config))
}

/// Update alerting configuration
#[allow(dead_code)] //#[utoipa::path(
    put,
    path = "/api/v1/alerting/config",
    request_body = AlertingConfig,
    responses(
        (status = 200, description = "Alerting configuration updated successfully"),
        (status = 400, description = "Invalid configuration")
    ),
    tag = "Alerting Rules"
)]
pub async fn update_alerting_config(
    State(state): State<AlertingRulesAppState>,
    Json(config): Json<AlertingConfig>,
) -> Result<Json<String>, StatusCode> {
    state.service.update_config(config).await;
    Ok(Json(
        "Alerting configuration updated successfully".to_string(),
    ))
}

/// Application state for Alerting Rules
#[derive(Clone)]
pub struct AlertingRulesAppState {
    pub service: Arc<AlertingRulesService>,
}

/// Alerting rules routes
pub fn alerting_rules_routes() -> Router<AlertingRulesAppState> {
    Router::new()
        .route("/alerting/config", get(get_alerting_config))
        .route("/alerting/config", put(update_alerting_config))
        .route("/alerting/rules", get(list_alert_rules))
        .route("/alerting/rules", post(create_alert_rule))
        .route("/alerting/rules/{id}", get(get_alert_rule))
        .route("/alerting/rules/{id}", put(update_alert_rule))
        .route("/alerting/rules/{id}", delete(delete_alert_rule))
        .route("/alerting/rules/{id}/enable", post(enable_alert_rule))
        .route("/alerting/rules/{id}/disable", post(disable_alert_rule))
        .route("/alerting/alerts", get(get_alert_instances))
        .route("/alerting/alerts/{id}", get(get_alert_instance))
        .route("/alerting/alerts/{id}/acknowledge", post(acknowledge_alert))
        .route("/alerting/statistics", get(get_alert_statistics))
        .route("/alerting/silences", get(list_silences))
        .route("/alerting/silences", post(create_silence))
        .route("/alerting/silences/{id}", delete(delete_silence))
}
