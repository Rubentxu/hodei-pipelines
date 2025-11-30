//! Story 10: Complete OpenAPI Documentation for Security APIs (US-API-ALIGN-010)
//!
//! Comprehensive tests to validate that all security and vulnerability tracking APIs
//! have complete OpenAPI 3.0 documentation including:
//! - Request/response schemas
//! - HTTP status codes
//! - Parameter documentation
//! - Type safety with utoipa

use chrono::Utc;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    use hodei_server::api_docs::{ApiDoc, *};
    use hodei_server::security_vulnerability_tracking::{
        ComplianceCheck, ComplianceFramework, ControlStatus, SecurityScore, Vulnerability,
        VulnerabilitySeverity, VulnerabilityStatus,
    };
    use utoipa::OpenApi;

    /// Test 1: Validate Vulnerability enum schema in OpenAPI
    #[tokio::test]
    async fn test_vulnerability_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("Vulnerability"),
            "Vulnerability should be in OpenAPI schemas"
        );

        println!("✅ Vulnerability schema defined");
    }

    /// Test 2: Validate VulnerabilitySeverity enum schema
    #[tokio::test]
    async fn test_vulnerability_severity_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("VulnerabilitySeverity"),
            "VulnerabilitySeverity should be in OpenAPI schemas"
        );

        println!("✅ VulnerabilitySeverity schema defined");
    }

    /// Test 3: Validate VulnerabilityStatus enum schema
    #[tokio::test]
    async fn test_vulnerability_status_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("VulnerabilityStatus"),
            "VulnerabilityStatus should be in OpenAPI schemas"
        );

        println!("✅ VulnerabilityStatus schema defined");
    }

    /// Test 4: Validate SecurityScore schema
    #[tokio::test]
    async fn test_security_score_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("SecurityScore"),
            "SecurityScore should be in OpenAPI schemas"
        );

        println!("✅ SecurityScore schema defined");
    }

    /// Test 5: Validate ComplianceCheck schema
    #[tokio::test]
    async fn test_compliance_check_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ComplianceCheck"),
            "ComplianceCheck should be in OpenAPI schemas"
        );

        println!("✅ ComplianceCheck schema defined");
    }

    /// Test 6: Validate ComplianceFramework enum schema
    #[tokio::test]
    async fn test_compliance_framework_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ComplianceFramework"),
            "ComplianceFramework should be in OpenAPI schemas"
        );

        println!("✅ ComplianceFramework schema defined");
    }

    /// Test 7: Validate ControlStatus enum schema
    #[tokio::test]
    async fn test_control_status_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ControlStatus"),
            "ControlStatus should be in OpenAPI schemas"
        );

        println!("✅ ControlStatus schema defined");
    }

    /// Test 8: Validate SecurityMetrics schema
    #[tokio::test]
    async fn test_security_metrics_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("SecurityMetrics"),
            "SecurityMetrics should be in OpenAPI schemas"
        );

        println!("✅ SecurityMetrics schema defined");
    }

    /// Test 9: Validate GET /api/v1/security/vulnerabilities endpoint
    #[tokio::test]
    async fn test_list_vulnerabilities_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/security/vulnerabilities"),
            "GET /api/v1/security/vulnerabilities should be documented"
        );

        println!("✅ List vulnerabilities endpoint documented");
    }

    /// Test 10: Validate GET /api/v1/security/vulnerabilities/{id} endpoint
    #[tokio::test]
    async fn test_get_vulnerability_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/security/vulnerabilities/{id}"),
            "GET /api/v1/security/vulnerabilities/{{id}} should be documented"
        );

        println!("✅ Get vulnerability endpoint documented");
    }

    /// Test 11: Validate GET /api/v1/security/score/{entity_id} endpoint
    #[tokio::test]
    async fn test_get_security_score_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/security/score/{entity_id}"),
            "GET /api/v1/security/score/{{entity_id}} should be documented"
        );

        println!("✅ Get security score endpoint documented");
    }

    /// Test 12: Validate GET /api/v1/security/scores endpoint
    #[tokio::test]
    async fn test_list_security_scores_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi.paths.paths.contains_key("/api/v1/security/scores"),
            "GET /api/v1/security/scores should be documented"
        );

        println!("✅ List security scores endpoint documented");
    }

    /// Test 13: Validate GET /api/v1/security/compliance endpoint
    #[tokio::test]
    async fn test_list_compliance_checks_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/security/compliance"),
            "GET /api/v1/security/compliance should be documented"
        );

        println!("✅ List compliance checks endpoint documented");
    }

    /// Test 14: Validate POST /api/v1/security/reports/generate endpoint
    #[tokio::test]
    async fn test_generate_security_report_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/security/reports/generate"),
            "POST /api/v1/security/reports/generate should be documented"
        );

        println!("✅ Security reports endpoint documented");
    }

    /// Test 15: Validate GET /api/v1/security/metrics/{tenant_id} endpoint
    #[tokio::test]
    async fn test_get_security_metrics_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/security/metrics/{tenant_id}"),
            "GET /api/v1/security/metrics/{{tenant_id}} should be documented"
        );

        println!("✅ Get security metrics endpoint documented");
    }

    /// Test 16: Verify "security" tag is used
    #[tokio::test]
    async fn test_security_tag_usage() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        let has_security_tag = openapi
            .tags
            .as_ref()
            .map_or(false, |tags| tags.iter().any(|tag| tag.name == "security"));

        assert!(has_security_tag, "security tag should be defined");

        println!("✅ security tag is defined in OpenAPI");
    }

    /// Test 17: Validate Vulnerability serialization/deserialization
    #[tokio::test]
    async fn test_vulnerability_dto_serialization() {
        let vulnerability = Vulnerability {
            id: "vuln-001".to_string(),
            title: "Test Vulnerability".to_string(),
            description: "A test vulnerability for validation".to_string(),
            severity: VulnerabilitySeverity::High,
            status: VulnerabilityStatus::Open,
            cve_id: Some("CVE-2023-12345".to_string()),
            cvss_score: 8.5,
            resource_id: "test-resource".to_string(),
            resource_type: "container".to_string(),
            tenant_id: "tenant-123".to_string(),
            discovered_at: Utc::now(),
            updated_at: Utc::now(),
            due_date: Some(Utc::now()),
            assigned_to: Some("security-team".to_string()),
            evidence: vec!["Evidence 1".to_string()],
            remediation_steps: vec!["Apply patch".to_string()],
            related_vulnerabilities: vec![],
            tags: vec!["critical".to_string()],
        };

        let serialized = serde_json::to_string(&vulnerability).unwrap();
        let deserialized: Vulnerability = serde_json::from_str(&serialized).unwrap();

        assert_eq!(vulnerability.id, deserialized.id);
        assert_eq!(vulnerability.severity, deserialized.severity);
        println!("✅ Vulnerability serialization/deserialization works");
    }

    /// Test 18: Validate SecurityScore serialization/deserialization
    #[tokio::test]
    async fn test_security_score_dto_serialization() {
        let score = SecurityScore {
            id: "score-001".to_string(),
            entity_id: "entity-123".to_string(),
            entity_type: "pipeline".to_string(),
            overall_score: 85.5,
            vulnerability_score: 20.0,
            compliance_score: 90.0,
            configuration_score: 85.0,
            score_breakdown: HashMap::new(),
            critical_count: 2,
            high_count: 5,
            medium_count: 10,
            low_count: 20,
            open_count: 15,
            resolved_count: 22,
            trend: "improving".to_string(),
            calculated_at: Utc::now(),
            tenant_id: "tenant-456".to_string(),
        };

        let serialized = serde_json::to_string(&score).unwrap();
        let deserialized: SecurityScore = serde_json::from_str(&serialized).unwrap();

        assert_eq!(score.entity_id, deserialized.entity_id);
        assert_eq!(score.overall_score, deserialized.overall_score);
        println!("✅ SecurityScore serialization/deserialization works");
    }

    /// Test 19: Validate ComplianceCheck serialization/deserialization
    #[tokio::test]
    async fn test_compliance_check_dto_serialization() {
        let check = ComplianceCheck {
            id: "check-001".to_string(),
            tenant_id: "tenant-789".to_string(),
            framework: ComplianceFramework::SOC2,
            control_id: "CC-1.1".to_string(),
            control_name: "Access Control".to_string(),
            control_description: "Access control policy".to_string(),
            status: ControlStatus::Implemented,
            implementation_percentage: 100.0,
            evidence: vec!["https://evidence.example.com".to_string()],
            last_assessment: Utc::now(),
            next_assessment: Utc::now(),
            resource_id: None,
            notes: Some("All checks passed".to_string()),
        };

        let serialized = serde_json::to_string(&check).unwrap();
        let deserialized: ComplianceCheck = serde_json::from_str(&serialized).unwrap();

        assert_eq!(check.id, deserialized.id);
        assert_eq!(check.framework, deserialized.framework);
        println!("✅ ComplianceCheck serialization/deserialization works");
    }

    /// Test 20: Test enum values for VulnerabilitySeverity
    #[tokio::test]
    async fn test_vulnerability_severity_enum_values() {
        let severities = vec![
            VulnerabilitySeverity::Critical,
            VulnerabilitySeverity::High,
            VulnerabilitySeverity::Medium,
            VulnerabilitySeverity::Low,
            VulnerabilitySeverity::Info,
        ];

        for severity in severities {
            let serialized = serde_json::to_string(&severity).unwrap();
            let deserialized: VulnerabilitySeverity = serde_json::from_str(&serialized).unwrap();
            assert_eq!(severity, deserialized);
        }

        println!("✅ VulnerabilitySeverity enum values validated");
    }
}
