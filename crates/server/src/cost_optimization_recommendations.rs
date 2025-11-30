//! AI-Powered Cost Optimization Recommendations Module
//!
//! Provides intelligent recommendations for reducing cloud costs based on usage patterns.
//! Implements US-016 from the Epic Web Frontend Production Ready.

use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Optimization type enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum OptimizationType {
    /// Rightsizing recommendations (downsize oversized instances)
    Rightsizing,
    /// Reserved instance recommendations
    ReservedInstances,
    /// Storage tiering recommendations
    StorageTiering,
    /// Spot instance utilization
    SpotInstances,
    /// Delete unused resources
    DeleteUnused,
    /// Schedule non-production resources
    ScheduleResources,
    /// Data transfer optimization
    DataTransferOptimization,
}

/// Resource type enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum ResourceType {
    Compute,
    Storage,
    Network,
    Database,
    LoadBalancer,
    Other,
}

/// Recommendation structure
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Recommendation {
    /// Unique recommendation ID
    pub id: String,
    /// Title of the recommendation
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Type of optimization
    pub optimization_type: OptimizationType,
    /// Resource type affected
    pub resource_type: ResourceType,
    /// Current cost for this resource
    pub current_cost: f64,
    /// Potential monthly savings
    pub potential_savings: f64,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    /// Priority level (1-5, 5 being highest)
    pub priority: u8,
    /// Estimated effort to implement (hours)
    pub effort_hours: f64,
    /// Action items to take
    pub action_items: Vec<String>,
    /// Resource identifiers affected
    pub affected_resources: Vec<String>,
    /// Date recommendation was generated
    pub generated_at: DateTime<Utc>,
    /// Valid until date
    pub valid_until: DateTime<Utc>,
}

/// Service for AI-powered cost optimization
#[derive(Debug)]
pub struct CostOptimizationService {
    /// Mock recommendation data
    mock_recommendations: Arc<Vec<MockRecommendationData>>,
}

#[derive(Debug, Clone)]
struct MockRecommendationData {
    id: String,
    title: String,
    description: String,
    optimization_type: OptimizationType,
    resource_type: ResourceType,
    current_cost: f64,
    potential_savings: f64,
    confidence: f64,
    priority: u8,
    effort_hours: f64,
    action_items: Vec<String>,
    affected_resources: Vec<String>,
}

impl CostOptimizationService {
    /// Create new cost optimization service
    pub fn new() -> Self {
        let mock_recommendations = Self::generate_mock_recommendations();

        Self {
            mock_recommendations: Arc::new(mock_recommendations),
        }
    }

    /// Get cost optimization recommendations
    pub async fn get_recommendations(
        &self,
        tenant_id: Option<&str>,
        resource_type: Option<ResourceType>,
    ) -> Vec<Recommendation> {
        let mut recommendations = self.get_filtered_recommendations(tenant_id, resource_type);

        // Apply AI-powered scoring and ranking
        self.rank_recommendations(&mut recommendations);

        recommendations
    }

    /// Get recommendations by resource type
    pub async fn get_recommendations_by_resource_type(
        &self,
        resource_type: ResourceType,
    ) -> Vec<Recommendation> {
        self.get_recommendations(None, Some(resource_type)).await
    }

    /// Get top recommendations
    pub async fn get_top_recommendations(
        &self,
        limit: usize,
        tenant_id: Option<&str>,
    ) -> Vec<Recommendation> {
        let mut recommendations = self.get_recommendations(tenant_id, None).await;

        recommendations.truncate(limit);

        recommendations
    }

    /// Get recommendation by ID
    pub async fn get_recommendation_by_id(&self, id: &str) -> Option<Recommendation> {
        let recommendations = self.mock_recommendations.clone();

        for data in recommendations.iter() {
            if data.id == id {
                return Some(self.convert_to_recommendation(data));
            }
        }

        None
    }

    /// Get summary of potential savings
    pub async fn get_savings_summary(&self, tenant_id: Option<&str>) -> SavingsSummary {
        let recommendations = self.get_recommendations(tenant_id, None).await;

        let total_potential_savings: f64 =
            recommendations.iter().map(|r| r.potential_savings).sum();

        let total_current_cost: f64 = recommendations.iter().map(|r| r.current_cost).sum();

        let savings_percentage = if total_current_cost > 0.0 {
            (total_potential_savings / total_current_cost) * 100.0
        } else {
            0.0
        };

        let mut savings_by_type = HashMap::new();
        for rec in &recommendations {
            let type_name = format!("{:?}", rec.optimization_type);
            *savings_by_type.entry(type_name).or_insert(0.0) += rec.potential_savings;
        }

        let mut savings_by_resource = HashMap::new();
        for rec in &recommendations {
            let resource_name = format!("{:?}", rec.resource_type);
            *savings_by_resource.entry(resource_name).or_insert(0.0) += rec.potential_savings;
        }

        SavingsSummary {
            total_potential_savings,
            total_current_cost,
            savings_percentage,
            recommendations_count: recommendations.len(),
            savings_by_optimization_type: savings_by_type,
            savings_by_resource_type: savings_by_resource,
        }
    }

    /// Get filtered recommendations
    fn get_filtered_recommendations(
        &self,
        tenant_id: Option<&str>,
        resource_type: Option<ResourceType>,
    ) -> Vec<Recommendation> {
        let recommendations = self.mock_recommendations.clone();

        recommendations
            .iter()
            .filter(|rec| {
                // Filter by tenant_id (in real implementation)
                // For now, all recommendations are valid

                // Filter by resource type
                if let Some(rt) = resource_type.clone() {
                    if rec.resource_type != rt {
                        return false;
                    }
                }

                true
            })
            .map(|data| self.convert_to_recommendation(data))
            .collect()
    }

    /// Rank recommendations using AI-powered scoring
    fn rank_recommendations(&self, recommendations: &mut [Recommendation]) {
        // Sort by priority, confidence, and potential savings
        recommendations.sort_by(|a, b| {
            // Higher priority first
            b.priority
                .cmp(&a.priority)
                // Higher confidence second
                .then_with(|| {
                    b.confidence
                        .partial_cmp(&a.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                // Higher savings third
                .then_with(|| {
                    b.potential_savings
                        .partial_cmp(&a.potential_savings)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
    }

    /// Convert mock data to recommendation
    fn convert_to_recommendation(&self, data: &MockRecommendationData) -> Recommendation {
        let now = Utc::now();
        let valid_until = now + chrono::Duration::days(30);

        Recommendation {
            id: data.id.clone(),
            title: data.title.clone(),
            description: data.description.clone(),
            optimization_type: data.optimization_type.clone(),
            resource_type: data.resource_type.clone(),
            current_cost: data.current_cost,
            potential_savings: data.potential_savings,
            confidence: data.confidence,
            priority: data.priority,
            effort_hours: data.effort_hours,
            action_items: data.action_items.clone(),
            affected_resources: data.affected_resources.clone(),
            generated_at: now,
            valid_until,
        }
    }

    /// Generate mock recommendation data
    fn generate_mock_recommendations() -> Vec<MockRecommendationData> {
        use rand::Rng;
        let mut rng = rand::rng();

        let mut recommendations = Vec::new();

        // Rightsizing recommendations
        recommendations.push(MockRecommendationData {
            id: "rec-001".to_string(),
            title: "Downsize Oversized Compute Instances".to_string(),
            description: "Your compute instances are utilizing only 23% of allocated resources on average. Rightsizing can reduce costs significantly.".to_string(),
            optimization_type: OptimizationType::Rightsizing,
            resource_type: ResourceType::Compute,
            current_cost: 2500.0,
            potential_savings: 875.0,
            confidence: 0.92,
            priority: 5,
            effort_hours: 8.0,
            action_items: vec![
                "Identify oversized instances".to_string(),
                "Calculate optimal instance sizes".to_string(),
                "Test in non-production environment".to_string(),
                "Schedule maintenance window".to_string(),
                "Apply rightsizing changes".to_string(),
            ],
            affected_resources: vec!["i-0123456789abcdef0".to_string(), "i-0abcdef1234567890".to_string()],
        });

        // Reserved instances
        recommendations.push(MockRecommendationData {
            id: "rec-002".to_string(),
            title: "Purchase Reserved Instances for Stable Workloads".to_string(),
            description: "You have stable, long-running compute workloads that would benefit from Reserved Instances with 35-60% savings.".to_string(),
            optimization_type: OptimizationType::ReservedInstances,
            resource_type: ResourceType::Compute,
            current_cost: 1800.0,
            potential_savings: 720.0,
            confidence: 0.88,
            priority: 4,
            effort_hours: 4.0,
            action_items: vec![
                "Analyze instance usage patterns".to_string(),
                "Select appropriate Reserved Instance type".to_string(),
                "Purchase Reserved Instances".to_string(),
            ],
            affected_resources: vec!["i-0987654321fedcba0".to_string()],
        });

        // Storage tiering
        recommendations.push(MockRecommendationData {
            id: "rec-003".to_string(),
            title: "Optimize Storage Costs with Intelligent Tiering".to_string(),
            description: "72% of your storage has not been accessed in 90+ days and can be moved to cheaper tiers.".to_string(),
            optimization_type: OptimizationType::StorageTiering,
            resource_type: ResourceType::Storage,
            current_cost: 1200.0,
            potential_savings: 480.0,
            confidence: 0.95,
            priority: 4,
            effort_hours: 2.0,
            action_items: vec![
                "Enable lifecycle policies".to_string(),
                "Configure tiering rules".to_string(),
                "Monitor data access patterns".to_string(),
            ],
            affected_resources: vec!["vol-0123456789abcdef0".to_string(), "vol-0abcdef1234567890".to_string()],
        });

        // Delete unused resources
        recommendations.push(MockRecommendationData {
            id: "rec-004".to_string(),
            title: "Delete Unused Resources".to_string(),
            description: "Found 3 unused volumes and 2 unattached IP addresses that are accruing costs without usage.".to_string(),
            optimization_type: OptimizationType::DeleteUnused,
            resource_type: ResourceType::Storage,
            current_cost: 350.0,
            potential_savings: 315.0,
            confidence: 0.98,
            priority: 5,
            effort_hours: 1.0,
            action_items: vec![
                "Review unused resources list".to_string(),
                "Ensure data backup if needed".to_string(),
                "Delete unused volumes".to_string(),
                "Release unused IP addresses".to_string(),
            ],
            affected_resources: vec!["vol-unused001".to_string(), "vol-unused002".to_string()],
        });

        // Schedule resources
        recommendations.push(MockRecommendationData {
            id: "rec-005".to_string(),
            title: "Schedule Non-Production Resources".to_string(),
            description: "Development and staging environments can be shut down during off-hours, saving up to 60% on these resources.".to_string(),
            optimization_type: OptimizationType::ScheduleResources,
            resource_type: ResourceType::Compute,
            current_cost: 800.0,
            potential_savings: 480.0,
            confidence: 0.85,
            priority: 3,
            effort_hours: 6.0,
            action_items: vec![
                "Identify non-production resources".to_string(),
                "Define schedule policies".to_string(),
                "Implement automation scripts".to_string(),
                "Test scheduled shutdowns".to_string(),
            ],
            affected_resources: vec!["i-dev001".to_string(), "i-staging01".to_string()],
        });

        // Spot instances
        recommendations.push(MockRecommendationData {
            id: "rec-006".to_string(),
            title: "Use Spot Instances for Fault-Tolerant Workloads".to_string(),
            description: "Your batch processing and CI/CD workloads are ideal for Spot Instances with 70-90% cost savings.".to_string(),
            optimization_type: OptimizationType::SpotInstances,
            resource_type: ResourceType::Compute,
            current_cost: 1500.0,
            potential_savings: 1050.0,
            confidence: 0.78,
            priority: 3,
            effort_hours: 12.0,
            action_items: vec![
                "Identify fault-tolerant workloads".to_string(),
                "Implement spot instance strategy".to_string(),
                "Set up interruption handling".to_string(),
                "Configure fallback mechanisms".to_string(),
            ],
            affected_resources: vec!["batch-job-001".to_string(), "ci-runner-001".to_string()],
        });

        // Network optimization
        recommendations.push(MockRecommendationData {
            id: "rec-007".to_string(),
            title: "Optimize Data Transfer Costs".to_string(),
            description: "High data transfer costs detected. Consider using CDN and optimizing inter-region transfers.".to_string(),
            optimization_type: OptimizationType::DataTransferOptimization,
            resource_type: ResourceType::Network,
            current_cost: 600.0,
            potential_savings: 240.0,
            confidence: 0.82,
            priority: 3,
            effort_hours: 10.0,
            action_items: vec![
                "Analyze data transfer patterns".to_string(),
                "Implement CDN for static content".to_string(),
                "Optimize inter-region transfers".to_string(),
                "Configure data compression".to_string(),
            ],
            affected_resources: vec!["bucket-main".to_string(), "cdn-config".to_string()],
        });

        recommendations
    }
}

/// Savings summary structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavingsSummary {
    /// Total potential monthly savings
    pub total_potential_savings: f64,
    /// Total current cost
    pub total_current_cost: f64,
    /// Savings percentage
    pub savings_percentage: f64,
    /// Number of recommendations
    pub recommendations_count: usize,
    /// Savings by optimization type
    pub savings_by_optimization_type: HashMap<String, f64>,
    /// Savings by resource type
    pub savings_by_resource_type: HashMap<String, f64>,
}

impl Default for CostOptimizationService {
    fn default() -> Self {
        Self::new()
    }
}

/// Application state for Cost Optimization API
#[derive(Clone)]
pub struct CostOptimizationApiAppState {
    pub service: Arc<CostOptimizationService>,
}

/// GET /api/v1/cost-optimization/recommendations - Get cost optimization recommendations
#[allow(dead_code)]
pub async fn get_recommendations_handler(
    State(state): State<CostOptimizationApiAppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Recommendation>>, StatusCode> {
    info!("🤖 Cost optimization recommendations requested");

    let resource_type = params.get("resource_type").and_then(|s| match s.as_str() {
        "compute" => Some(ResourceType::Compute),
        "storage" => Some(ResourceType::Storage),
        "network" => Some(ResourceType::Network),
        "database" => Some(ResourceType::Database),
        "load_balancer" => Some(ResourceType::LoadBalancer),
        "other" => Some(ResourceType::Other),
        _ => None,
    });

    let recommendations = state.service.get_recommendations(None, resource_type).await;

    info!("✅ Returned {} recommendations", recommendations.len());

    Ok(Json(recommendations))
}

/// GET /api/v1/cost-optimization/recommendations/{id} - Get recommendation by ID
#[allow(dead_code)]
pub async fn get_recommendation_by_id_handler(
    State(state): State<CostOptimizationApiAppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<Recommendation>, StatusCode> {
    info!("🤖 Cost optimization recommendation {} requested", id);

    let recommendation = state.service.get_recommendation_by_id(&id).await;

    if let Some(rec) = recommendation {
        info!("✅ Recommendation found: {}", rec.title);
        Ok(Json(rec))
    } else {
        info!("❌ Recommendation not found: {}", id);
        Err(StatusCode::NOT_FOUND)
    }
}

/// GET /api/v1/cost-optimization/savings-summary - Get savings summary
#[allow(dead_code)]
pub async fn get_savings_summary_handler(
    State(state): State<CostOptimizationApiAppState>,
) -> Result<Json<SavingsSummary>, StatusCode> {
    info!("🤖 Cost optimization savings summary requested");

    let summary = state.service.get_savings_summary(None).await;

    info!(
        "✅ Savings summary - Potential: ${:.2} ({:.1}% savings)",
        summary.total_potential_savings, summary.savings_percentage
    );

    Ok(Json(summary))
}

/// GET /api/v1/cost-optimization/top-recommendations - Get top recommendations
#[allow(dead_code)]
pub async fn get_top_recommendations_handler(
    State(state): State<CostOptimizationApiAppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Recommendation>>, StatusCode> {
    info!("🤖 Top cost optimization recommendations requested");

    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(5);

    let recommendations = state.service.get_top_recommendations(limit, None).await;

    info!("✅ Returned top {} recommendations", recommendations.len());

    Ok(Json(recommendations))
}

/// Cost Optimization API routes
pub fn cost_optimization_api_routes() -> Router<CostOptimizationApiAppState> {
    Router::new()
        .route(
            "/cost-optimization/recommendations",
            get(get_recommendations_handler),
        )
        .route(
            "/cost-optimization/recommendations/{id}",
            get(get_recommendation_by_id_handler),
        )
        .route(
            "/cost-optimization/savings-summary",
            get(get_savings_summary_handler),
        )
        .route(
            "/cost-optimization/top-recommendations",
            get(get_top_recommendations_handler),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cost_optimization_service_new() {
        let service = CostOptimizationService::new();
        assert!(!service.mock_recommendations.is_empty());
    }

    #[tokio::test]
    async fn test_cost_optimization_service_get_recommendations() {
        let service = CostOptimizationService::new();

        let recommendations = service.get_recommendations(None, None).await;

        assert!(!recommendations.is_empty());
        for rec in &recommendations {
            assert!(!rec.id.is_empty());
            assert!(!rec.title.is_empty());
            assert!(rec.potential_savings >= 0.0);
            assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
            assert!(rec.priority >= 1 && rec.priority <= 5);
        }
    }

    #[tokio::test]
    async fn test_cost_optimization_service_get_recommendations_by_resource_type() {
        let service = CostOptimizationService::new();

        let compute_recs = service
            .get_recommendations_by_resource_type(ResourceType::Compute)
            .await;

        assert!(!compute_recs.is_empty());
        for rec in &compute_recs {
            assert_eq!(rec.resource_type, ResourceType::Compute);
        }
    }

    #[tokio::test]
    async fn test_cost_optimization_service_get_recommendation_by_id() {
        let service = CostOptimizationService::new();

        let rec = service.get_recommendation_by_id("rec-001").await;

        assert!(rec.is_some());
        let rec = rec.unwrap();
        assert_eq!(rec.id, "rec-001");
        assert!(rec.title.contains("Downsize"));
        assert_eq!(rec.optimization_type, OptimizationType::Rightsizing);
    }

    #[tokio::test]
    async fn test_cost_optimization_service_get_savings_summary() {
        let service = CostOptimizationService::new();

        let summary = service.get_savings_summary(None).await;

        assert!(summary.total_potential_savings > 0.0);
        assert!(summary.total_current_cost > 0.0);
        assert!(summary.recommendations_count > 0);
        assert!(!summary.savings_by_optimization_type.is_empty());
        assert!(!summary.savings_by_resource_type.is_empty());
    }

    #[tokio::test]
    async fn test_recommendation_serialization() {
        let recommendation = Recommendation {
            id: "rec-test".to_string(),
            title: "Test Recommendation".to_string(),
            description: "Test description".to_string(),
            optimization_type: OptimizationType::Rightsizing,
            resource_type: ResourceType::Compute,
            current_cost: 1000.0,
            potential_savings: 350.0,
            confidence: 0.9,
            priority: 5,
            effort_hours: 8.0,
            action_items: vec!["Action 1".to_string()],
            affected_resources: vec!["res-001".to_string()],
            generated_at: Utc::now(),
            valid_until: Utc::now() + chrono::Duration::days(30),
        };

        let json = serde_json::to_string(&recommendation).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("title"));
        assert!(json.contains("potential_savings"));

        let deserialized: Recommendation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, recommendation.id);
        assert_eq!(
            deserialized.potential_savings,
            recommendation.potential_savings
        );
    }

    #[tokio::test]
    async fn test_savings_summary_serialization() {
        let summary = SavingsSummary {
            total_potential_savings: 1500.0,
            total_current_cost: 5000.0,
            savings_percentage: 30.0,
            recommendations_count: 5,
            savings_by_optimization_type: HashMap::new(),
            savings_by_resource_type: HashMap::new(),
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("total_potential_savings"));
        assert!(json.contains("savings_percentage"));

        let deserialized: SavingsSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.total_potential_savings,
            summary.total_potential_savings
        );
    }

    #[tokio::test]
    async fn test_optimization_type_deserialization() {
        let json = r#"{"Rightsizing":null}"#;
        let optimization_type: Result<OptimizationType, _> = serde_json::from_str(json);
        assert!(optimization_type.is_ok());

        let json = r#""Rightsizing""#;
        let optimization_type: Result<OptimizationType, _> = serde_json::from_str(json);
        assert!(optimization_type.is_ok());
    }

    #[tokio::test]
    async fn test_resource_type_deserialization() {
        let json = r#"{"Compute":null}"#;
        let resource_type: Result<ResourceType, _> = serde_json::from_str(json);
        assert!(resource_type.is_ok());

        let json = r#""Compute""#;
        let resource_type: Result<ResourceType, _> = serde_json::from_str(json);
        assert!(resource_type.is_ok());
    }
}
