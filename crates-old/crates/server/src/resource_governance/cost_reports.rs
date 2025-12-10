//! Cost Reports API
//!
//! This module provides endpoints for generating and retrieving detailed cost reports
//! across resource pools, including daily, weekly, monthly, and custom date range reports.

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{error, info};

/// Cost period type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CostPeriod {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
    Custom,
}

/// Resource cost details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCost {
    pub resource_type: String,
    pub cost_amount: f64,
    pub usage_amount: f64,
    pub usage_unit: String,
    pub cost_per_unit: f64,
}

/// Cost by pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostByPool {
    pub pool_id: String,
    pub pool_name: Option<String>,
    pub total_cost: f64,
    pub resource_costs: Vec<ResourceCost>,
    pub percentage_of_total: f64,
}

/// Cost by time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostByPeriod {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub period_type: CostPeriod,
    pub total_cost: f64,
    pub cost_by_pool: Vec<CostByPool>,
}

/// Cost summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub total_cost: f64,
    pub previous_period_cost: f64,
    pub cost_change: f64,
    pub cost_change_percentage: f64,
    pub average_daily_cost: f64,
    pub projected_monthly_cost: f64,
    pub projected_annual_cost: f64,
}

/// Tag cost
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCost {
    pub tag_key: String,
    pub tag_value: String,
    pub cost: f64,
    pub percentage_of_total: f64,
}

/// Cost report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostReport {
    pub report_id: String,
    pub report_name: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub period_type: CostPeriod,
    pub generated_at: DateTime<Utc>,
    pub cost_by_period: Vec<CostByPeriod>,
    pub cost_by_pool: Vec<CostByPool>,
    pub cost_by_tag: Vec<TagCost>,
    pub cost_summary: CostSummary,
    pub currency: String,
    pub filters: HashMap<String, String>,
}

/// Create cost report request
#[derive(Debug, Deserialize)]
pub struct CreateCostReportRequest {
    pub report_name: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub period_type: String, // daily, weekly, monthly, quarterly, yearly, custom
    pub pool_filter: Option<Vec<String>>,
    pub tag_filter: Option<HashMap<String, String>>,
    pub currency: Option<String>,
}

/// Generate cost report response
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateCostReportResponse {
    pub report_id: String,
    pub report_name: String,
    pub total_cost: f64,
    pub generated_at: DateTime<Utc>,
    pub download_url: Option<String>,
}

/// Message response
#[derive(Debug, Serialize)]
pub struct CostReportMessageResponse {
    pub message: String,
}

/// Cost Reports Service
#[derive(Debug, Clone)]
pub struct CostReportsService {
    /// Stored cost reports
    cost_reports: Arc<RwLock<HashMap<String, CostReport>>>,
    /// Mock historical cost data
    mock_cost_data: Arc<RwLock<HashMap<String, Vec<(DateTime<Utc>, f64)>>>>,
}

impl CostReportsService {
    /// Create new Cost Reports Service
    pub fn new() -> Self {
        info!("Initializing Cost Reports Service");
        Self {
            cost_reports: Arc::new(RwLock::new(HashMap::new())),
            mock_cost_data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate cost report
    pub async fn generate_cost_report(
        &self,
        request: CreateCostReportRequest,
    ) -> Result<CostReport, String> {
        let period_type = match request.period_type.to_lowercase().as_str() {
            "daily" => CostPeriod::Daily,
            "weekly" => CostPeriod::Weekly,
            "monthly" => CostPeriod::Monthly,
            "quarterly" => CostPeriod::Quarterly,
            "yearly" => CostPeriod::Yearly,
            "custom" => CostPeriod::Custom,
            _ => return Err("Invalid period type".to_string()),
        };

        let report_id = uuid::Uuid::new_v4().to_string();
        let generated_at = Utc::now();

        // Generate mock cost data
        let cost_by_period = self
            .generate_cost_by_period(
                &request.start_date,
                &request.end_date,
                &period_type,
                request.pool_filter.as_deref(),
            )
            .await;

        let cost_by_pool = self
            .generate_cost_by_pool(
                &request.start_date,
                &request.end_date,
                request.pool_filter.as_deref(),
            )
            .await;

        let cost_by_tag = self
            .generate_cost_by_tag(&request.start_date, &request.end_date)
            .await;

        let cost_summary = self
            .calculate_cost_summary(
                &cost_by_period,
                &cost_by_pool,
                &request.start_date,
                &request.end_date,
            )
            .await;

        let report = CostReport {
            report_id: report_id.clone(),
            report_name: request.report_name,
            start_date: request.start_date,
            end_date: request.end_date,
            period_type,
            generated_at,
            cost_by_period,
            cost_by_pool,
            cost_by_tag,
            cost_summary,
            currency: request.currency.unwrap_or_else(|| "USD".to_string()),
            filters: request.tag_filter.unwrap_or_else(HashMap::new),
        };

        let report_total_cost = report.cost_summary.total_cost;
        let report_id_for_log = report_id.clone();

        // Store report
        let mut reports = self.cost_reports.write().await;
        reports.insert(report_id, report.clone());

        info!(
            "Generated cost report: {} (cost: ${:.2})",
            report_id_for_log, report_total_cost
        );

        Ok(report)
    }

    /// Get cost report
    pub async fn get_cost_report(&self, report_id: &str) -> Result<CostReport, String> {
        let reports = self.cost_reports.read().await;
        reports
            .get(report_id)
            .cloned()
            .ok_or_else(|| "Cost report not found".to_string())
    }

    /// List cost reports
    pub async fn list_cost_reports(&self) -> Vec<String> {
        let reports = self.cost_reports.read().await;
        reports.keys().cloned().collect()
    }

    /// Delete cost report
    pub async fn delete_cost_report(&self, report_id: &str) -> Result<(), String> {
        let mut reports = self.cost_reports.write().await;

        if !reports.contains_key(report_id) {
            return Err("Cost report not found".to_string());
        }

        reports.remove(report_id);

        info!("Deleted cost report: {}", report_id);

        Ok(())
    }

    /// Generate cost by period
    async fn generate_cost_by_period(
        &self,
        start_date: &DateTime<Utc>,
        end_date: &DateTime<Utc>,
        period_type: &CostPeriod,
        pool_filter: Option<&[String]>,
    ) -> Vec<CostByPeriod> {
        let mut periods = Vec::new();
        let mut current = *start_date;

        while current < *end_date {
            let period_end = match period_type {
                CostPeriod::Daily => current + chrono::Duration::days(1),
                CostPeriod::Weekly => current + chrono::Duration::days(7),
                CostPeriod::Monthly => current + chrono::Duration::days(30),
                CostPeriod::Quarterly => current + chrono::Duration::days(90),
                CostPeriod::Yearly => current + chrono::Duration::days(365),
                CostPeriod::Custom => {
                    let duration = *end_date - current;
                    if duration.num_days() < 30 {
                        current + chrono::Duration::days(duration.num_days())
                    } else {
                        current + chrono::Duration::days(30)
                    }
                }
            }
            .min(*end_date);

            let period_cost = self
                .calculate_period_cost(&current, &period_end, pool_filter)
                .await;
            let cost_by_pool = self
                .generate_cost_by_pool(&current, &period_end, pool_filter)
                .await;

            periods.push(CostByPeriod {
                period_start: current,
                period_end,
                period_type: period_type.clone(),
                total_cost: period_cost,
                cost_by_pool,
            });

            current = period_end;
        }

        periods
    }

    /// Calculate period cost
    async fn calculate_period_cost(
        &self,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
        pool_filter: Option<&[String]>,
    ) -> f64 {
        // Mock calculation based on period duration
        let days = (end.signed_duration_since(*start)).num_days();
        let base_daily_cost = if let Some(pools) = pool_filter {
            pools.len() as f64 * 50.0 // $50 per pool per day
        } else {
            500.0 // Default total daily cost
        };

        base_daily_cost * days as f64
    }

    /// Generate cost by pool
    async fn generate_cost_by_pool(
        &self,
        start_date: &DateTime<Utc>,
        end_date: &DateTime<Utc>,
        pool_filter: Option<&[String]>,
    ) -> Vec<CostByPool> {
        let pools = if let Some(filter) = pool_filter {
            filter
        } else {
            &[
                "pool-1".to_string(),
                "pool-2".to_string(),
                "pool-3".to_string(),
            ]
        };
        let days = (end_date.signed_duration_since(*start_date)).num_days();
        let total_cost = (pools.len() as f64 * 50.0) * days as f64;

        pools
            .iter()
            .map(|pool_id| {
                let hash = pool_id.chars().map(|c| c as u64).sum::<u64>();
                let pool_cost = (50.0 + (hash % 100) as f64) * days as f64;

                CostByPool {
                    pool_id: pool_id.clone(),
                    pool_name: Some(
                        format!("Pool {}", pool_id.chars().last().unwrap_or('?')).to_string(),
                    ),
                    total_cost: pool_cost,
                    resource_costs: vec![
                        ResourceCost {
                            resource_type: "compute".to_string(),
                            cost_amount: pool_cost * 0.6,
                            usage_amount: pool_cost * 0.6 / 0.05,
                            usage_unit: "instance-hours".to_string(),
                            cost_per_unit: 0.05,
                        },
                        ResourceCost {
                            resource_type: "storage".to_string(),
                            cost_amount: pool_cost * 0.2,
                            usage_amount: pool_cost * 0.2 / 0.1,
                            usage_unit: "GB-month".to_string(),
                            cost_per_unit: 0.1,
                        },
                        ResourceCost {
                            resource_type: "network".to_string(),
                            cost_amount: pool_cost * 0.15,
                            usage_amount: pool_cost * 0.15 / 0.09,
                            usage_unit: "GB".to_string(),
                            cost_per_unit: 0.09,
                        },
                        ResourceCost {
                            resource_type: "other".to_string(),
                            cost_amount: pool_cost * 0.05,
                            usage_amount: pool_cost * 0.05,
                            usage_unit: "misc".to_string(),
                            cost_per_unit: 1.0,
                        },
                    ],
                    percentage_of_total: if total_cost > 0.0 {
                        (pool_cost / total_cost) * 100.0
                    } else {
                        0.0
                    },
                }
            })
            .collect()
    }

    /// Generate cost by tag
    async fn generate_cost_by_tag(
        &self,
        _start_date: &DateTime<Utc>,
        _end_date: &DateTime<Utc>,
    ) -> Vec<TagCost> {
        let tags = vec![
            ("environment".to_string(), "production".to_string(), 800.0),
            ("environment".to_string(), "staging".to_string(), 300.0),
            ("team".to_string(), "data-engineering".to_string(), 600.0),
            ("team".to_string(), "ml-platform".to_string(), 500.0),
            ("project".to_string(), "analytics".to_string(), 450.0),
            ("project".to_string(), "recommendations".to_string(), 350.0),
        ];

        let total_cost = 1100.0;

        tags.into_iter()
            .map(|(key, value, cost)| TagCost {
                tag_key: key,
                tag_value: value,
                cost,
                percentage_of_total: (cost / total_cost) * 100.0,
            })
            .collect()
    }

    /// Calculate cost summary
    async fn calculate_cost_summary(
        &self,
        cost_by_period: &[CostByPeriod],
        _cost_by_pool: &[CostByPool],
        start_date: &DateTime<Utc>,
        end_date: &DateTime<Utc>,
    ) -> CostSummary {
        let total_cost: f64 = cost_by_period.iter().map(|p| p.total_cost).sum();
        let days = (end_date.signed_duration_since(*start_date)).num_days() as f64;
        let average_daily_cost = if days > 0.0 { total_cost / days } else { 0.0 };
        let projected_monthly_cost = average_daily_cost * 30.0;
        let projected_annual_cost = average_daily_cost * 365.0;

        // Mock previous period for comparison
        let previous_period_cost = total_cost * 0.85; // Assume 15% decrease
        let cost_change = total_cost - previous_period_cost;
        let cost_change_percentage = if previous_period_cost > 0.0 {
            (cost_change / previous_period_cost) * 100.0
        } else {
            0.0
        };

        CostSummary {
            total_cost,
            previous_period_cost,
            cost_change,
            cost_change_percentage,
            average_daily_cost,
            projected_monthly_cost,
            projected_annual_cost,
        }
    }
}

/// Application state for Cost Reports
#[derive(Clone)]
pub struct CostReportsAppState {
    pub service: Arc<CostReportsService>,
}

/// Create router for Cost Reports API
pub fn cost_reports_routes() -> Router<CostReportsAppState> {
    Router::new()
        .route("/cost-reports", post(generate_cost_report_handler))
        .route("/cost-reports", get(list_cost_reports_handler))
        .route("/cost-reports/:report_id", get(get_cost_report_handler))
        .route(
            "/cost-reports/:report_id",
            delete(delete_cost_report_handler),
        )
}

/// Generate cost report handler
async fn generate_cost_report_handler(
    State(state): State<CostReportsAppState>,
    Json(payload): Json<CreateCostReportRequest>,
) -> Result<Json<GenerateCostReportResponse>, StatusCode> {
    match state.service.generate_cost_report(payload).await {
        Ok(report) => {
            let report_id = report.report_id.clone();
            Ok(Json(GenerateCostReportResponse {
                report_id,
                report_name: report.report_name,
                total_cost: report.cost_summary.total_cost,
                generated_at: report.generated_at,
                download_url: Some(format!(
                    "/api/v1/cost-reports/{}/download",
                    report.report_id
                )),
            }))
        }
        Err(e) => {
            error!("Failed to generate cost report: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Get cost report handler
async fn get_cost_report_handler(
    State(state): State<CostReportsAppState>,
    axum::extract::Path(report_id): axum::extract::Path<String>,
) -> Result<Json<CostReport>, StatusCode> {
    match state.service.get_cost_report(&report_id).await {
        Ok(report) => Ok(Json(report)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// List cost reports handler
async fn list_cost_reports_handler(
    State(state): State<CostReportsAppState>,
) -> Result<Json<Vec<String>>, StatusCode> {
    Ok(Json(state.service.list_cost_reports().await))
}

/// Delete cost report handler
async fn delete_cost_report_handler(
    State(state): State<CostReportsAppState>,
    axum::extract::Path(report_id): axum::extract::Path<String>,
) -> Result<Json<CostReportMessageResponse>, StatusCode> {
    match state.service.delete_cost_report(&report_id).await {
        Ok(_) => Ok(Json(CostReportMessageResponse {
            message: format!("Cost report {} deleted successfully", report_id),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_cost_report() {
        let service = CostReportsService::new();

        let request = CreateCostReportRequest {
            report_name: "Monthly Cost Report".to_string(),
            start_date: Utc::now() - chrono::Duration::days(30),
            end_date: Utc::now(),
            period_type: "monthly".to_string(),
            pool_filter: Some(vec!["pool-1".to_string()]),
            tag_filter: None,
            currency: Some("USD".to_string()),
        };

        let result = service.generate_cost_report(request).await;
        assert!(result.is_ok());

        let report = result.unwrap();
        assert_eq!(report.report_name, "Monthly Cost Report");
        assert!(!report.cost_by_period.is_empty());
        assert!(report.cost_summary.total_cost > 0.0);
    }

    #[tokio::test]
    async fn test_get_cost_report() {
        let service = CostReportsService::new();

        let request = CreateCostReportRequest {
            report_name: "Test Report".to_string(),
            start_date: Utc::now() - chrono::Duration::days(7),
            end_date: Utc::now(),
            period_type: "weekly".to_string(),
            pool_filter: None,
            tag_filter: None,
            currency: None,
        };

        let report = service.generate_cost_report(request).await.unwrap();

        let result = service.get_cost_report(&report.report_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().report_id, report.report_id);
    }

    #[tokio::test]
    async fn test_list_cost_reports() {
        let service = CostReportsService::new();

        let request = CreateCostReportRequest {
            report_name: "Test Report".to_string(),
            start_date: Utc::now() - chrono::Duration::days(7),
            end_date: Utc::now(),
            period_type: "weekly".to_string(),
            pool_filter: None,
            tag_filter: None,
            currency: None,
        };

        service.generate_cost_report(request).await.unwrap();

        let reports = service.list_cost_reports().await;
        assert_eq!(reports.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_cost_report() {
        let service = CostReportsService::new();

        let request = CreateCostReportRequest {
            report_name: "Test Report".to_string(),
            start_date: Utc::now() - chrono::Duration::days(7),
            end_date: Utc::now(),
            period_type: "weekly".to_string(),
            pool_filter: None,
            tag_filter: None,
            currency: None,
        };

        let report = service.generate_cost_report(request).await.unwrap();

        let result = service.delete_cost_report(&report.report_id).await;
        assert!(result.is_ok());

        let reports = service.list_cost_reports().await;
        assert!(reports.is_empty());
    }

    #[tokio::test]
    async fn test_invalid_period_type() {
        let service = CostReportsService::new();

        let request = CreateCostReportRequest {
            report_name: "Test Report".to_string(),
            start_date: Utc::now() - chrono::Duration::days(7),
            end_date: Utc::now(),
            period_type: "invalid_type".to_string(),
            pool_filter: None,
            tag_filter: None,
            currency: None,
        };

        let result = service.generate_cost_report(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid period type"));
    }
}
