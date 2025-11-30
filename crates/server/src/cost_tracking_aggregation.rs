//! Cost Tracking & Aggregation Module
//!
//! Provides APIs for tracking and aggregating costs across resources and tenants.
//! Implements US-015 from the Epic Web Frontend Production Ready.

use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
};
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Cost summary structure
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CostSummary {
    /// Total cost for the period
    pub total_cost: f64,
    /// Period start timestamp
    pub period_start: DateTime<Utc>,
    /// Period end timestamp
    pub period_end: DateTime<Utc>,
    /// Currency code (USD, EUR, etc.)
    pub currency: String,
    /// Cost breakdown by resource type
    pub breakdown_by_resource: HashMap<String, f64>,
    /// Cost breakdown by tenant
    pub breakdown_by_tenant: HashMap<String, f64>,
}

/// Cost breakdown by resource type
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CostBreakdown {
    /// Resource type (compute, storage, network, etc.)
    pub resource_type: String,
    /// Total cost for this resource type
    pub cost: f64,
    /// Usage quantity
    pub usage_quantity: f64,
    /// Unit of measurement (GB, GB-hours, etc.)
    pub unit: String,
    /// Cost per unit
    pub cost_per_unit: f64,
    /// Period start timestamp
    pub period_start: DateTime<Utc>,
    /// Period end timestamp
    pub period_end: DateTime<Utc>,
}

/// Cost breakdown by tenant
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TenantCostBreakdown {
    /// Tenant ID
    pub tenant_id: String,
    /// Total cost for this tenant
    pub total_cost: f64,
    /// Breakdown by resource type
    pub resource_breakdown: HashMap<String, f64>,
    /// Period start timestamp
    pub period_start: DateTime<Utc>,
    /// Period end timestamp
    pub period_end: DateTime<Utc>,
}

/// Cost trend data point
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CostTrend {
    /// Date of the trend data
    pub date: DateTime<Utc>,
    /// Total cost for this date
    pub total_cost: f64,
    /// Compute cost for this date
    pub compute_cost: f64,
    /// Storage cost for this date
    pub storage_cost: f64,
    /// Network cost for this date
    pub network_cost: f64,
}

/// Service for cost tracking and aggregation
#[derive(Debug)]
pub struct CostTrackingService {
    /// Mock cost data for demonstration
    mock_costs: Arc<Vec<MockCostData>>,
}

#[derive(Debug, Clone)]
struct MockCostData {
    tenant_id: String,
    resource_type: String,
    date: DateTime<Utc>,
    cost: f64,
    usage_quantity: f64,
    unit: String,
}

impl CostTrackingService {
    /// Create new cost tracking service
    pub fn new() -> Self {
        let mock_costs = Self::generate_mock_cost_data();

        Self {
            mock_costs: Arc::new(mock_costs),
        }
    }

    /// Get cost summary
    pub async fn get_cost_summary(
        &self,
        tenant_id: Option<&str>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        currency: Option<&str>,
    ) -> CostSummary {
        let costs = self.get_filtered_costs(tenant_id, start_date, end_date);

        let total_cost = costs.iter().map(|c| c.cost).sum::<f64>();

        let mut breakdown_by_resource = HashMap::new();
        for cost in &costs {
            *breakdown_by_resource
                .entry(cost.resource_type.clone())
                .or_insert(0.0) += cost.cost;
        }

        let mut breakdown_by_tenant = HashMap::new();
        for cost in &costs {
            *breakdown_by_tenant
                .entry(cost.tenant_id.clone())
                .or_insert(0.0) += cost.cost;
        }

        let default_date = Utc::now().date_naive();
        let period_start = start_date.unwrap_or_else(|| {
            Utc.with_ymd_and_hms(
                default_date.year(),
                default_date.month(),
                default_date.day(),
                0,
                0,
                0,
            )
            .unwrap()
        });
        let period_end = end_date.unwrap_or_else(|| {
            Utc.with_ymd_and_hms(
                default_date.year(),
                default_date.month(),
                default_date.day(),
                23,
                59,
                59,
            )
            .unwrap()
        });

        CostSummary {
            total_cost,
            period_start,
            period_end,
            currency: currency.unwrap_or("USD").to_string(),
            breakdown_by_resource,
            breakdown_by_tenant,
        }
    }

    /// Get cost breakdown by resource type
    pub async fn get_cost_by_resource(
        &self,
        tenant_id: Option<&str>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Vec<CostBreakdown> {
        let costs = self.get_filtered_costs(tenant_id, start_date, end_date);

        let mut resource_costs = HashMap::new();
        for cost in &costs {
            let entry = resource_costs
                .entry(cost.resource_type.clone())
                .or_insert(MockCostData {
                    tenant_id: "all".to_string(),
                    resource_type: cost.resource_type.clone(),
                    date: cost.date,
                    cost: 0.0,
                    usage_quantity: 0.0,
                    unit: cost.unit.clone(),
                });
            entry.cost += cost.cost;
            entry.usage_quantity += cost.usage_quantity;
        }

        let default_date = Utc::now().date_naive();
        let period_start = start_date.unwrap_or_else(|| {
            Utc.with_ymd_and_hms(
                default_date.year(),
                default_date.month(),
                default_date.day(),
                0,
                0,
                0,
            )
            .unwrap()
        });
        let period_end = end_date.unwrap_or_else(|| {
            Utc.with_ymd_and_hms(
                default_date.year(),
                default_date.month(),
                default_date.day(),
                23,
                59,
                59,
            )
            .unwrap()
        });

        resource_costs
            .into_values()
            .map(|data| CostBreakdown {
                resource_type: data.resource_type,
                cost: data.cost,
                usage_quantity: data.usage_quantity,
                unit: data.unit,
                cost_per_unit: if data.usage_quantity > 0.0 {
                    data.cost / data.usage_quantity
                } else {
                    0.0
                },
                period_start,
                period_end,
            })
            .collect()
    }

    /// Get cost breakdown by tenant
    pub async fn get_cost_by_tenant(
        &self,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Vec<TenantCostBreakdown> {
        let costs = self.get_filtered_costs(None, start_date, end_date);

        let mut tenant_costs = HashMap::new();
        for cost in &costs {
            let entry = tenant_costs
                .entry(cost.tenant_id.clone())
                .or_insert_with(|| {
                    (
                        HashMap::new(), // resource_breakdown
                        0.0f64,         // total_cost
                    )
                });

            *entry.0.entry(cost.resource_type.clone()).or_insert(0.0) += cost.cost;
            entry.1 += cost.cost;
        }

        let default_date = Utc::now().date_naive();
        let period_start = start_date.unwrap_or_else(|| {
            Utc.with_ymd_and_hms(
                default_date.year(),
                default_date.month(),
                default_date.day(),
                0,
                0,
                0,
            )
            .unwrap()
        });
        let period_end = end_date.unwrap_or_else(|| {
            Utc.with_ymd_and_hms(
                default_date.year(),
                default_date.month(),
                default_date.day(),
                23,
                59,
                59,
            )
            .unwrap()
        });

        tenant_costs
            .into_iter()
            .map(
                |(tenant_id, (resource_breakdown, total_cost))| TenantCostBreakdown {
                    tenant_id,
                    total_cost,
                    resource_breakdown,
                    period_start,
                    period_end,
                },
            )
            .collect()
    }

    /// Get cost trends over time
    pub async fn get_cost_trends(
        &self,
        tenant_id: Option<&str>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Vec<CostTrend> {
        let costs = self.get_filtered_costs(tenant_id, start_date, end_date);

        let mut daily_costs = HashMap::new();
        for cost in &costs {
            let date_only = cost.date.date_naive();
            let entry = daily_costs.entry(date_only).or_insert(MockDailyCost {
                total: 0.0,
                compute: 0.0,
                storage: 0.0,
                network: 0.0,
            });

            entry.total += cost.cost;
            match cost.resource_type.as_str() {
                "compute" => entry.compute += cost.cost,
                "storage" => entry.storage += cost.cost,
                "network" => entry.network += cost.cost,
                _ => {}
            }
        }

        let mut trends: Vec<_> = daily_costs
            .into_iter()
            .map(|(date, data)| {
                let datetime = Utc
                    .with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
                    .unwrap();
                CostTrend {
                    date: datetime,
                    total_cost: data.total,
                    compute_cost: data.compute,
                    storage_cost: data.storage,
                    network_cost: data.network,
                }
            })
            .collect();

        trends.sort_by(|a, b| a.date.cmp(&b.date));
        trends
    }

    /// Get filtered cost data
    fn get_filtered_costs(
        &self,
        tenant_id: Option<&str>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Vec<MockCostData> {
        let costs = self.mock_costs.clone();

        costs
            .iter()
            .filter(|cost| {
                if let Some(tenant) = tenant_id {
                    if cost.tenant_id != tenant {
                        return false;
                    }
                }

                if let Some(start) = start_date {
                    if cost.date < start {
                        return false;
                    }
                }

                if let Some(end) = end_date {
                    if cost.date > end {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect()
    }

    /// Generate mock cost data for demonstration
    fn generate_mock_cost_data() -> Vec<MockCostData> {
        use rand::Rng;
        let mut rng = rand::rng();

        let tenants = vec!["tenant-123", "tenant-456", "tenant-789"];
        let resource_types = vec!["compute", "storage", "network"];
        let units = vec!["GB-hours", "GB", "GB-transferred"];

        let mut costs = Vec::new();
        let today = Utc::now();

        // Generate 30 days of mock cost data
        for day_offset in 0..30 {
            let date = today - chrono::Duration::days(day_offset);

            for tenant in &tenants {
                for (i, resource_type) in resource_types.iter().enumerate() {
                    let unit = units[i];
                    let base_cost = match *resource_type {
                        "compute" => 100.0,
                        "storage" => 50.0,
                        "network" => 25.0,
                        _ => 10.0,
                    };

                    // Add some randomness
                    let cost = base_cost
                        * (0.8 + rng.random_range(0.0..0.4))
                        * (1.0 + rng.random_range(0.0..0.5) * day_offset as f64 / 30.0);

                    let usage_quantity = cost * rng.random_range(1.5..2.5);

                    costs.push(MockCostData {
                        tenant_id: tenant.to_string(),
                        resource_type: resource_type.to_string(),
                        date,
                        cost: cost as f64,
                        usage_quantity,
                        unit: unit.to_string(),
                    });
                }
            }
        }

        costs
    }
}

#[derive(Debug, Clone, Default)]
struct MockDailyCost {
    total: f64,
    compute: f64,
    storage: f64,
    network: f64,
}

impl Default for CostTrackingService {
    fn default() -> Self {
        Self::new()
    }
}

/// Application state for Cost Tracking API
#[derive(Clone)]
pub struct CostTrackingApiAppState {
    pub service: Arc<CostTrackingService>,
}

/// GET /api/v1/costs/summary - Get cost summary
#[utoipa::path(
    get,
    path = "/api/v1/costs/summary",
    params(
        ("tenant_id" = Option<String>, Query, description = "Filter by tenant ID"),
        ("start_date" = Option<String>, Query, description = "Start date (YYYY-MM-DD)"),
        ("end_date" = Option<String>, Query, description = "End date (YYYY-MM-DD)"),
        ("currency" = Option<String>, Query, description = "Currency code (e.g., USD, EUR)")
    ),
    responses(
        (status = 200, description = "Cost summary", body = CostSummary),
        (status = 500, description = "Internal server error")
    ),
    tag = "cost-management"
)]
pub async fn get_cost_summary_handler(
    State(state): State<CostTrackingApiAppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<CostSummary>, StatusCode> {
    info!("💰 Cost summary requested");

    let tenant_id = params.get("tenant_id").map(|s| s.as_str());
    let currency = params.get("currency").map(|s| s.as_str());

    let start_date = params.get("start_date").and_then(|s| {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .map(|d| {
                Utc.with_ymd_and_hms(d.year(), d.month(), d.day(), 0, 0, 0)
                    .unwrap()
            })
    });

    let end_date = params.get("end_date").and_then(|s| {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .map(|d| {
                Utc.with_ymd_and_hms(d.year(), d.month(), d.day(), 23, 59, 59)
                    .unwrap()
            })
    });

    let summary = state
        .service
        .get_cost_summary(tenant_id, start_date, end_date, currency)
        .await;

    info!(
        "✅ Cost summary returned - Total: {:.2} {}",
        summary.total_cost, summary.currency
    );

    Ok(Json(summary))
}

/// GET /api/v1/costs/by-resource - Get cost breakdown by resource type
#[utoipa::path(
    get,
    path = "/api/v1/costs/by-resource",
    params(
        ("tenant_id" = Option<String>, Query, description = "Filter by tenant ID"),
        ("start_date" = Option<String>, Query, description = "Start date (YYYY-MM-DD)"),
        ("end_date" = Option<String>, Query, description = "End date (YYYY-MM-DD)")
    ),
    responses(
        (status = 200, description = "Cost breakdown by resource", body = [CostBreakdown]),
        (status = 500, description = "Internal server error")
    ),
    tag = "cost-management"
)]
pub async fn get_cost_by_resource_handler(
    State(state): State<CostTrackingApiAppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<CostBreakdown>>, StatusCode> {
    info!("💰 Cost by resource requested");

    let tenant_id = params.get("tenant_id").map(|s| s.as_str());

    let start_date = params.get("start_date").and_then(|s| {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .map(|d| {
                Utc.with_ymd_and_hms(d.year(), d.month(), d.day(), 0, 0, 0)
                    .unwrap()
            })
    });

    let end_date = params.get("end_date").and_then(|s| {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .map(|d| {
                Utc.with_ymd_and_hms(d.year(), d.month(), d.day(), 23, 59, 59)
                    .unwrap()
            })
    });

    let breakdowns = state
        .service
        .get_cost_by_resource(tenant_id, start_date, end_date)
        .await;

    info!(
        "✅ Cost by resource returned - {} resource types",
        breakdowns.len()
    );

    Ok(Json(breakdowns))
}

/// GET /api/v1/costs/by-tenant - Get cost breakdown by tenant
#[utoipa::path(
    get,
    path = "/api/v1/costs/by-tenant",
    params(
        ("start_date" = Option<String>, Query, description = "Start date (YYYY-MM-DD)"),
        ("end_date" = Option<String>, Query, description = "End date (YYYY-MM-DD)")
    ),
    responses(
        (status = 200, description = "Cost breakdown by tenant", body = [TenantCostBreakdown]),
        (status = 500, description = "Internal server error")
    ),
    tag = "cost-management"
)]
pub async fn get_cost_by_tenant_handler(
    State(state): State<CostTrackingApiAppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<TenantCostBreakdown>>, StatusCode> {
    info!("💰 Cost by tenant requested");

    let start_date = params.get("start_date").and_then(|s| {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .map(|d| {
                Utc.with_ymd_and_hms(d.year(), d.month(), d.day(), 0, 0, 0)
                    .unwrap()
            })
    });

    let end_date = params.get("end_date").and_then(|s| {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .map(|d| {
                Utc.with_ymd_and_hms(d.year(), d.month(), d.day(), 23, 59, 59)
                    .unwrap()
            })
    });

    let breakdowns = state.service.get_cost_by_tenant(start_date, end_date).await;

    info!("✅ Cost by tenant returned - {} tenants", breakdowns.len());

    Ok(Json(breakdowns))
}

/// GET /api/v1/costs/trends - Get cost trends
#[utoipa::path(
    get,
    path = "/api/v1/costs/trends",
    params(
        ("tenant_id" = Option<String>, Query, description = "Filter by tenant ID"),
        ("start_date" = Option<String>, Query, description = "Start date (YYYY-MM-DD)"),
        ("end_date" = Option<String>, Query, description = "End date (YYYY-MM-DD)")
    ),
    responses(
        (status = 200, description = "Cost trends", body = [CostTrend]),
        (status = 500, description = "Internal server error")
    ),
    tag = "cost-management"
)]
pub async fn get_cost_trends_handler(
    State(state): State<CostTrackingApiAppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<CostTrend>>, StatusCode> {
    info!("💰 Cost trends requested");

    let tenant_id = params.get("tenant_id").map(|s| s.as_str());

    let start_date = params.get("start_date").and_then(|s| {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .map(|d| {
                Utc.with_ymd_and_hms(d.year(), d.month(), d.day(), 0, 0, 0)
                    .unwrap()
            })
    });

    let end_date = params.get("end_date").and_then(|s| {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .map(|d| {
                Utc.with_ymd_and_hms(d.year(), d.month(), d.day(), 23, 59, 59)
                    .unwrap()
            })
    });

    let trends = state
        .service
        .get_cost_trends(tenant_id, start_date, end_date)
        .await;

    info!("✅ Cost trends returned - {} data points", trends.len());

    Ok(Json(trends))
}

/// Cost Tracking API routes
pub fn cost_tracking_api_routes() -> Router<CostTrackingApiAppState> {
    Router::new()
        .route("/summary", get(get_cost_summary_handler))
        .route("/by-resource", get(get_cost_by_resource_handler))
        .route("/by-tenant", get(get_cost_by_tenant_handler))
        .route("/trends", get(get_cost_trends_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cost_tracking_service_new() {
        let service = CostTrackingService::new();
        assert!(!service.mock_costs.is_empty());
    }

    #[tokio::test]
    async fn test_cost_tracking_service_get_cost_summary() {
        let service = CostTrackingService::new();

        let summary = service.get_cost_summary(None, None, None, None).await;

        assert!(summary.total_cost > 0.0);
        assert!(!summary.breakdown_by_resource.is_empty());
        assert!(!summary.breakdown_by_tenant.is_empty());
    }

    #[tokio::test]
    async fn test_cost_tracking_service_get_cost_by_resource() {
        let service = CostTrackingService::new();

        let breakdowns = service.get_cost_by_resource(None, None, None).await;

        assert!(!breakdowns.is_empty());
        for breakdown in &breakdowns {
            assert!(breakdown.cost > 0.0);
            assert!(breakdown.cost_per_unit >= 0.0);
        }
    }

    #[tokio::test]
    async fn test_cost_tracking_service_get_cost_by_tenant() {
        let service = CostTrackingService::new();

        let breakdowns = service.get_cost_by_tenant(None, None).await;

        assert!(!breakdowns.is_empty());
        for breakdown in &breakdowns {
            assert!(breakdown.total_cost > 0.0);
            assert!(!breakdown.resource_breakdown.is_empty());
        }
    }

    #[tokio::test]
    async fn test_cost_tracking_service_get_cost_trends() {
        let service = CostTrackingService::new();

        let trends = service.get_cost_trends(None, None, None).await;

        assert!(!trends.is_empty());
        for trend in &trends {
            assert!(trend.total_cost > 0.0);
            assert!(trend.compute_cost >= 0.0);
            assert!(trend.storage_cost >= 0.0);
            assert!(trend.network_cost >= 0.0);
        }
    }

    #[tokio::test]
    async fn test_cost_summary_serialization() {
        let summary = CostSummary {
            total_cost: 1250.50,
            period_start: Utc::now(),
            period_end: Utc::now(),
            currency: "USD".to_string(),
            breakdown_by_resource: HashMap::new(),
            breakdown_by_tenant: HashMap::new(),
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("total_cost"));
        assert!(json.contains("currency"));

        let deserialized: CostSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_cost, summary.total_cost);
    }

    #[tokio::test]
    async fn test_cost_breakdown_serialization() {
        let breakdown = CostBreakdown {
            resource_type: "compute".to_string(),
            cost: 800.0,
            usage_quantity: 1200.0,
            unit: "GB-hours".to_string(),
            cost_per_unit: 0.6667,
            period_start: Utc::now(),
            period_end: Utc::now(),
        };

        let json = serde_json::to_string(&breakdown).unwrap();
        assert!(json.contains("resource_type"));
        assert!(json.contains("cost"));

        let deserialized: CostBreakdown = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.resource_type, breakdown.resource_type);
    }

    #[tokio::test]
    async fn test_cost_trend_serialization() {
        let trend = CostTrend {
            date: Utc::now(),
            total_cost: 42.50,
            compute_cost: 28.0,
            storage_cost: 10.0,
            network_cost: 4.50,
        };

        let json = serde_json::to_string(&trend).unwrap();
        assert!(json.contains("date"));
        assert!(json.contains("total_cost"));

        let deserialized: CostTrend = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.date, trend.date);
    }
}
