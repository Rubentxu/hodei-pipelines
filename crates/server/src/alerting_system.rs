//! Alerting System Module
//!
//! Provides APIs for managing alerts and alert rules.
//! Implements US-014 from the Epic Web Frontend Production Ready.

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

/// Alert structure representing an active alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique alert ID
    pub id: String,
    /// Alert name
    pub name: String,
    /// Alert description
    pub description: String,
    /// Severity level (critical, warning, info)
    pub severity: String,
    /// Alert status (firing, resolved, pending)
    pub status: String,
    /// Associated rule ID
    pub rule_id: String,
    /// Tenant ID
    pub tenant_id: String,
    /// Alert labels
    pub labels: HashMap<String, String>,
    /// Alert annotations
    pub annotations: HashMap<String, String>,
    /// Alert start time
    pub start_time: DateTime<Utc>,
    /// Alert end time (optional)
    pub end_time: Option<DateTime<Utc>>,
    /// Alert creation timestamp
    pub created_at: DateTime<Utc>,
    /// Alert last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Alert rule structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Unique rule ID
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// PromQL-style query
    pub query: String,
    /// Severity level (critical, warning, info)
    pub severity: String,
    /// Whether the rule is enabled
    pub enabled: bool,
    /// Tenant ID
    pub tenant_id: String,
    /// Rule labels
    pub labels: HashMap<String, String>,
    /// Rule annotations
    pub annotations: HashMap<String, String>,
    /// Notification channels
    pub notification_channels: Vec<String>,
    /// Duration before firing (in seconds)
    pub for_duration: u32,
    /// Rule creation timestamp
    pub created_at: DateTime<Utc>,
    /// Rule last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Alert history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertHistoryEntry {
    /// Unique history entry ID
    pub id: String,
    /// Associated alert ID
    pub alert_id: String,
    /// Associated rule ID
    pub rule_id: String,
    /// Alert status
    pub status: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Alert labels
    pub labels: HashMap<String, String>,
    /// Alert annotations
    pub annotations: HashMap<String, String>,
}

/// Alert query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertQueryResponse {
    /// List of alerts
    pub alerts: Vec<Alert>,
    /// Total number of alerts
    pub total: u64,
}

/// Alert rule query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRuleQueryResponse {
    /// List of alert rules
    pub rules: Vec<AlertRule>,
    /// Total number of rules
    pub total: u64,
}

/// Alert history query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertHistoryResponse {
    /// List of history entries
    pub entries: Vec<AlertHistoryEntry>,
    /// Total number of entries
    pub total: u64,
}

/// Service for managing alerts
#[derive(Debug)]
pub struct AlertingService {
    /// Mock alerts for demonstration
    mock_alerts: Arc<Vec<Alert>>,
    /// Mock alert rules for demonstration
    mock_rules: Arc<Vec<AlertRule>>,
    /// Mock alert history for demonstration
    mock_history: Arc<Vec<AlertHistoryEntry>>,
}

impl AlertingService {
    /// Create new alerting service
    pub fn new() -> Self {
        let mock_rules = Self::generate_mock_rules();
        let mock_alerts = Self::generate_mock_alerts(&mock_rules);
        let mock_history = Self::generate_mock_history(&mock_alerts, &mock_rules);

        Self {
            mock_alerts: Arc::new(mock_alerts),
            mock_rules: Arc::new(mock_rules),
            mock_history: Arc::new(mock_history),
        }
    }

    /// Get all alerts
    pub async fn get_alerts(&self, tenant_id: Option<&str>) -> Vec<Alert> {
        let alerts = self.mock_alerts.clone();

        if let Some(tenant) = tenant_id {
            alerts
                .iter()
                .filter(|a| a.tenant_id == tenant)
                .cloned()
                .collect()
        } else {
            alerts.iter().cloned().collect()
        }
    }

    /// Get a specific alert by ID
    pub async fn get_alert(&self, alert_id: &str) -> Option<Alert> {
        self.mock_alerts
            .iter()
            .find(|alert| alert.id == alert_id)
            .cloned()
    }

    /// Create a new alert
    pub async fn create_alert(&self, alert: Alert) -> Result<Alert, String> {
        // In production, this would save to database
        Ok(alert)
    }

    /// Get all alert rules
    pub async fn get_alert_rules(&self, tenant_id: Option<&str>) -> Vec<AlertRule> {
        let rules = self.mock_rules.clone();

        if let Some(tenant) = tenant_id {
            rules
                .iter()
                .filter(|r| r.tenant_id == tenant)
                .cloned()
                .collect()
        } else {
            rules.iter().cloned().collect()
        }
    }

    /// Get a specific alert rule by ID
    pub async fn get_alert_rule(&self, rule_id: &str) -> Option<AlertRule> {
        self.mock_rules
            .iter()
            .find(|rule| rule.id == rule_id)
            .cloned()
    }

    /// Create a new alert rule
    pub async fn create_alert_rule(&self, rule: AlertRule) -> Result<AlertRule, String> {
        // In production, this would save to database
        Ok(rule)
    }

    /// Update an existing alert rule
    pub async fn update_alert_rule(
        &self,
        _rule_id: &str,
        rule: AlertRule,
    ) -> Result<AlertRule, String> {
        // In production, this would update in database
        Ok(rule)
    }

    /// Delete an alert rule
    pub async fn delete_alert_rule(&self, _rule_id: &str) -> Result<(), String> {
        // In production, this would delete from database
        Ok(())
    }

    /// Enable/disable an alert rule
    pub async fn toggle_alert_rule(&self, _rule_id: &str, _enabled: bool) -> Result<(), String> {
        // In production, this would update in database
        Ok(())
    }

    /// Get alert history
    pub async fn get_alert_history(
        &self,
        alert_id: Option<&str>,
        limit: Option<u32>,
    ) -> Vec<AlertHistoryEntry> {
        let history = self.mock_history.clone();

        let history_vec = if let Some(alert) = alert_id {
            history
                .iter()
                .filter(|h| h.alert_id == alert)
                .cloned()
                .collect::<Vec<_>>()
        } else {
            history.iter().cloned().collect::<Vec<_>>()
        };

        if let Some(lim) = limit {
            history_vec.into_iter().take(lim as usize).collect()
        } else {
            history_vec
        }
    }

    /// Generate mock alerts for demonstration
    fn generate_mock_alerts(rules: &[AlertRule]) -> Vec<Alert> {
        use rand::Rng;
        let mut rng = rand::rng();

        let tenants = vec!["tenant-123", "tenant-456", "tenant-789"];
        let severities = vec!["critical", "warning", "info"];
        let statuses = vec!["firing", "resolved", "pending"];

        let mut alerts = Vec::new();
        let now = Utc::now();

        for i in 0..50 {
            let tenant_idx = rng.random_range(0..tenants.len());
            let severity_idx = rng.random_range(0..severities.len());
            let status_idx = rng.random_range(0..statuses.len());
            let rule = &rules[rng.random_range(0..rules.len())];

            let alert_id = format!("alert-{}", i);
            let tenant_id = tenants[tenant_idx].to_string();
            let severity = severities[severity_idx].to_string();
            let status = statuses[status_idx].to_string();

            let start_time = now - chrono::Duration::minutes(rng.random_range(0..1440)); // Random within last 24 hours
            let end_time = if status == "resolved" {
                Some(start_time + chrono::Duration::minutes(rng.random_range(1..60)))
            } else {
                None
            };

            let labels = {
                let mut labels = HashMap::new();
                labels.insert("service".to_string(), "hodei-server".to_string());
                labels.insert(
                    "instance".to_string(),
                    format!("prod-{}", rng.random_range(1..10)),
                );
                labels
            };

            let annotations = {
                let mut annotations = HashMap::new();
                annotations.insert("summary".to_string(), format!("{} detected", rule.name));
                annotations.insert("description".to_string(), rule.description.clone());
                annotations
            };

            alerts.push(Alert {
                id: alert_id,
                name: rule.name.clone(),
                description: rule.description.clone(),
                severity,
                status,
                rule_id: rule.id.clone(),
                tenant_id,
                labels,
                annotations,
                start_time,
                end_time,
                created_at: start_time,
                updated_at: end_time.unwrap_or(now),
            });
        }

        alerts
    }

    /// Generate mock alert rules for demonstration
    fn generate_mock_rules() -> Vec<AlertRule> {
        use rand::Rng;
        let mut rng = rand::rng();

        let tenants = vec!["tenant-123", "tenant-456", "tenant-789"];
        let severities = vec!["critical", "warning", "info"];
        let notification_channels = vec![
            vec!["email".to_string()],
            vec!["email".to_string(), "slack".to_string()],
            vec!["slack".to_string(), "webhook".to_string()],
            vec![
                "email".to_string(),
                "slack".to_string(),
                "webhook".to_string(),
            ],
        ];

        let rule_templates = vec![
            ("High CPU Usage", "cpu_usage > 90"),
            ("High Memory Usage", "memory_usage > 85"),
            ("High Disk Usage", "disk_usage > 90"),
            ("Service Down", "service_status == 0"),
            ("High Error Rate", "error_rate > 5"),
            ("High Response Time", "response_time > 1000"),
            ("Low Throughput", "throughput < 100"),
        ];

        let mut rules = Vec::new();
        let now = Utc::now();

        for i in 0..30 {
            let tenant_idx = rng.random_range(0..tenants.len());
            let severity_idx = rng.random_range(0..severities.len());
            let channel_idx = rng.random_range(0..notification_channels.len());
            let template_idx = rng.random_range(0..rule_templates.len());

            let (name, query) = rule_templates[template_idx];
            let severity = severities[severity_idx].to_string();
            let tenant_id = tenants[tenant_idx].to_string();

            let labels = {
                let mut labels = HashMap::new();
                labels.insert("team".to_string(), "platform".to_string());
                labels.insert("category".to_string(), "infrastructure".to_string());
                labels
            };

            let annotations = {
                let mut annotations = HashMap::new();
                annotations.insert(
                    "runbook".to_string(),
                    "https://wiki.example.com/runbook".to_string(),
                );
                annotations.insert(
                    "documentation".to_string(),
                    "https://docs.example.com/alerts".to_string(),
                );
                annotations
            };

            rules.push(AlertRule {
                id: format!("rule-{}", i),
                name: name.to_string(),
                description: format!("Alert when {}", name.to_lowercase()),
                query: query.to_string(),
                severity,
                enabled: rng.random_bool(0.8),
                tenant_id,
                labels,
                annotations,
                notification_channels: notification_channels[channel_idx].clone(),
                for_duration: rng.random_range(60..600), // 1-10 minutes
                created_at: now - chrono::Duration::days(rng.random_range(1..30)),
                updated_at: now,
            });
        }

        rules
    }

    /// Generate mock alert history for demonstration
    fn generate_mock_history(alerts: &[Alert], rules: &[AlertRule]) -> Vec<AlertHistoryEntry> {
        use rand::Rng;
        let mut rng = rand::rng();

        let statuses = vec!["firing", "resolved"];

        let mut history = Vec::new();
        let now = Utc::now();

        for i in 0..200 {
            let alert = &alerts[rng.random_range(0..alerts.len())];
            let _rule = rules.iter().find(|r| r.id == alert.rule_id).unwrap();
            let status = statuses[rng.random_range(0..statuses.len())];

            let timestamp = now - chrono::Duration::hours(rng.random_range(0..168)); // Random within last week

            let labels = {
                let mut labels = alert.labels.clone();
                labels.insert("history_id".to_string(), format!("history-{}", i));
                labels
            };

            let annotations = {
                let mut annotations = alert.annotations.clone();
                annotations.insert(
                    "message".to_string(),
                    format!("Alert {}: {}", status, alert.name),
                );
                annotations
            };

            history.push(AlertHistoryEntry {
                id: format!("history-{}", i),
                alert_id: alert.id.clone(),
                rule_id: alert.rule_id.clone(),
                status: status.to_string(),
                timestamp,
                labels,
                annotations,
            });
        }

        history
    }
}

impl Default for AlertingService {
    fn default() -> Self {
        Self::new()
    }
}

/// Application state for Alerting API
#[derive(Clone)]
pub struct AlertingApiAppState {
    pub service: Arc<AlertingService>,
}

/// GET /api/v1/alerts - Get all alerts
#[allow(dead_code)]
pub async fn get_alerts_handler(
    State(state): State<AlertingApiAppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<AlertQueryResponse>, StatusCode> {
    info!("🔔 Alerts requested");

    let tenant_id = params.get("tenant_id").map(|s| s.as_str());
    let alerts = state.service.get_alerts(tenant_id).await;
    let total = alerts.len() as u64;

    info!("✅ Returned {} alerts", total);

    Ok(Json(AlertQueryResponse { alerts, total }))
}

/// GET /api/v1/alerts/{id} - Get a specific alert
#[allow(dead_code)]
pub async fn get_alert_handler(
    State(state): State<AlertingApiAppState>,
    Path(alert_id): Path<String>,
) -> Result<Json<Alert>, StatusCode> {
    info!("🔔 Alert requested: {}", alert_id);

    match state.service.get_alert(&alert_id).await {
        Some(alert) => {
            info!("✅ Alert found: {}", alert_id);
            Ok(Json(alert))
        }
        None => {
            info!("❌ Alert not found: {}", alert_id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// POST /api/v1/alerts - Create a new alert
#[allow(dead_code)]
pub async fn create_alert_handler(
    State(state): State<AlertingApiAppState>,
    Json(alert): Json<Alert>,
) -> Result<Json<Alert>, StatusCode> {
    info!("🔔 Creating alert: {}", alert.id);

    match state.service.create_alert(alert).await {
        Ok(alert) => {
            info!("✅ Alert created: {}", alert.id);
            Ok(Json(alert))
        }
        Err(e) => {
            info!("❌ Failed to create alert: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v1/alerts/rules - Get all alert rules
#[allow(dead_code)]
pub async fn get_alert_rules_handler(
    State(state): State<AlertingApiAppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<AlertRuleQueryResponse>, StatusCode> {
    info!("📋 Alert rules requested");

    let tenant_id = params.get("tenant_id").map(|s| s.as_str());
    let rules = state.service.get_alert_rules(tenant_id).await;
    let total = rules.len() as u64;

    info!("✅ Returned {} alert rules", total);

    Ok(Json(AlertRuleQueryResponse { rules, total }))
}

/// POST /api/v1/alerts/rules - Create a new alert rule
#[allow(dead_code)]
pub async fn create_alert_rule_handler(
    State(state): State<AlertingApiAppState>,
    Json(rule): Json<AlertRule>,
) -> Result<Json<AlertRule>, StatusCode> {
    info!("📋 Creating alert rule: {}", rule.id);

    match state.service.create_alert_rule(rule).await {
        Ok(rule) => {
            info!("✅ Alert rule created: {}", rule.id);
            Ok(Json(rule))
        }
        Err(e) => {
            info!("❌ Failed to create alert rule: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// PUT /api/v1/alerts/rules/{id} - Update an alert rule
#[allow(dead_code)]
pub async fn update_alert_rule_handler(
    State(state): State<AlertingApiAppState>,
    Path(rule_id): Path<String>,
    Json(rule): Json<AlertRule>,
) -> Result<Json<AlertRule>, StatusCode> {
    info!("📋 Updating alert rule: {}", rule_id);

    match state.service.update_alert_rule(&rule_id, rule).await {
        Ok(rule) => {
            info!("✅ Alert rule updated: {}", rule.id);
            Ok(Json(rule))
        }
        Err(e) => {
            info!("❌ Failed to update alert rule: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /api/v1/alerts/rules/{id} - Delete an alert rule
#[allow(dead_code)]
pub async fn delete_alert_rule_handler(
    State(state): State<AlertingApiAppState>,
    Path(rule_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    info!("📋 Deleting alert rule: {}", rule_id);

    match state.service.delete_alert_rule(&rule_id).await {
        Ok(_) => {
            info!("✅ Alert rule deleted: {}", rule_id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            info!("❌ Failed to delete alert rule: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v1/alerts/history - Get alert history
#[allow(dead_code)]
pub async fn get_alert_history_handler(
    State(state): State<AlertingApiAppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<AlertHistoryResponse>, StatusCode> {
    info!("📊 Alert history requested");

    let alert_id = params.get("alert_id").map(|s| s.as_str());
    let limit = params.get("limit").and_then(|s| s.parse().ok());

    let entries = state.service.get_alert_history(alert_id, limit).await;
    let total = entries.len() as u64;

    info!("✅ Returned {} history entries", total);

    Ok(Json(AlertHistoryResponse { entries, total }))
}

/// Alerting API routes
pub fn alerting_api_routes() -> Router<AlertingApiAppState> {
    Router::new()
        .route(
            "/alerts",
            get(get_alerts_handler).post(create_alert_handler),
        )
        .route("/alerts/{id}", get(get_alert_handler))
        .route(
            "/alerts/rules",
            get(get_alert_rules_handler).post(create_alert_rule_handler),
        )
        .route(
            "/alerts/rules/{id}",
            get(get_alert_handler)
                .put(update_alert_rule_handler)
                .delete(delete_alert_rule_handler),
        )
        .route("/alerts/history", get(get_alert_history_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_alerting_service_new() {
        let service = AlertingService::new();
        assert!(!service.mock_alerts.is_empty());
        assert!(!service.mock_rules.is_empty());
        assert!(!service.mock_history.is_empty());
    }

    #[tokio::test]
    async fn test_alerting_service_get_alerts() {
        let service = AlertingService::new();
        let alerts = service.get_alerts(None).await;
        assert!(!alerts.is_empty());
    }

    #[tokio::test]
    async fn test_alerting_service_get_alert_rules() {
        let service = AlertingService::new();
        let rules = service.get_alert_rules(None).await;
        assert!(!rules.is_empty());
    }

    #[tokio::test]
    async fn test_alerting_service_get_alert_history() {
        let service = AlertingService::new();
        let history = service.get_alert_history(None, Some(10)).await;
        assert!(history.len() <= 10);
    }

    #[tokio::test]
    async fn test_alert_serialization() {
        let alert = Alert {
            id: "alert-1".to_string(),
            name: "High CPU Usage".to_string(),
            description: "CPU usage above threshold".to_string(),
            severity: "critical".to_string(),
            status: "firing".to_string(),
            rule_id: "rule-1".to_string(),
            tenant_id: "tenant-123".to_string(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
            start_time: Utc::now(),
            end_time: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&alert).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("severity"));

        let deserialized: Alert = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, alert.id);
    }

    #[tokio::test]
    async fn test_alert_rule_serialization() {
        let rule = AlertRule {
            id: "rule-1".to_string(),
            name: "High CPU Usage".to_string(),
            description: "CPU usage above threshold".to_string(),
            query: "cpu_usage > 90".to_string(),
            severity: "critical".to_string(),
            enabled: true,
            tenant_id: "tenant-123".to_string(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
            notification_channels: vec!["email".to_string()],
            for_duration: 300,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("query"));

        let deserialized: AlertRule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, rule.id);
    }

    #[tokio::test]
    async fn test_alert_history_entry_serialization() {
        let entry = AlertHistoryEntry {
            id: "history-1".to_string(),
            alert_id: "alert-1".to_string(),
            rule_id: "rule-1".to_string(),
            status: "firing".to_string(),
            timestamp: Utc::now(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("alert_id"));
        assert!(json.contains("status"));

        let deserialized: AlertHistoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, entry.id);
    }
}
