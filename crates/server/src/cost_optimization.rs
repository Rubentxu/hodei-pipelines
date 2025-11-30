//! Cost Optimization API
//!
//! This module provides endpoints for analyzing and optimizing costs across resource pools,
//! including usage pattern analysis, right-sizing recommendations, and cost-saving opportunities.

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
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

/// Cost breakdown by resource type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub compute_cost: f64, // USD per hour
    pub storage_cost: f64, // USD per hour
    pub network_cost: f64, // USD per hour
    pub license_cost: f64, // USD per hour
    pub total_cost: f64,   // USD per hour
}

/// Usage pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePattern {
    pub average_cpu_utilization: f64,
    pub peak_cpu_utilization: f64,
    pub average_memory_utilization: f64,
    pub peak_memory_utilization: f64,
    pub average_concurrent_jobs: f64,
    pub peak_concurrent_jobs: u64,
    pub idle_time_percentage: f64,
    pub active_hours: Vec<String>, // e.g., ["09:00-17:00"]
}

/// Cost projection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostProjection {
    pub current_monthly_cost: f64,
    pub projected_monthly_cost: f64,
    pub annual_cost: f64,
    pub cost_trend: String,    // increasing, decreasing, stable
    pub confidence_level: f64, // 0-100%
}

/// Right-sizing recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightSizingRecommendation {
    pub current_capacity: i32,
    pub recommended_capacity: i32,
    pub savings_per_hour: f64,
    pub savings_per_month: f64,
    pub confidence: f64, // 0-100%
    pub reasoning: String,
    pub risk_level: String, // low, medium, high
}

/// Reserved instance recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservedInstanceRecommendation {
    pub usage_percentage: f64,
    pub recommended_term: String,           // "1 year", "3 years"
    pub recommended_payment_option: String, // "all upfront", "partial upfront", "no upfront"
    pub estimated_savings: f64,
    pub break_even_months: u32,
}

/// Idle resource recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleResourceRecommendation {
    pub resource_id: String,
    pub resource_type: String,
    pub days_idle: u64,
    pub cost_per_day: f64,
    pub action: String, // terminate, hibernate, downsize
    pub potential_savings: f64,
}

/// Cost optimization opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostOptimizationOpportunity {
    pub opportunity_id: String,
    pub pool_id: String,
    pub opportunity_type: String, // right_sizing, reserved_instances, idle_resources, spot_instances
    pub title: String,
    pub description: String,
    pub estimated_savings_per_month: f64,
    pub effort_level: String, // low, medium, high
    pub priority: u32,
    pub implementation_details: HashMap<String, String>,
}

/// Cost optimization analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostOptimizationAnalysis {
    pub pool_id: String,
    pub analysis_date: DateTime<Utc>,
    pub current_cost_breakdown: CostBreakdown,
    pub usage_pattern: UsagePattern,
    pub cost_projection: CostProjection,
    pub recommendations: Vec<CostOptimizationOpportunity>,
    pub total_potential_savings_per_month: f64,
    pub optimization_score: f64, // 0-100
}

/// Create cost analysis request
#[derive(Debug, Deserialize)]
pub struct CreateCostAnalysisRequest {
    pub pool_id: String,
    pub time_period_days: u64,
    pub include_recommendations: bool,
    pub budget_threshold: Option<f64>,
}

/// Cost optimization policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostOptimizationPolicy {
    pub policy_id: String,
    pub name: String,
    pub pool_id: String,
    pub max_monthly_budget: f64,
    pub auto_scaling_enabled: bool,
    pub reserved_instance_target: f64,
    pub idle_resource_threshold_days: u64,
    pub notification_email: Option<String>,
}

/// Message response
#[derive(Debug, Serialize)]
pub struct CostOptimizationMessageResponse {
    pub message: String,
}

/// Cost Optimization Service
#[derive(Debug, Clone)]
pub struct CostOptimizationService {
    /// Cost analyses by pool
    cost_analyses: Arc<RwLock<HashMap<String, CostOptimizationAnalysis>>>,
    /// Cost optimization policies
    policies: Arc<RwLock<HashMap<String, CostOptimizationPolicy>>>,
    /// Historical cost data
    cost_history: Arc<RwLock<HashMap<String, Vec<(DateTime<Utc>, f64)>>>>,
}

impl CostOptimizationService {
    /// Create new Cost Optimization Service
    pub fn new() -> Self {
        info!("Initializing Cost Optimization Service");
        Self {
            cost_analyses: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            cost_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create cost analysis
    pub async fn create_cost_analysis(
        &self,
        request: CreateCostAnalysisRequest,
    ) -> Result<CostOptimizationAnalysis, String> {
        let pool_id = request.pool_id.clone();
        let analysis_date = Utc::now();

        // Generate mock data based on pool ID
        let current_cost = self.calculate_current_cost(&pool_id);
        let usage_pattern = self.analyze_usage_pattern(&pool_id);
        let cost_projection = self.project_costs(&pool_id, &current_cost);

        let mut recommendations = Vec::new();
        if request.include_recommendations {
            recommendations = self
                .generate_recommendations(&pool_id, &current_cost, &usage_pattern)
                .await;
        }

        // Calculate total potential savings
        let total_potential_savings: f64 = recommendations
            .iter()
            .map(|r| r.estimated_savings_per_month)
            .sum();

        // Calculate optimization score (simplified algorithm)
        let optimization_score = self.calculate_optimization_score(
            &usage_pattern,
            &current_cost,
            total_potential_savings,
        );

        let analysis = CostOptimizationAnalysis {
            pool_id,
            analysis_date,
            current_cost_breakdown: current_cost,
            usage_pattern,
            cost_projection,
            recommendations,
            total_potential_savings_per_month: total_potential_savings,
            optimization_score,
        };

        // Store analysis
        let mut analyses = self.cost_analyses.write().await;
        analyses.insert(request.pool_id.clone(), analysis.clone());

        // Store in history
        let mut history = self.cost_history.write().await;
        let pool_history = history
            .entry(request.pool_id.clone())
            .or_insert_with(Vec::new);
        pool_history.push((analysis_date, analysis.cost_projection.current_monthly_cost));

        // Keep only last 30 days of history
        let thirty_days_ago = Utc::now() - chrono::Duration::days(30);
        pool_history.retain(|(date, _)| *date >= thirty_days_ago);

        info!(
            "Created cost analysis for pool: {} (potential savings: ${:.2}/month)",
            request.pool_id, total_potential_savings
        );

        Ok(analysis)
    }

    /// Get cost analysis
    pub async fn get_cost_analysis(
        &self,
        pool_id: &str,
    ) -> Result<CostOptimizationAnalysis, String> {
        let analyses = self.cost_analyses.read().await;
        analyses
            .get(pool_id)
            .cloned()
            .ok_or_else(|| "Cost analysis not found".to_string())
    }

    /// List cost analyses
    pub async fn list_cost_analyses(&self) -> Vec<String> {
        let analyses = self.cost_analyses.read().await;
        analyses.keys().cloned().collect()
    }

    /// Calculate current cost for pool
    fn calculate_current_cost(&self, pool_id: &str) -> CostBreakdown {
        // Mock calculation based on pool ID hash
        let hash = pool_id.chars().map(|c| c as u64).sum::<u64>();
        let base_cost = (hash % 1000) as f64 / 10.0;

        let compute_cost = base_cost + 50.0;
        let storage_cost = base_cost * 0.3;
        let network_cost = base_cost * 0.2;
        let license_cost = base_cost * 0.1;
        let total_cost = compute_cost + storage_cost + network_cost + license_cost;

        CostBreakdown {
            compute_cost,
            storage_cost,
            network_cost,
            license_cost,
            total_cost,
        }
    }

    /// Analyze usage patterns
    fn analyze_usage_pattern(&self, pool_id: &str) -> UsagePattern {
        let hash = pool_id.chars().map(|c| c as u64).sum::<u64>();
        let seed = (hash % 100) as f64 / 100.0;

        UsagePattern {
            average_cpu_utilization: 30.0 + seed * 40.0,    // 30-70%
            peak_cpu_utilization: 60.0 + seed * 30.0,       // 60-90%
            average_memory_utilization: 35.0 + seed * 35.0, // 35-70%
            peak_memory_utilization: 65.0 + seed * 25.0,    // 65-90%
            average_concurrent_jobs: 5.0 + seed * 10.0,     // 5-15
            peak_concurrent_jobs: (10.0 + seed * 20.0) as u64,
            idle_time_percentage: 20.0 + seed * 40.0, // 20-60%
            active_hours: vec!["09:00-17:00".to_string()],
        }
    }

    /// Project future costs
    fn project_costs(&self, pool_id: &str, current_cost: &CostBreakdown) -> CostProjection {
        let hash = pool_id.chars().map(|c| c as u64).sum::<u64>();
        let variance = ((hash % 20) as f64 - 10.0) / 100.0; // -10% to +10%

        let current_monthly = current_cost.total_cost * 24.0 * 30.0;
        let projected_monthly = current_monthly * (1.0 + variance);
        let annual_cost = projected_monthly * 12.0;

        let cost_trend = if variance > 0.05 {
            "increasing".to_string()
        } else if variance < -0.05 {
            "decreasing".to_string()
        } else {
            "stable".to_string()
        };

        CostProjection {
            current_monthly_cost: current_monthly,
            projected_monthly_cost: projected_monthly,
            annual_cost,
            cost_trend,
            confidence_level: 75.0 + (hash % 25) as f64,
        }
    }

    /// Generate optimization recommendations
    async fn generate_recommendations(
        &self,
        pool_id: &str,
        current_cost: &CostBreakdown,
        usage_pattern: &UsagePattern,
    ) -> Vec<CostOptimizationOpportunity> {
        let mut recommendations = Vec::new();

        // Right-sizing recommendation
        if usage_pattern.average_cpu_utilization < 40.0 {
            let recommended_capacity = (usage_pattern.peak_concurrent_jobs * 2) as i32;
            let current_capacity = 20; // Mock current capacity
            let savings_per_hour = (current_capacity - recommended_capacity) as f64 * 0.05;

            recommendations.push(CostOptimizationOpportunity {
                opportunity_id: uuid::Uuid::new_v4().to_string(),
                pool_id: pool_id.to_string(),
                opportunity_type: "right_sizing".to_string(),
                title: "Right-size compute resources".to_string(),
                description: format!(
                    "Current average CPU utilization is {:.1}%. Consider reducing capacity from {} to {} workers.",
                    usage_pattern.average_cpu_utilization, current_capacity, recommended_capacity
                ),
                estimated_savings_per_month: savings_per_hour * 24.0 * 30.0,
                effort_level: "low".to_string(),
                priority: 1,
                implementation_details: {
                    let mut map = HashMap::new();
                    map.insert("action".to_string(), "reduce_capacity".to_string());
                    map.insert("target_capacity".to_string(), recommended_capacity.to_string());
                    map
                },
            });
        }

        // Reserved instances recommendation
        if usage_pattern.average_cpu_utilization > 60.0 {
            let estimated_savings = current_cost.compute_cost * 0.30 * 24.0 * 30.0; // 30% savings

            recommendations.push(CostOptimizationOpportunity {
                opportunity_id: uuid::Uuid::new_v4().to_string(),
                pool_id: pool_id.to_string(),
                opportunity_type: "reserved_instances".to_string(),
                title: "Use reserved instances".to_string(),
                description: format!(
                    "High and steady utilization ({:.1}% average) suggests reserved instances could save 30-50%.",
                    usage_pattern.average_cpu_utilization
                ),
                estimated_savings_per_month: estimated_savings,
                effort_level: "medium".to_string(),
                priority: 2,
                implementation_details: {
                    let mut map = HashMap::new();
                    map.insert("action".to_string(), "purchase_reserved".to_string());
                    map.insert("term".to_string(), "1 year".to_string());
                    map.insert("payment_option".to_string(), "all upfront".to_string());
                    map
                },
            });
        }

        // Idle resources recommendation
        if usage_pattern.idle_time_percentage > 50.0 {
            let idle_cost_per_day =
                current_cost.total_cost * usage_pattern.idle_time_percentage / 100.0;
            let potential_savings = idle_cost_per_day * 30.0;

            recommendations.push(CostOptimizationOpportunity {
                opportunity_id: uuid::Uuid::new_v4().to_string(),
                pool_id: pool_id.to_string(),
                opportunity_type: "idle_resources".to_string(),
                title: "Optimize idle resources".to_string(),
                description: format!(
                    "{:.1}% of resources are idle. Consider auto-scaling or resource termination.",
                    usage_pattern.idle_time_percentage
                ),
                estimated_savings_per_month: potential_savings,
                effort_level: "low".to_string(),
                priority: 3,
                implementation_details: {
                    let mut map = HashMap::new();
                    map.insert("action".to_string(), "enable_auto_scaling".to_string());
                    map.insert("min_capacity".to_string(), "2".to_string());
                    map
                },
            });
        }

        // Spot instances recommendation
        if usage_pattern.average_concurrent_jobs < 10.0 {
            let estimated_savings = current_cost.compute_cost * 0.60 * 24.0 * 30.0; // 60% savings

            recommendations.push(CostOptimizationOpportunity {
                opportunity_id: uuid::Uuid::new_v4().to_string(),
                pool_id: pool_id.to_string(),
                opportunity_type: "spot_instances".to_string(),
                title: "Use spot instances".to_string(),
                description: "Low-priority workloads can use spot instances for up to 90% savings."
                    .to_string(),
                estimated_savings_per_month: estimated_savings,
                effort_level: "high".to_string(),
                priority: 4,
                implementation_details: {
                    let mut map = HashMap::new();
                    map.insert("action".to_string(), "migrate_to_spot".to_string());
                    map.insert("max_price".to_string(), "0.03".to_string());
                    map
                },
            });
        }

        recommendations.sort_by(|a, b| a.priority.cmp(&b.priority));

        recommendations
    }

    /// Calculate optimization score
    fn calculate_optimization_score(
        &self,
        usage_pattern: &UsagePattern,
        current_cost: &CostBreakdown,
        potential_savings: f64,
    ) -> f64 {
        let mut score = 100.0;

        // Penalize high idle time
        score -= usage_pattern.idle_time_percentage * 0.5;

        // Penalize low average utilization
        if usage_pattern.average_cpu_utilization < 30.0 {
            score -= 20.0;
        } else if usage_pattern.average_cpu_utilization < 50.0 {
            score -= 10.0;
        }

        // Add bonus for potential savings
        let savings_bonus = (potential_savings / current_cost.total_cost / 24.0 / 30.0) * 10.0;
        score += savings_bonus;

        // Clamp between 0 and 100
        score.max(0.0).min(100.0)
    }

    /// Create cost optimization policy
    pub async fn create_policy(&self, mut policy: CostOptimizationPolicy) -> Result<(), String> {
        let mut policies = self.policies.write().await;

        let policy_id = policy.policy_id.clone();

        if policies.contains_key(&policy_id) {
            return Err("Policy already exists".to_string());
        }

        policies.insert(policy_id.clone(), policy);

        info!("Created cost optimization policy: {}", policy_id);

        Ok(())
    }

    /// Get cost optimization policy
    pub async fn get_policy(&self, policy_id: &str) -> Result<CostOptimizationPolicy, String> {
        let policies = self.policies.read().await;
        policies
            .get(policy_id)
            .cloned()
            .ok_or_else(|| "Policy not found".to_string())
    }

    /// List all policies
    pub async fn list_policies(&self) -> Vec<CostOptimizationPolicy> {
        let policies = self.policies.read().await;
        policies.values().cloned().collect()
    }
}

/// Application state for Cost Optimization
#[derive(Clone)]
pub struct CostOptimizationAppState {
    pub service: Arc<CostOptimizationService>,
}

/// Create router for Cost Optimization API
pub fn cost_optimization_routes() -> Router<CostOptimizationAppState> {
    Router::new()
        .route(
            "/cost-optimization/analyses",
            post(create_cost_analysis_handler),
        )
        .route(
            "/cost-optimization/analyses/:pool_id",
            get(get_cost_analysis_handler),
        )
        .route(
            "/cost-optimization/analyses",
            get(list_cost_analyses_handler),
        )
        .route("/cost-optimization/policies", post(create_policy_handler))
        .route(
            "/cost-optimization/policies/:policy_id",
            get(get_policy_handler),
        )
        .route("/cost-optimization/policies", get(list_policies_handler))
}

/// Create cost analysis handler
async fn create_cost_analysis_handler(
    State(state): State<CostOptimizationAppState>,
    Json(payload): Json<CreateCostAnalysisRequest>,
) -> Result<Json<CostOptimizationMessageResponse>, StatusCode> {
    match state.service.create_cost_analysis(payload).await {
        Ok(analysis) => Ok(Json(CostOptimizationMessageResponse {
            message: format!(
                "Cost analysis created for pool {}. Potential savings: ${:.2}/month",
                analysis.pool_id, analysis.total_potential_savings_per_month
            ),
        })),
        Err(e) => {
            error!("Failed to create cost analysis: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Get cost analysis handler
async fn get_cost_analysis_handler(
    State(state): State<CostOptimizationAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
) -> Result<Json<CostOptimizationAnalysis>, StatusCode> {
    match state.service.get_cost_analysis(&pool_id).await {
        Ok(analysis) => Ok(Json(analysis)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// List cost analyses handler
async fn list_cost_analyses_handler(
    State(state): State<CostOptimizationAppState>,
) -> Result<Json<Vec<String>>, StatusCode> {
    Ok(Json(state.service.list_cost_analyses().await))
}

/// Create policy handler
async fn create_policy_handler(
    State(state): State<CostOptimizationAppState>,
    Json(payload): Json<CostOptimizationPolicy>,
) -> Result<Json<CostOptimizationMessageResponse>, StatusCode> {
    match state.service.create_policy(payload).await {
        Ok(_) => Ok(Json(CostOptimizationMessageResponse {
            message: "Cost optimization policy created successfully".to_string(),
        })),
        Err(e) => {
            error!("Failed to create policy: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Get policy handler
async fn get_policy_handler(
    State(state): State<CostOptimizationAppState>,
    axum::extract::Path(policy_id): axum::extract::Path<String>,
) -> Result<Json<CostOptimizationPolicy>, StatusCode> {
    match state.service.get_policy(&policy_id).await {
        Ok(policy) => Ok(Json(policy)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// List policies handler
async fn list_policies_handler(
    State(state): State<CostOptimizationAppState>,
) -> Result<Json<Vec<CostOptimizationPolicy>>, StatusCode> {
    Ok(Json(state.service.list_policies().await))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_cost_analysis() {
        let service = CostOptimizationService::new();

        let request = CreateCostAnalysisRequest {
            pool_id: "pool-1".to_string(),
            time_period_days: 30,
            include_recommendations: true,
            budget_threshold: Some(1000.0),
        };

        let result = service.create_cost_analysis(request).await;
        assert!(result.is_ok());

        let analysis = result.unwrap();
        assert_eq!(analysis.pool_id, "pool-1");
        assert!(!analysis.recommendations.is_empty());
        assert!(analysis.optimization_score >= 0.0 && analysis.optimization_score <= 100.0);
    }

    #[tokio::test]
    async fn test_get_cost_analysis() {
        let service = CostOptimizationService::new();

        let request = CreateCostAnalysisRequest {
            pool_id: "pool-1".to_string(),
            time_period_days: 30,
            include_recommendations: true,
            budget_threshold: None,
        };

        service.create_cost_analysis(request).await.unwrap();

        let analysis = service.get_cost_analysis("pool-1").await.unwrap();
        assert_eq!(analysis.pool_id, "pool-1");
    }

    #[tokio::test]
    async fn test_list_cost_analyses() {
        let service = CostOptimizationService::new();

        let request = CreateCostAnalysisRequest {
            pool_id: "pool-1".to_string(),
            time_period_days: 30,
            include_recommendations: false,
            budget_threshold: None,
        };

        service.create_cost_analysis(request).await.unwrap();

        let analyses = service.list_cost_analyses().await;
        assert_eq!(analyses.len(), 1);
        assert_eq!(analyses[0], "pool-1");
    }

    #[tokio::test]
    async fn test_create_policy() {
        let service = CostOptimizationService::new();

        let policy = CostOptimizationPolicy {
            policy_id: "policy-1".to_string(),
            name: "Cost Control Policy".to_string(),
            pool_id: "pool-1".to_string(),
            max_monthly_budget: 1000.0,
            auto_scaling_enabled: true,
            reserved_instance_target: 0.7,
            idle_resource_threshold_days: 7,
            notification_email: Some("admin@example.com".to_string()),
        };

        let result = service.create_policy(policy).await;
        assert!(result.is_ok());

        let policies = service.list_policies().await;
        assert_eq!(policies.len(), 1);
    }

    #[tokio::test]
    async fn test_duplicate_policy() {
        let service = CostOptimizationService::new();

        let policy = CostOptimizationPolicy {
            policy_id: "policy-1".to_string(),
            name: "Cost Control Policy".to_string(),
            pool_id: "pool-1".to_string(),
            max_monthly_budget: 1000.0,
            auto_scaling_enabled: true,
            reserved_instance_target: 0.7,
            idle_resource_threshold_days: 7,
            notification_email: None,
        };

        service.create_policy(policy.clone()).await.unwrap();
        let result = service.create_policy(policy).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }
}
