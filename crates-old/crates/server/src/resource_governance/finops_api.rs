use axum::{Json, Router, extract::State, response::IntoResponse, routing::get};
use chrono::{Datelike, Utc};
use hodei_pipelines_adapters::MetricsTimeseriesRepository;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// DTOs matching Frontend types
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FinOpsMetrics {
    pub metrics: CostMetrics,
    pub breakdown: Vec<CostBreakdown>,
    pub history: Vec<CostHistory>,
    pub alerts: Vec<BudgetAlert>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CostMetrics {
    #[serde(rename = "totalCost")]
    pub total_cost: f64,
    #[serde(rename = "budgetLimit")]
    pub budget_limit: f64,
    #[serde(rename = "budgetUsedPercentage")]
    pub budget_used_percentage: f64,
    #[serde(rename = "projectedCost")]
    pub projected_cost: f64,
    #[serde(rename = "savingsPotential")]
    pub savings_potential: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CostBreakdown {
    #[serde(rename = "pipelineId")]
    pub pipeline_id: String,
    #[serde(rename = "pipelineName")]
    pub pipeline_name: String,
    pub cost: f64,
    pub percentage: f64,
    #[serde(rename = "jobCount")]
    pub job_count: i32,
    #[serde(rename = "avgDuration")]
    pub avg_duration: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CostHistory {
    pub date: String,
    pub cost: f64,
    pub budget: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BudgetAlert {
    pub id: String,
    pub severity: String, // "info" | "warning" | "critical"
    pub message: String,
    pub timestamp: String,
}

#[derive(Clone)]
pub struct FinOpsApiAppState {
    pub metrics_repo: Arc<MetricsTimeseriesRepository>,
}

pub fn finops_api_routes(state: FinOpsApiAppState) -> Router {
    Router::new()
        .route("/metrics", get(get_finops_metrics))
        .with_state(state)
}

async fn get_finops_metrics(State(state): State<FinOpsApiAppState>) -> impl IntoResponse {
    let now = Utc::now();
    let start_of_month = now
        .date_naive()
        .with_day(1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    // 1. Get total cost for current month
    let total_cost = match state
        .metrics_repo
        .get_aggregated_metrics(
            "job_cost",
            "sum",
            start_of_month,
            now,
            None, // No specific interval, just total
        )
        .await
    {
        Ok(metrics) => metrics.first().map(|m| m.avg_value).unwrap_or(0.0),
        Err(e) => {
            tracing::error!("Failed to fetch total cost: {}", e);
            0.0
        }
    };

    // 2. Get cost breakdown by pipeline
    // Note: This requires a more complex query or multiple queries.
    // For MVP, we might need to fetch all job_cost metrics and aggregate in memory
    // or add a specific method to repository.
    // Assuming we can get stats or use a specific tag query.
    // For now, let's use a simplified approach or mock if repository doesn't support grouping yet.
    // Ideally: state.metrics_repo.get_cost_breakdown(start_of_month, now).await

    // 3. Get cost history (daily)
    let history_start = now - chrono::Duration::days(30);
    let history = match state
        .metrics_repo
        .get_aggregated_metrics("job_cost", "sum", history_start, now, Some("1 day"))
        .await
    {
        Ok(metrics) => metrics
            .into_iter()
            .map(|m| CostHistory {
                date: m.bucket.to_rfc3339(),
                cost: m.avg_value,
                budget: 100.0, // Hardcoded daily budget for now
            })
            .collect(),
        Err(e) => {
            tracing::error!("Failed to fetch cost history: {}", e);
            vec![]
        }
    };

    let budget_limit = 5000.0; // Hardcoded monthly budget
    let budget_used_percentage = if budget_limit > 0.0 {
        (total_cost / budget_limit) * 100.0
    } else {
        0.0
    };

    let response = FinOpsMetrics {
        metrics: CostMetrics {
            total_cost,
            budget_limit,
            budget_used_percentage,
            projected_cost: total_cost * 1.2,    // Simple projection
            savings_potential: total_cost * 0.1, // Simple estimation
        },
        breakdown: vec![], // TODO: Implement breakdown query in repository
        history,
        alerts: vec![], // TODO: Implement alerts query
        last_updated: now.to_rfc3339(),
    };

    Json(response)
}
