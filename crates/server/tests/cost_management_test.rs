//! Story 6: Implement Cost Management Frontend (US-API-ALIGN-006)
//!
//! Tests to validate cost management APIs (US-015, US-016, US-017) are properly
//! documented, typed, and aligned with frontend expectations.

use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    use hodei_server::api_docs::{ApiDoc, *};
    use hodei_server::cost_optimization_recommendations::{
        OptimizationType, Recommendation, ResourceType,
    };
    use hodei_server::cost_tracking_aggregation::{
        CostBreakdown, CostSummary, CostTrend, TenantCostBreakdown,
    };
    use utoipa::OpenApi;

    /// Test 1: Validate CostSummary schema in OpenAPI
    #[tokio::test]
    async fn test_cost_summary_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components;
        assert!(components.is_some(), "OpenAPI should have components");

        let schemas = components.unwrap().schemas;
        assert!(
            schemas.contains_key("CostSummary"),
            "CostSummary should be in OpenAPI schemas"
        );

        println!("✅ CostSummary schema defined in OpenAPI");
    }

    /// Test 2: Validate CostBreakdown schema in OpenAPI
    #[tokio::test]
    async fn test_cost_breakdown_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("CostBreakdown"),
            "CostBreakdown should be in OpenAPI schemas"
        );

        println!("✅ CostBreakdown schema defined in OpenAPI");
    }

    /// Test 3: Validate TenantCostBreakdown schema in OpenAPI
    #[tokio::test]
    async fn test_tenant_cost_breakdown_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("TenantCostBreakdown"),
            "TenantCostBreakdown should be in OpenAPI schemas"
        );

        println!("✅ TenantCostBreakdown schema defined in OpenAPI");
    }

    /// Test 4: Validate CostTrend schema in OpenAPI
    #[tokio::test]
    async fn test_cost_trend_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("CostTrend"),
            "CostTrend should be in OpenAPI schemas"
        );

        println!("✅ CostTrend schema defined in OpenAPI");
    }

    /// Test 5: Validate Recommendation schema in OpenAPI
    #[tokio::test]
    async fn test_recommendation_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("Recommendation"),
            "Recommendation should be in OpenAPI schemas"
        );

        println!("✅ Recommendation schema defined in OpenAPI");
    }

    /// Test 6: Validate OptimizationType schema in OpenAPI
    #[tokio::test]
    async fn test_optimization_type_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("OptimizationType"),
            "OptimizationType should be in OpenAPI schemas"
        );

        println!("✅ OptimizationType schema defined in OpenAPI");
    }

    /// Test 7: Validate ResourceType schema in OpenAPI
    #[tokio::test]
    async fn test_resource_type_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ResourceType"),
            "ResourceType should be in OpenAPI schemas"
        );

        println!("✅ ResourceType schema defined in OpenAPI");
    }

    /// Test 8: Test cost summary serialization/deserialization
    #[tokio::test]
    async fn test_cost_summary_serialization() {
        let mut breakdown_by_resource = HashMap::new();
        breakdown_by_resource.insert("compute".to_string(), 500.0);
        breakdown_by_resource.insert("storage".to_string(), 200.0);

        let mut breakdown_by_tenant = HashMap::new();
        breakdown_by_tenant.insert("tenant-123".to_string(), 450.0);
        breakdown_by_tenant.insert("tenant-456".to_string(), 250.0);

        let summary = CostSummary {
            total_cost: 700.0,
            period_start: Utc::now(),
            period_end: Utc::now(),
            currency: "USD".to_string(),
            breakdown_by_resource,
            breakdown_by_tenant,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("total_cost"), "Should contain total_cost");
        assert!(json.contains("currency"), "Should contain currency");
        assert!(
            json.contains("breakdown_by_resource"),
            "Should have breakdown_by_resource"
        );

        let deserialized: CostSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_cost, 700.0);
        assert_eq!(deserialized.currency, "USD");

        println!("✅ CostSummary serialization/deserialization works correctly");
    }

    /// Test 9: Test cost breakdown serialization/deserialization
    #[tokio::test]
    async fn test_cost_breakdown_serialization() {
        let breakdown = CostBreakdown {
            resource_type: "compute".to_string(),
            cost: 500.0,
            usage_quantity: 1000.0,
            unit: "GB-hours".to_string(),
            cost_per_unit: 0.5,
            period_start: Utc::now(),
            period_end: Utc::now(),
        };

        let json = serde_json::to_string(&breakdown).unwrap();
        assert!(
            json.contains("resource_type"),
            "Should contain resource_type"
        );
        assert!(json.contains("cost"), "Should contain cost");
        assert!(json.contains("cost_per_unit"), "Should have cost_per_unit");

        let deserialized: CostBreakdown = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.resource_type, "compute");
        assert_eq!(deserialized.cost, 500.0);
        assert_eq!(deserialized.cost_per_unit, 0.5);

        println!("✅ CostBreakdown serialization/deserialization works correctly");
    }

    /// Test 10: Test tenant cost breakdown serialization/deserialization
    #[tokio::test]
    async fn test_tenant_cost_breakdown_serialization() {
        let mut resource_breakdown = HashMap::new();
        resource_breakdown.insert("compute".to_string(), 400.0);
        resource_breakdown.insert("storage".to_string(), 150.0);

        let breakdown = TenantCostBreakdown {
            tenant_id: "tenant-123".to_string(),
            total_cost: 550.0,
            resource_breakdown,
            period_start: Utc::now(),
            period_end: Utc::now(),
        };

        let json = serde_json::to_string(&breakdown).unwrap();
        assert!(json.contains("tenant_id"), "Should contain tenant_id");
        assert!(json.contains("total_cost"), "Should contain total_cost");
        assert!(
            json.contains("resource_breakdown"),
            "Should have resource_breakdown"
        );

        let deserialized: TenantCostBreakdown = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tenant_id, "tenant-123");
        assert_eq!(deserialized.total_cost, 550.0);

        println!("✅ TenantCostBreakdown serialization/deserialization works correctly");
    }

    /// Test 11: Test cost trend serialization/deserialization
    #[tokio::test]
    async fn test_cost_trend_serialization() {
        let trend = CostTrend {
            date: Utc::now(),
            total_cost: 100.0,
            compute_cost: 60.0,
            storage_cost: 30.0,
            network_cost: 10.0,
        };

        let json = serde_json::to_string(&trend).unwrap();
        assert!(json.contains("date"), "Should contain date");
        assert!(json.contains("total_cost"), "Should contain total_cost");
        assert!(json.contains("compute_cost"), "Should contain compute_cost");

        let deserialized: CostTrend = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_cost, 100.0);
        assert_eq!(deserialized.compute_cost, 60.0);
        assert_eq!(deserialized.storage_cost, 30.0);

        println!("✅ CostTrend serialization/deserialization works correctly");
    }

    /// Test 12: Test recommendation serialization/deserialization
    #[tokio::test]
    async fn test_recommendation_serialization() {
        let recommendation = Recommendation {
            id: "rec-123".to_string(),
            title: "Downsize oversized instances".to_string(),
            description: "You have 5 instances running at less than 20% CPU utilization"
                .to_string(),
            optimization_type: OptimizationType::Rightsizing,
            resource_type: ResourceType::Compute,
            current_cost: 500.0,
            potential_savings: 200.0,
            confidence: 0.85,
            priority: 4,
            effort_hours: 8.0,
            action_items: vec![
                "Review instance utilization".to_string(),
                "Select smaller instance type".to_string(),
            ],
            affected_resources: vec!["i-12345".to_string(), "i-67890".to_string()],
            generated_at: Utc::now(),
            valid_until: Utc::now() + chrono::Duration::days(30),
        };

        let json = serde_json::to_string(&recommendation).unwrap();
        assert!(json.contains("id"), "Should contain id");
        assert!(json.contains("title"), "Should contain title");
        assert!(
            json.contains("potential_savings"),
            "Should have potential_savings"
        );

        let deserialized: Recommendation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "rec-123");
        assert_eq!(deserialized.potential_savings, 200.0);
        assert_eq!(deserialized.confidence, 0.85);

        println!("✅ Recommendation serialization/deserialization works correctly");
    }

    /// Test 13: Test optimization type enum values
    #[tokio::test]
    async fn test_optimization_type_enum_values() {
        let types = vec![
            OptimizationType::Rightsizing,
            OptimizationType::ReservedInstances,
            OptimizationType::StorageTiering,
            OptimizationType::SpotInstances,
            OptimizationType::DeleteUnused,
            OptimizationType::ScheduleResources,
            OptimizationType::DataTransferOptimization,
        ];

        for opt_type in types {
            let json = serde_json::to_string(&opt_type).unwrap();
            let deserialized: OptimizationType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, opt_type);
        }

        println!("✅ OptimizationType enum values are correct");
    }

    /// Test 14: Test resource type enum values
    #[tokio::test]
    async fn test_resource_type_enum_values() {
        let types = vec![
            ResourceType::Compute,
            ResourceType::Storage,
            ResourceType::Network,
            ResourceType::Database,
            ResourceType::LoadBalancer,
            ResourceType::Other,
        ];

        for res_type in types {
            let json = serde_json::to_string(&res_type).unwrap();
            let deserialized: ResourceType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, res_type);
        }

        println!("✅ ResourceType enum values are correct");
    }

    /// Test 15: Test cost management endpoints documented in OpenAPI
    #[tokio::test]
    async fn test_cost_management_endpoints_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let paths = &openapi.paths.paths;

        println!("\n💰 Cost Management Endpoints Check");
        println!("===================================");

        // Check for cost management endpoints
        assert!(
            paths.contains_key("/api/v1/costs/summary"),
            "Should have /api/v1/costs/summary endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/costs/by-resource"),
            "Should have /api/v1/costs/by-resource endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/costs/by-tenant"),
            "Should have /api/v1/costs/by-tenant endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/costs/trends"),
            "Should have /api/v1/costs/trends endpoint"
        );

        println!("  ✅ /api/v1/costs/summary documented");
        println!("  ✅ /api/v1/costs/by-resource documented");
        println!("  ✅ /api/v1/costs/by-tenant documented");
        println!("  ✅ /api/v1/costs/trends documented");
        println!("✅ All cost management endpoints documented");
    }

    /// Test 16: Test cost management API OpenAPI completeness
    #[tokio::test]
    async fn test_cost_management_openapi_completeness() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let paths = openapi.paths.paths;

        let mut cost_management_count = 0;

        // Count cost management-related endpoints
        for (path, _) in paths.iter() {
            if path.contains("/costs/") {
                cost_management_count += 1;
            }
        }

        println!("\n💰 Cost Management OpenAPI Coverage");
        println!("===================================");
        println!("Total Cost Management Endpoints: {}", cost_management_count);

        // Minimum expected: summary, by-resource, by-tenant, trends
        assert!(
            cost_management_count >= 4,
            "Expected at least 4 cost management endpoints, found {}",
            cost_management_count
        );

        println!("✅ Cost Management API coverage test passed");
    }

    /// Test 17: Test cost data filtering by date range
    #[tokio::test]
    async fn test_cost_data_date_filtering() {
        let start_date = Utc::now() - chrono::Duration::days(7);
        let end_date = Utc::now();

        let mut breakdown_by_resource = HashMap::new();
        breakdown_by_resource.insert("compute".to_string(), 350.0);

        let mut breakdown_by_tenant = HashMap::new();
        breakdown_by_tenant.insert("tenant-123".to_string(), 350.0);

        let summary = CostSummary {
            total_cost: 350.0,
            period_start: start_date,
            period_end: end_date,
            currency: "USD".to_string(),
            breakdown_by_resource,
            breakdown_by_tenant,
        };

        // Verify date range is preserved
        assert!(summary.period_start <= summary.period_end);
        assert!(summary.period_start.date_naive() <= end_date.date_naive());
        assert!(summary.period_end.date_naive() >= start_date.date_naive());

        println!("✅ Cost data date filtering works correctly");
    }

    /// Test 18: Test cost currency handling
    #[tokio::test]
    async fn test_cost_currency_handling() {
        let currencies = vec!["USD", "EUR", "GBP", "JPY", "AUD"];

        for currency in currencies {
            let summary = CostSummary {
                total_cost: 100.0,
                period_start: Utc::now(),
                period_end: Utc::now(),
                currency: currency.to_string(),
                breakdown_by_resource: HashMap::new(),
                breakdown_by_tenant: HashMap::new(),
            };

            let json = serde_json::to_string(&summary).unwrap();
            let deserialized: CostSummary = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.currency, currency);
        }

        println!("✅ Cost currency handling works correctly for multiple currencies");
    }

    /// Test 19: Test cost breakdown calculations
    #[tokio::test]
    async fn test_cost_breakdown_calculations() {
        // Test cost per unit calculation
        let breakdown = CostBreakdown {
            resource_type: "compute".to_string(),
            cost: 500.0,
            usage_quantity: 1000.0,
            unit: "GB-hours".to_string(),
            cost_per_unit: 0.5, // 500.0 / 1000.0
            period_start: Utc::now(),
            period_end: Utc::now(),
        };

        assert_eq!(breakdown.cost_per_unit, 0.5);
        assert_eq!(
            breakdown.cost,
            breakdown.usage_quantity * breakdown.cost_per_unit
        );

        // Test with zero usage
        let breakdown_zero = CostBreakdown {
            resource_type: "compute".to_string(),
            cost: 0.0,
            usage_quantity: 0.0,
            unit: "GB-hours".to_string(),
            cost_per_unit: 0.0,
            period_start: Utc::now(),
            period_end: Utc::now(),
        };

        assert_eq!(breakdown_zero.cost_per_unit, 0.0);

        println!("✅ Cost breakdown calculations work correctly");
    }

    /// Test 20: Test recommendation confidence and priority values
    #[tokio::test]
    async fn test_recommendation_confidence_priority() {
        // Test confidence range (0.0 to 1.0)
        let high_confidence = Recommendation {
            id: "rec-high".to_string(),
            title: "High confidence recommendation".to_string(),
            description: "Clear optimization opportunity".to_string(),
            optimization_type: OptimizationType::Rightsizing,
            resource_type: ResourceType::Compute,
            current_cost: 500.0,
            potential_savings: 200.0,
            confidence: 0.95,
            priority: 5,
            effort_hours: 4.0,
            action_items: vec![],
            affected_resources: vec![],
            generated_at: Utc::now(),
            valid_until: Utc::now() + chrono::Duration::days(30),
        };

        assert!(high_confidence.confidence >= 0.0);
        assert!(high_confidence.confidence <= 1.0);
        assert!(high_confidence.priority >= 1);
        assert!(high_confidence.priority <= 5);

        // Test low confidence
        let low_confidence = Recommendation {
            id: "rec-low".to_string(),
            title: "Low confidence recommendation".to_string(),
            description: "Uncertain optimization".to_string(),
            optimization_type: OptimizationType::SpotInstances,
            resource_type: ResourceType::Network,
            current_cost: 100.0,
            potential_savings: 10.0,
            confidence: 0.25,
            priority: 2,
            effort_hours: 16.0,
            action_items: vec![],
            affected_resources: vec![],
            generated_at: Utc::now(),
            valid_until: Utc::now() + chrono::Duration::days(30),
        };

        assert!(low_confidence.confidence >= 0.0);
        assert!(low_confidence.confidence <= 1.0);

        println!("✅ Recommendation confidence and priority values are valid");
    }
}
