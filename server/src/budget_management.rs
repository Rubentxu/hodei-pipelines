//! Budget Management & Alerts Module
//!
//! Provides budget tracking and alerting capabilities for tenant cost management.
//! Implements US-017 from the Epic Web Frontend Production Ready.

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Budget period enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BudgetPeriod {
    /// Daily budget
    Daily,
    /// Weekly budget
    Weekly,
    /// Monthly budget
    Monthly,
    /// Quarterly budget
    Quarterly,
    /// Yearly budget
    Yearly,
}

/// Alert threshold enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertThreshold {
    /// Alert at 50% of budget
    FiftyPercent,
    /// Alert at 75% of budget
    SeventyFivePercent,
    /// Alert at 90% of budget
    NinetyPercent,
    /// Alert at 100% of budget
    HundredPercent,
    /// Custom threshold percentage
    Custom(f64),
}

/// Budget structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    /// Unique budget ID
    pub id: String,
    /// Tenant ID this budget belongs to
    pub tenant_id: String,
    /// Budget name/description
    pub name: String,
    /// Maximum budget amount
    pub amount_limit: f64,
    /// Current spending amount
    pub current_spend: f64,
    /// Budget period (daily, weekly, monthly, etc.)
    pub period: BudgetPeriod,
    /// Currency code (USD, EUR, etc.)
    pub currency: String,
    /// Whether alerts are enabled
    pub alerts_enabled: bool,
    /// Alert thresholds configured
    pub alert_thresholds: Vec<AlertThreshold>,
    /// Period start date
    pub period_start: DateTime<Utc>,
    /// Period end date
    pub period_end: DateTime<Utc>,
    /// Budget creation date
    pub created_at: DateTime<Utc>,
    /// Budget last updated date
    pub updated_at: DateTime<Utc>,
    /// Whether budget is active
    pub is_active: bool,
}

/// Budget alert structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAlert {
    /// Unique alert ID
    pub id: String,
    /// Budget ID this alert belongs to
    pub budget_id: String,
    /// Tenant ID
    pub tenant_id: String,
    /// Alert threshold that triggered
    pub threshold: AlertThreshold,
    /// Threshold percentage (0-100)
    pub threshold_percentage: f64,
    /// Current spending amount when alert triggered
    pub current_spend: f64,
    /// Budget limit amount
    pub budget_limit: f64,
    /// Alert type
    pub alert_type: String,
    /// Alert message
    pub message: String,
    /// Alert severity (info, warning, critical)
    pub severity: String,
    /// Date alert was triggered
    pub triggered_at: DateTime<Utc>,
    /// Whether alert has been acknowledged
    pub acknowledged: bool,
    /// Date alert was acknowledged (if applicable)
    pub acknowledged_at: Option<DateTime<Utc>>,
}

/// Budget usage structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetUsage {
    /// Budget ID
    pub budget_id: String,
    /// Tenant ID
    pub tenant_id: String,
    /// Budget limit
    pub limit: f64,
    /// Current spending
    pub current_spend: f64,
    /// Percentage used (0-100)
    pub percentage_used: f64,
    /// Remaining budget
    pub remaining: f64,
    /// Days remaining in period
    pub days_remaining: u32,
    /// Average daily spend
    pub avg_daily_spend: f64,
    /// Projected end-of-period spend
    pub projected_spend: f64,
    /// Whether over budget
    pub is_over_budget: bool,
    /// Number of alerts triggered
    pub alerts_count: u32,
}

/// Service for budget management
#[derive(Debug)]
pub struct BudgetManagementService {
    /// Mock budget data
    mock_budgets: Arc<Vec<MockBudgetData>>,
}

#[derive(Debug, Clone)]
struct MockBudgetData {
    id: String,
    tenant_id: String,
    name: String,
    amount_limit: f64,
    current_spend: f64,
    period: BudgetPeriod,
    currency: String,
    alerts_enabled: bool,
    alert_thresholds: Vec<AlertThreshold>,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    created_at: DateTime<Utc>,
    is_active: bool,
}

impl BudgetManagementService {
    /// Create new budget management service
    pub fn new() -> Self {
        let mock_budgets = Self::generate_mock_budgets();

        Self {
            mock_budgets: Arc::new(mock_budgets),
        }
    }

    /// Create a new budget
    pub async fn create_budget(&self, budget: Budget) -> Result<Budget, String> {
        info!("💰 Creating budget for tenant: {}", budget.tenant_id);

        // In production, this would save to database
        Ok(budget)
    }

    /// Update an existing budget
    pub async fn update_budget(&self, budget_id: &str, updates: Budget) -> Result<Budget, String> {
        info!("💰 Updating budget: {}", budget_id);

        // In production, this would update database
        Ok(updates)
    }

    /// Delete a budget
    pub async fn delete_budget(&self, budget_id: &str) -> Result<(), String> {
        info!("💰 Deleting budget: {}", budget_id);

        // In production, this would delete from database
        Ok(())
    }

    /// Get budget by ID
    pub async fn get_budget_by_id(&self, budget_id: &str) -> Option<Budget> {
        let budgets = self.mock_budgets.clone();

        for data in budgets.iter() {
            if data.id == budget_id {
                return Some(self.convert_to_budget(data));
            }
        }

        None
    }

    /// Get budget by tenant ID
    pub async fn get_budget_by_tenant(&self, tenant_id: &str) -> Option<Budget> {
        let budgets = self.mock_budgets.clone();

        for data in budgets.iter() {
            if data.tenant_id == tenant_id && data.is_active {
                return Some(self.convert_to_budget(data));
            }
        }

        None
    }

    /// List all budgets for a tenant
    pub async fn list_budgets(&self, tenant_id: Option<&str>) -> Vec<Budget> {
        let budgets = self.mock_budgets.clone();

        budgets
            .iter()
            .filter(|data| {
                if let Some(tenant) = tenant_id {
                    data.tenant_id == tenant
                } else {
                    true
                }
            })
            .map(|data| self.convert_to_budget(data))
            .collect()
    }

    /// Get budget usage for a tenant
    pub async fn get_budget_usage(&self, tenant_id: &str) -> Option<BudgetUsage> {
        let budgets = self.mock_budgets.clone();

        for data in budgets.iter() {
            if data.tenant_id == tenant_id && data.is_active {
                return Some(self.calculate_usage(&data));
            }
        }

        None
    }

    /// Get budget alerts for a tenant
    pub async fn get_budget_alerts(&self, tenant_id: &str) -> Vec<BudgetAlert> {
        let budgets = self.mock_budgets.clone();

        let mut alerts = Vec::new();

        for data in budgets.iter() {
            if data.tenant_id == tenant_id {
                let budget_alerts = self.generate_alerts_for_budget(data);
                alerts.extend(budget_alerts);
            }
        }

        alerts
    }

    /// Check and trigger budget alerts
    pub async fn check_budget_alerts(&self, tenant_id: &str) -> Vec<BudgetAlert> {
        info!("💰 Checking budget alerts for tenant: {}", tenant_id);

        self.get_budget_alerts(tenant_id).await
    }

    /// Calculate budget usage metrics
    fn calculate_usage(&self, data: &MockBudgetData) -> BudgetUsage {
        let percentage_used = if data.amount_limit > 0.0 {
            (data.current_spend / data.amount_limit) * 100.0
        } else {
            0.0
        };

        let remaining = data.amount_limit - data.current_spend;

        let period_duration = data.period_end - data.period_start;
        let days_remaining = period_duration.num_days() as u32;

        let avg_daily_spend = if days_remaining > 0 {
            data.current_spend / (period_duration.num_days() as f64)
        } else {
            0.0
        };

        let projected_spend = avg_daily_spend * (period_duration.num_days() as f64);

        BudgetUsage {
            budget_id: data.id.clone(),
            tenant_id: data.tenant_id.clone(),
            limit: data.amount_limit,
            current_spend: data.current_spend,
            percentage_used,
            remaining,
            days_remaining,
            avg_daily_spend,
            projected_spend,
            is_over_budget: data.current_spend > data.amount_limit,
            alerts_count: 0,
        }
    }

    /// Generate alerts for a budget based on thresholds
    fn generate_alerts_for_budget(&self, data: &MockBudgetData) -> Vec<BudgetAlert> {
        if !data.alerts_enabled {
            return Vec::new();
        }

        let mut alerts = Vec::new();
        let percentage_used = if data.amount_limit > 0.0 {
            (data.current_spend / data.amount_limit) * 100.0
        } else {
            0.0
        };

        for threshold in &data.alert_thresholds {
            let (threshold_pct, severity) = match threshold {
                AlertThreshold::FiftyPercent => (50.0, "info"),
                AlertThreshold::SeventyFivePercent => (75.0, "warning"),
                AlertThreshold::NinetyPercent => (90.0, "warning"),
                AlertThreshold::HundredPercent => (100.0, "critical"),
                AlertThreshold::Custom(pct) => (*pct, "info"),
            };

            if percentage_used >= threshold_pct {
                let alert = BudgetAlert {
                    id: format!("alert-{}-{}", data.id, threshold_pct as i32),
                    budget_id: data.id.clone(),
                    tenant_id: data.tenant_id.clone(),
                    threshold: threshold.clone(),
                    threshold_percentage: threshold_pct,
                    current_spend: data.current_spend,
                    budget_limit: data.amount_limit,
                    alert_type: format!("{}% threshold", threshold_pct),
                    message: format!(
                        "Budget '{}' has reached {:.1}% of limit (${:.2} of ${:.2})",
                        data.name, percentage_used, data.current_spend, data.amount_limit
                    ),
                    severity: severity.to_string(),
                    triggered_at: Utc::now(),
                    acknowledged: false,
                    acknowledged_at: None,
                };
                alerts.push(alert);
            }
        }

        alerts
    }

    /// Convert mock data to budget
    fn convert_to_budget(&self, data: &MockBudgetData) -> Budget {
        let now = Utc::now();

        Budget {
            id: data.id.clone(),
            tenant_id: data.tenant_id.clone(),
            name: data.name.clone(),
            amount_limit: data.amount_limit,
            current_spend: data.current_spend,
            period: data.period.clone(),
            currency: data.currency.clone(),
            alerts_enabled: data.alerts_enabled,
            alert_thresholds: data.alert_thresholds.clone(),
            period_start: data.period_start,
            period_end: data.period_end,
            created_at: data.created_at,
            updated_at: now,
            is_active: data.is_active,
        }
    }

    /// Generate mock budget data
    fn generate_mock_budgets() -> Vec<MockBudgetData> {
        let now = Utc::now();

        vec![
            MockBudgetData {
                id: "budget-001".to_string(),
                tenant_id: "tenant-123".to_string(),
                name: "Monthly Production Budget".to_string(),
                amount_limit: 10000.0,
                current_spend: 3750.0,
                period: BudgetPeriod::Monthly,
                currency: "USD".to_string(),
                alerts_enabled: true,
                alert_thresholds: vec![
                    AlertThreshold::FiftyPercent,
                    AlertThreshold::SeventyFivePercent,
                    AlertThreshold::NinetyPercent,
                    AlertThreshold::HundredPercent,
                ],
                period_start: now - chrono::Duration::days(10),
                period_end: now + chrono::Duration::days(20),
                created_at: now - chrono::Duration::days(30),
                is_active: true,
            },
            MockBudgetData {
                id: "budget-002".to_string(),
                tenant_id: "tenant-456".to_string(),
                name: "Development Environment Budget".to_string(),
                amount_limit: 5000.0,
                current_spend: 1200.0,
                period: BudgetPeriod::Monthly,
                currency: "USD".to_string(),
                alerts_enabled: true,
                alert_thresholds: vec![
                    AlertThreshold::SeventyFivePercent,
                    AlertThreshold::NinetyPercent,
                ],
                period_start: now - chrono::Duration::days(5),
                period_end: now + chrono::Duration::days(25),
                created_at: now - chrono::Duration::days(30),
                is_active: true,
            },
            MockBudgetData {
                id: "budget-003".to_string(),
                tenant_id: "tenant-789".to_string(),
                name: "Q4 Infrastructure Budget".to_string(),
                amount_limit: 25000.0,
                current_spend: 18500.0,
                period: BudgetPeriod::Quarterly,
                currency: "USD".to_string(),
                alerts_enabled: true,
                alert_thresholds: vec![
                    AlertThreshold::FiftyPercent,
                    AlertThreshold::SeventyFivePercent,
                    AlertThreshold::NinetyPercent,
                    AlertThreshold::HundredPercent,
                ],
                period_start: now - chrono::Duration::days(45),
                period_end: now + chrono::Duration::days(45),
                created_at: now - chrono::Duration::days(90),
                is_active: true,
            },
        ]
    }
}

impl Default for BudgetManagementService {
    fn default() -> Self {
        Self::new()
    }
}

/// Application state for Budget Management API
#[derive(Clone)]
pub struct BudgetManagementApiAppState {
    pub service: Arc<BudgetManagementService>,
}

/// GET /api/v1/budgets - List budgets
#[allow(dead_code)]
pub async fn list_budgets_handler(
    State(state): State<BudgetManagementApiAppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Budget>>, StatusCode> {
    info!("💰 Budget list requested");

    let tenant_id = params.get("tenant_id").map(|s| s.as_str());

    let budgets = state.service.list_budgets(tenant_id).await;

    info!("✅ Returned {} budgets", budgets.len());

    Ok(Json(budgets))
}

/// GET /api/v1/budgets/{id} - Get budget by ID
#[allow(dead_code)]
pub async fn get_budget_handler(
    State(state): State<BudgetManagementApiAppState>,
    Path(id): Path<String>,
) -> Result<Json<Budget>, StatusCode> {
    info!("💰 Budget {} requested", id);

    let budget = state.service.get_budget_by_id(&id).await;

    if let Some(budget) = budget {
        info!("✅ Budget found: {}", budget.name);
        Ok(Json(budget))
    } else {
        info!("❌ Budget not found: {}", id);
        Err(StatusCode::NOT_FOUND)
    }
}

/// POST /api/v1/budgets - Create new budget
#[allow(dead_code)]
pub async fn create_budget_handler(
    State(state): State<BudgetManagementApiAppState>,
    Json(budget): Json<Budget>,
) -> Result<Json<Budget>, StatusCode> {
    info!("💰 Creating budget: {}", budget.name);

    match state.service.create_budget(budget.clone()).await {
        Ok(created_budget) => {
            info!("✅ Budget created: {}", created_budget.name);
            Ok(Json(created_budget))
        }
        Err(e) => {
            info!("❌ Failed to create budget: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// PUT /api/v1/budgets/{id} - Update budget
#[allow(dead_code)]
pub async fn update_budget_handler(
    State(state): State<BudgetManagementApiAppState>,
    Path(id): Path<String>,
    Json(budget): Json<Budget>,
) -> Result<Json<Budget>, StatusCode> {
    info!("💰 Updating budget: {}", id);

    match state.service.update_budget(&id, budget).await {
        Ok(updated_budget) => {
            info!("✅ Budget updated: {}", updated_budget.name);
            Ok(Json(updated_budget))
        }
        Err(e) => {
            info!("❌ Failed to update budget: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /api/v1/budgets/{id} - Delete budget
#[allow(dead_code)]
pub async fn delete_budget_handler(
    State(state): State<BudgetManagementApiAppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    info!("💰 Deleting budget: {}", id);

    match state.service.delete_budget(&id).await {
        Ok(_) => {
            info!("✅ Budget deleted: {}", id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            info!("❌ Failed to delete budget: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v1/budgets/usage/{tenant_id} - Get budget usage
#[allow(dead_code)]
pub async fn get_budget_usage_handler(
    State(state): State<BudgetManagementApiAppState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<BudgetUsage>, StatusCode> {
    info!("💰 Budget usage requested for tenant: {}", tenant_id);

    let usage = state.service.get_budget_usage(&tenant_id).await;

    if let Some(usage) = usage {
        info!(
            "✅ Budget usage - Spend: ${:.2} ({:.1}%), Remaining: ${:.2}",
            usage.current_spend, usage.percentage_used, usage.remaining
        );
        Ok(Json(usage))
    } else {
        info!("❌ No active budget found for tenant: {}", tenant_id);
        Err(StatusCode::NOT_FOUND)
    }
}

/// GET /api/v1/budgets/alerts/{tenant_id} - Get budget alerts
#[allow(dead_code)]
pub async fn get_budget_alerts_handler(
    State(state): State<BudgetManagementApiAppState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<Vec<BudgetAlert>>, StatusCode> {
    info!("💰 Budget alerts requested for tenant: {}", tenant_id);

    let alerts = state.service.get_budget_alerts(&tenant_id).await;

    info!("✅ Returned {} budget alerts", alerts.len());

    Ok(Json(alerts))
}

/// POST /api/v1/budgets/check-alerts/{tenant_id} - Check and trigger budget alerts
#[allow(dead_code)]
pub async fn check_budget_alerts_handler(
    State(state): State<BudgetManagementApiAppState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<Vec<BudgetAlert>>, StatusCode> {
    info!("💰 Checking budget alerts for tenant: {}", tenant_id);

    let alerts = state.service.check_budget_alerts(&tenant_id).await;

    info!("✅ Triggered {} budget alerts", alerts.len());

    Ok(Json(alerts))
}

/// Budget Management API routes
pub fn budget_management_api_routes() -> Router<BudgetManagementApiAppState> {
    Router::new()
        .route("/budgets", get(list_budgets_handler))
        .route("/budgets/{id}", get(get_budget_handler))
        .route("/budgets", post(create_budget_handler))
        .route("/budgets/{id}", put(update_budget_handler))
        .route("/budgets/{id}", delete(delete_budget_handler))
        .route("/budgets/usage/{tenant_id}", get(get_budget_usage_handler))
        .route(
            "/budgets/alerts/{tenant_id}",
            get(get_budget_alerts_handler),
        )
        .route(
            "/budgets/check-alerts/{tenant_id}",
            post(check_budget_alerts_handler),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_budget_management_service_new() {
        let service = BudgetManagementService::new();
        assert!(!service.mock_budgets.is_empty());
    }

    #[tokio::test]
    async fn test_budget_management_service_get_budget_by_id() {
        let service = BudgetManagementService::new();

        let budget = service.get_budget_by_id("budget-001").await;

        assert!(budget.is_some());
        let budget = budget.unwrap();
        assert_eq!(budget.id, "budget-001");
        assert_eq!(budget.tenant_id, "tenant-123");
    }

    #[tokio::test]
    async fn test_budget_management_service_get_budget_by_tenant() {
        let service = BudgetManagementService::new();

        let budget = service.get_budget_by_tenant("tenant-123").await;

        assert!(budget.is_some());
        let budget = budget.unwrap();
        assert_eq!(budget.tenant_id, "tenant-123");
        assert!(budget.is_active);
    }

    #[tokio::test]
    async fn test_budget_management_service_list_budgets() {
        let service = BudgetManagementService::new();

        let budgets = service.list_budgets(None).await;

        assert!(!budgets.is_empty());
        assert_eq!(budgets.len(), 3);
    }

    #[tokio::test]
    async fn test_budget_management_service_get_budget_usage() {
        let service = BudgetManagementService::new();

        let usage = service.get_budget_usage("tenant-123").await;

        assert!(usage.is_some());
        let usage = usage.unwrap();
        assert_eq!(usage.tenant_id, "tenant-123");
        assert!(usage.percentage_used > 0.0);
        assert!(!usage.is_over_budget);
    }

    #[tokio::test]
    async fn test_budget_management_service_get_budget_alerts() {
        let service = BudgetManagementService::new();

        let alerts = service.get_budget_alerts("tenant-789").await;

        // tenant-789 has spent 74% of budget, should trigger 50% and 75% alerts
        assert!(!alerts.is_empty());
        for alert in &alerts {
            assert_eq!(alert.tenant_id, "tenant-789");
            assert!(alert.threshold_percentage <= 75.0);
        }
    }

    #[tokio::test]
    async fn test_budget_creation_and_update() {
        let service = BudgetManagementService::new();

        let budget = Budget {
            id: "budget-test".to_string(),
            tenant_id: "tenant-test".to_string(),
            name: "Test Budget".to_string(),
            amount_limit: 1000.0,
            current_spend: 0.0,
            period: BudgetPeriod::Monthly,
            currency: "USD".to_string(),
            alerts_enabled: true,
            alert_thresholds: vec![AlertThreshold::SeventyFivePercent],
            period_start: Utc::now(),
            period_end: Utc::now() + chrono::Duration::days(30),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_active: true,
        };

        let created = service.create_budget(budget.clone()).await;
        assert!(created.is_ok());

        let updated = service.update_budget("budget-test", budget).await;
        assert!(updated.is_ok());

        let deleted = service.delete_budget("budget-test").await;
        assert!(deleted.is_ok());
    }

    #[tokio::test]
    async fn test_budget_serialization() {
        let budget = Budget {
            id: "budget-test".to_string(),
            tenant_id: "tenant-test".to_string(),
            name: "Test Budget".to_string(),
            amount_limit: 1000.0,
            current_spend: 250.0,
            period: BudgetPeriod::Monthly,
            currency: "USD".to_string(),
            alerts_enabled: true,
            alert_thresholds: vec![AlertThreshold::SeventyFivePercent],
            period_start: Utc::now(),
            period_end: Utc::now() + chrono::Duration::days(30),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_active: true,
        };

        let json = serde_json::to_string(&budget).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("tenant_id"));
        assert!(json.contains("amount_limit"));

        let deserialized: Budget = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, budget.id);
        assert_eq!(deserialized.amount_limit, budget.amount_limit);
    }

    #[tokio::test]
    async fn test_budget_alert_serialization() {
        let alert = BudgetAlert {
            id: "alert-001".to_string(),
            budget_id: "budget-001".to_string(),
            tenant_id: "tenant-123".to_string(),
            threshold: AlertThreshold::SeventyFivePercent,
            threshold_percentage: 75.0,
            current_spend: 7500.0,
            budget_limit: 10000.0,
            alert_type: "75% threshold".to_string(),
            message: "Budget reached 75%".to_string(),
            severity: "warning".to_string(),
            triggered_at: Utc::now(),
            acknowledged: false,
            acknowledged_at: None,
        };

        let json = serde_json::to_string(&alert).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("budget_id"));
        assert!(json.contains("threshold_percentage"));

        let deserialized: BudgetAlert = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, alert.id);
        assert_eq!(
            deserialized.threshold_percentage,
            alert.threshold_percentage
        );
    }

    #[tokio::test]
    async fn test_budget_usage_serialization() {
        let usage = BudgetUsage {
            budget_id: "budget-001".to_string(),
            tenant_id: "tenant-123".to_string(),
            limit: 10000.0,
            current_spend: 2500.0,
            percentage_used: 25.0,
            remaining: 7500.0,
            days_remaining: 20,
            avg_daily_spend: 250.0,
            projected_spend: 7500.0,
            is_over_budget: false,
            alerts_count: 0,
        };

        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("budget_id"));
        assert!(json.contains("percentage_used"));
        assert!(json.contains("is_over_budget"));

        let deserialized: BudgetUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.budget_id, usage.budget_id);
        assert_eq!(deserialized.percentage_used, usage.percentage_used);
    }

    #[tokio::test]
    async fn test_alert_threshold_deserialization() {
        let json = r#"{"SeventyFivePercent":null}"#;
        let threshold: Result<AlertThreshold, _> = serde_json::from_str(json);
        assert!(threshold.is_ok());

        let json = r#"{"Custom":80.0}"#;
        let threshold: Result<AlertThreshold, _> = serde_json::from_str(json);
        assert!(threshold.is_ok());
        if let Ok(AlertThreshold::Custom(pct)) = threshold {
            assert_eq!(pct, 80.0);
        }
    }
}
