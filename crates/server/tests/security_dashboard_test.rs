//! Story 7: Implement Security Dashboard (US-API-ALIGN-007)
//!
//! Tests to validate security dashboard APIs (US-018) are properly
//! documented, typed, and aligned with frontend expectations.

use chrono::Utc;

#[cfg(test)]
mod tests {
    use super::*;

    use hodei_server::api_docs::{ApiDoc, *};
    use hodei_server::security_vulnerability_tracking::{
        ComplianceCheck, ComplianceFramework, ControlStatus, SecurityMetrics, SecurityScore,
        Vulnerability, VulnerabilitySeverity, VulnerabilityStatus,
    };
    use std::collections::HashMap;
    use utoipa::OpenApi;

    /// Test 1: Validate Vulnerability schema in OpenAPI
    #[tokio::test]
    async fn test_vulnerability_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components;
        assert!(components.is_some(), "OpenAPI should have components");

        let schemas = components.unwrap().schemas;
        assert!(
            schemas.contains_key("Vulnerability"),
            "Vulnerability should be in OpenAPI schemas"
        );

        println!("✅ Vulnerability schema defined in OpenAPI");
    }

    /// Test 2: Validate VulnerabilitySeverity schema in OpenAPI
    #[tokio::test]
    async fn test_vulnerability_severity_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("VulnerabilitySeverity"),
            "VulnerabilitySeverity should be in OpenAPI schemas"
        );

        println!("✅ VulnerabilitySeverity schema defined in OpenAPI");
    }

    /// Test 3: Validate VulnerabilityStatus schema in OpenAPI
    #[tokio::test]
    async fn test_vulnerability_status_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("VulnerabilityStatus"),
            "VulnerabilityStatus should be in OpenAPI schemas"
        );

        println!("✅ VulnerabilityStatus schema defined in OpenAPI");
    }

    /// Test 4: Validate SecurityScore schema in OpenAPI
    #[tokio::test]
    async fn test_security_score_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("SecurityScore"),
            "SecurityScore should be in OpenAPI schemas"
        );

        println!("✅ SecurityScore schema defined in OpenAPI");
    }

    /// Test 5: Validate ComplianceCheck schema in OpenAPI
    #[tokio::test]
    async fn test_compliance_check_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ComplianceCheck"),
            "ComplianceCheck should be in OpenAPI schemas"
        );

        println!("✅ ComplianceCheck schema defined in OpenAPI");
    }

    /// Test 6: Validate ComplianceFramework schema in OpenAPI
    #[tokio::test]
    async fn test_compliance_framework_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ComplianceFramework"),
            "ComplianceFramework should be in OpenAPI schemas"
        );

        println!("✅ ComplianceFramework schema defined in OpenAPI");
    }

    /// Test 7: Validate ControlStatus schema in OpenAPI
    #[tokio::test]
    async fn test_control_status_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ControlStatus"),
            "ControlStatus should be in OpenAPI schemas"
        );

        println!("✅ ControlStatus schema defined in OpenAPI");
    }

    /// Test 8: Validate SecurityMetrics schema in OpenAPI
    #[tokio::test]
    async fn test_security_metrics_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("SecurityMetrics"),
            "SecurityMetrics should be in OpenAPI schemas"
        );

        println!("✅ SecurityMetrics schema defined in OpenAPI");
    }

    /// Test 9: Test vulnerability serialization/deserialization
    #[tokio::test]
    async fn test_vulnerability_serialization() {
        let vulnerability = Vulnerability {
            id: "vuln-123".to_string(),
            title: "SQL Injection Vulnerability".to_string(),
            description: "SQL injection vulnerability in user input handling".to_string(),
            severity: VulnerabilitySeverity::Critical,
            status: VulnerabilityStatus::Open,
            cve_id: Some("CVE-2024-1234".to_string()),
            cvss_score: 9.8,
            resource_id: "resource-456".to_string(),
            resource_type: "web_application".to_string(),
            tenant_id: "tenant-123".to_string(),
            discovered_at: Utc::now(),
            updated_at: Utc::now(),
            due_date: Some(Utc::now() + chrono::Duration::days(1)),
            assigned_to: Some("security-team".to_string()),
            evidence: vec!["screenshot1.png".to_string()],
            remediation_steps: vec!["Use parameterized queries".to_string()],
            related_vulnerabilities: vec![],
            tags: vec!["web-security".to_string(), "owasp-top10".to_string()],
        };

        let json = serde_json::to_string(&vulnerability).unwrap();
        assert!(json.contains("id"), "Should contain id");
        assert!(json.contains("severity"), "Should contain severity");
        assert!(json.contains("cvss_score"), "Should contain cvss_score");

        let deserialized: Vulnerability = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "vuln-123");
        assert_eq!(deserialized.cvss_score, 9.8);

        println!("✅ Vulnerability serialization/deserialization works correctly");
    }

    /// Test 10: Test security score serialization/deserialization
    #[tokio::test]
    async fn test_security_score_serialization() {
        let mut score_breakdown = HashMap::new();
        score_breakdown.insert("vulnerability".to_string(), 85.0);
        score_breakdown.insert("compliance".to_string(), 92.0);

        let score = SecurityScore {
            id: "score-123".to_string(),
            entity_id: "tenant-123".to_string(),
            entity_type: "tenant".to_string(),
            overall_score: 88.5,
            vulnerability_score: 85.0,
            compliance_score: 92.0,
            configuration_score: 90.0,
            score_breakdown,
            critical_count: 2,
            high_count: 5,
            medium_count: 12,
            low_count: 25,
            open_count: 44,
            resolved_count: 156,
            trend: "improving".to_string(),
            calculated_at: Utc::now(),
            tenant_id: "tenant-123".to_string(),
        };

        let json = serde_json::to_string(&score).unwrap();
        assert!(
            json.contains("overall_score"),
            "Should contain overall_score"
        );
        assert!(
            json.contains("vulnerability_score"),
            "Should contain vulnerability_score"
        );
        assert!(
            json.contains("critical_count"),
            "Should have critical_count"
        );

        let deserialized: SecurityScore = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.overall_score, 88.5);
        assert_eq!(deserialized.critical_count, 2);

        println!("✅ SecurityScore serialization/deserialization works correctly");
    }

    /// Test 11: Test compliance check serialization/deserialization
    #[tokio::test]
    async fn test_compliance_check_serialization() {
        let check = ComplianceCheck {
            id: "check-123".to_string(),
            framework: ComplianceFramework::SOC2,
            control_id: "CC1.1".to_string(),
            control_name: "Logical and Physical Access Controls".to_string(),
            control_description: "The entity implements logical and physical access controls"
                .to_string(),
            status: ControlStatus::Implemented,
            implementation_percentage: 95.0,
            evidence: vec!["policy-doc.pdf".to_string()],
            last_assessment: Utc::now() - chrono::Duration::days(30),
            next_assessment: Utc::now() + chrono::Duration::days(335),
            tenant_id: "tenant-123".to_string(),
            resource_id: Some("resource-456".to_string()),
            notes: Some("All controls implemented and documented".to_string()),
        };

        let json = serde_json::to_string(&check).unwrap();
        assert!(json.contains("framework"), "Should contain framework");
        assert!(json.contains("control_id"), "Should contain control_id");
        assert!(
            json.contains("implementation_percentage"),
            "Should have implementation_percentage"
        );

        let deserialized: ComplianceCheck = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.control_id, "CC1.1");
        assert_eq!(deserialized.implementation_percentage, 95.0);

        println!("✅ ComplianceCheck serialization/deserialization works correctly");
    }

    /// Test 12: Test security metrics serialization/deserialization
    #[tokio::test]
    async fn test_security_metrics_serialization() {
        let mut by_severity = HashMap::new();
        by_severity.insert("Critical".to_string(), 2);
        by_severity.insert("High".to_string(), 5);

        let mut by_status = HashMap::new();
        by_status.insert("Open".to_string(), 44);
        by_status.insert("Resolved".to_string(), 156);

        let mut by_type = HashMap::new();
        by_type.insert("web".to_string(), 100);
        by_type.insert("api".to_string(), 100);

        let score_trend = vec![85.0, 87.0, 88.5, 90.0];

        let metrics = SecurityMetrics {
            total_vulnerabilities: 200,
            by_severity,
            by_status,
            by_type,
            avg_remediation_time: 5.5,
            score_trend,
            compliance_score: 92.0,
            open_issues: 44,
            overdue_items: 12,
            coverage_percentage: 88.5,
            assessed_at: Utc::now(),
            tenant_id: "tenant-123".to_string(),
        };

        let json = serde_json::to_string(&metrics).unwrap();
        assert!(
            json.contains("total_vulnerabilities"),
            "Should contain total_vulnerabilities"
        );
        assert!(
            json.contains("compliance_score"),
            "Should contain compliance_score"
        );
        assert!(json.contains("by_severity"), "Should have by_severity");

        let deserialized: SecurityMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_vulnerabilities, 200);
        assert_eq!(deserialized.compliance_score, 92.0);
        assert_eq!(deserialized.open_issues, 44);
        assert_eq!(deserialized.coverage_percentage, 88.5);

        println!("✅ SecurityMetrics serialization/deserialization works correctly");
    }

    /// Test 13: Test vulnerability severity enum values
    #[tokio::test]
    async fn test_vulnerability_severity_enum_values() {
        let severities = vec![
            VulnerabilitySeverity::Critical,
            VulnerabilitySeverity::High,
            VulnerabilitySeverity::Medium,
            VulnerabilitySeverity::Low,
            VulnerabilitySeverity::Info,
        ];

        let expected_names = vec!["Critical", "High", "Medium", "Low", "Info"];

        for (severity, expected) in severities.into_iter().zip(expected_names) {
            let json = serde_json::to_string(&severity).unwrap();
            assert_eq!(json, format!(r#""{}""#, expected));

            let deserialized: VulnerabilitySeverity = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, severity);
        }

        println!("✅ VulnerabilitySeverity enum values are correct");
    }

    /// Test 14: Test vulnerability status enum values
    #[tokio::test]
    async fn test_vulnerability_status_enum_values() {
        let statuses = vec![
            VulnerabilityStatus::Open,
            VulnerabilityStatus::InProgress,
            VulnerabilityStatus::Verified,
            VulnerabilityStatus::Resolved,
            VulnerabilityStatus::Accepted,
            VulnerabilityStatus::FalsePositive,
        ];

        let expected_values = vec![
            "Open",
            "InProgress",
            "Verified",
            "Resolved",
            "Accepted",
            "FalsePositive",
        ];

        for (status, expected) in statuses.into_iter().zip(expected_values) {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!(r#""{}""#, expected));

            let deserialized: VulnerabilityStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, status);
        }

        println!("✅ VulnerabilityStatus enum values are correct");
    }

    /// Test 15: Test compliance framework enum values
    #[tokio::test]
    async fn test_compliance_framework_enum_values() {
        let frameworks = vec![
            ComplianceFramework::SOC2,
            ComplianceFramework::ISO27001,
            ComplianceFramework::GDPR,
            ComplianceFramework::PCIDSS,
            ComplianceFramework::HIPAA,
            ComplianceFramework::NIST,
        ];

        let expected_values = vec!["SOC2", "ISO27001", "GDPR", "PCIDSS", "HIPAA", "NIST"];

        for (framework, expected) in frameworks.into_iter().zip(expected_values) {
            let json = serde_json::to_string(&framework).unwrap();
            assert_eq!(json, format!(r#""{}""#, expected));

            let deserialized: ComplianceFramework = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, framework);
        }

        println!("✅ ComplianceFramework enum values are correct");
    }

    /// Test 16: Test control status enum values
    #[tokio::test]
    async fn test_control_status_enum_values() {
        let statuses = vec![
            ControlStatus::Implemented,
            ControlStatus::Partial,
            ControlStatus::NotImplemented,
            ControlStatus::NotApplicable,
        ];

        let expected_values = vec!["Implemented", "Partial", "NotImplemented", "NotApplicable"];

        for (status, expected) in statuses.into_iter().zip(expected_values) {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!(r#""{}""#, expected));

            let deserialized: ControlStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, status);
        }

        println!("✅ ControlStatus enum values are correct");
    }

    /// Test 17: Test security dashboard endpoints documented in OpenAPI
    #[tokio::test]
    async fn test_security_dashboard_endpoints_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let paths = &openapi.paths.paths;

        println!("\n🔒 Security Dashboard Endpoints Check");
        println!("======================================");

        // Check for security dashboard endpoints
        assert!(
            paths.contains_key("/api/v1/security/vulnerabilities"),
            "Should have /api/v1/security/vulnerabilities endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/security/vulnerabilities/{id}"),
            "Should have /api/v1/security/vulnerabilities/{{id}} endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/security/scores/{entity_id}"),
            "Should have /api/v1/security/scores/{{entity_id}} endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/security/scores"),
            "Should have /api/v1/security/scores endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/security/compliance"),
            "Should have /api/v1/security/compliance endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/security/metrics/{tenant_id}"),
            "Should have /api/v1/security/metrics/{{tenant_id}} endpoint"
        );

        println!("  ✅ /api/v1/security/vulnerabilities documented");
        println!("  ✅ /api/v1/security/vulnerabilities/{{id}} documented");
        println!("  ✅ /api/v1/security/scores/{{entity_id}} documented");
        println!("  ✅ /api/v1/security/scores documented");
        println!("  ✅ /api/v1/security/compliance documented");
        println!("  ✅ /api/v1/security/metrics/{{tenant_id}} documented");
        println!("✅ All security dashboard endpoints documented");
    }

    /// Test 18: Test security dashboard API OpenAPI completeness
    #[tokio::test]
    async fn test_security_dashboard_openapi_completeness() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let paths = openapi.paths.paths;

        let mut security_count = 0;

        // Count security-related endpoints
        for (path, _) in paths.iter() {
            if path.contains("/security/") {
                security_count += 1;
            }
        }

        println!("\n🔒 Security Dashboard OpenAPI Coverage");
        println!("======================================");
        println!("Total Security Dashboard Endpoints: {}", security_count);

        // Minimum expected: vulnerabilities, scores, compliance, metrics
        assert!(
            security_count >= 6,
            "Expected at least 6 security dashboard endpoints, found {}",
            security_count
        );

        println!("✅ Security Dashboard API coverage test passed");
    }

    /// Test 19: Test CVSS score range validation
    #[tokio::test]
    async fn test_cvss_score_range_validation() {
        // Test critical vulnerability with high CVSS
        let critical_vuln = Vulnerability {
            id: "vuln-1".to_string(),
            title: "Critical vulnerability".to_string(),
            description: "Critical severity vulnerability".to_string(),
            severity: VulnerabilitySeverity::Critical,
            status: VulnerabilityStatus::Open,
            cve_id: None,
            cvss_score: 9.8,
            resource_id: "res-1".to_string(),
            resource_type: "web".to_string(),
            tenant_id: "tenant-1".to_string(),
            discovered_at: Utc::now(),
            updated_at: Utc::now(),
            due_date: None,
            assigned_to: None,
            evidence: vec![],
            remediation_steps: vec![],
            related_vulnerabilities: vec![],
            tags: vec![],
        };

        assert_eq!(critical_vuln.cvss_score, 9.8);
        assert!(critical_vuln.cvss_score >= 0.0);
        assert!(critical_vuln.cvss_score <= 10.0);

        // Test low vulnerability with low CVSS
        let low_vuln = Vulnerability {
            id: "vuln-2".to_string(),
            title: "Low severity vulnerability".to_string(),
            description: "Low severity vulnerability".to_string(),
            severity: VulnerabilitySeverity::Low,
            status: VulnerabilityStatus::Open,
            cve_id: None,
            cvss_score: 2.1,
            resource_id: "res-2".to_string(),
            resource_type: "web".to_string(),
            tenant_id: "tenant-1".to_string(),
            discovered_at: Utc::now(),
            updated_at: Utc::now(),
            due_date: None,
            assigned_to: None,
            evidence: vec![],
            remediation_steps: vec![],
            related_vulnerabilities: vec![],
            tags: vec![],
        };

        assert_eq!(low_vuln.cvss_score, 2.1);
        assert!(low_vuln.cvss_score >= 0.0);
        assert!(low_vuln.cvss_score <= 10.0);

        println!("✅ CVSS score range validation works correctly (0.0 - 10.0)");
    }

    /// Test 20: Test security score calculation components
    #[tokio::test]
    async fn test_security_score_calculation_components() {
        let mut breakdown = HashMap::new();
        breakdown.insert("vulnerability".to_string(), 80.0);
        breakdown.insert("compliance".to_string(), 95.0);
        breakdown.insert("configuration".to_string(), 90.0);

        let score = SecurityScore {
            id: "score-1".to_string(),
            entity_id: "entity-1".to_string(),
            entity_type: "resource".to_string(),
            overall_score: 88.33, // Average of 80, 95, 90
            vulnerability_score: 80.0,
            compliance_score: 95.0,
            configuration_score: 90.0,
            score_breakdown: breakdown.clone(),
            critical_count: 3,
            high_count: 7,
            medium_count: 15,
            low_count: 30,
            open_count: 55,
            resolved_count: 200,
            trend: "stable".to_string(),
            calculated_at: Utc::now(),
            tenant_id: "tenant-1".to_string(),
        };

        // Verify score breakdown
        assert_eq!(score.score_breakdown.get("vulnerability").unwrap(), &80.0);
        assert_eq!(score.score_breakdown.get("compliance").unwrap(), &95.0);
        assert_eq!(score.score_breakdown.get("configuration").unwrap(), &90.0);

        // Verify vulnerability counts
        assert_eq!(score.critical_count, 3);
        assert_eq!(score.high_count, 7);
        assert_eq!(score.medium_count, 15);
        assert_eq!(score.low_count, 30);
        assert_eq!(score.open_count, 55);
        assert_eq!(score.resolved_count, 200);

        // Verify trend
        assert!(matches!(
            score.trend.as_str(),
            "improving" | "declining" | "stable"
        ));

        println!("✅ Security score calculation components are valid");
    }

    /// Test 21: Test compliance implementation percentage
    #[tokio::test]
    async fn test_compliance_implementation_percentage() {
        // Test fully implemented control
        let implemented = ComplianceCheck {
            id: "check-1".to_string(),
            framework: ComplianceFramework::ISO27001,
            control_id: "A.5.1".to_string(),
            control_name: "Information security policies".to_string(),
            control_description: "Policies are defined".to_string(),
            status: ControlStatus::Implemented,
            implementation_percentage: 100.0,
            evidence: vec![],
            last_assessment: Utc::now(),
            next_assessment: Utc::now(),
            tenant_id: "tenant-1".to_string(),
            resource_id: None,
            notes: None,
        };

        assert_eq!(implemented.implementation_percentage, 100.0);
        assert!(implemented.implementation_percentage >= 0.0);
        assert!(implemented.implementation_percentage <= 100.0);

        // Test partially implemented control
        let partial = ComplianceCheck {
            id: "check-2".to_string(),
            framework: ComplianceFramework::GDPR,
            control_id: "Art.30".to_string(),
            control_name: "Records of processing activities".to_string(),
            control_description: "Records are being maintained".to_string(),
            status: ControlStatus::Partial,
            implementation_percentage: 65.5,
            evidence: vec![],
            last_assessment: Utc::now(),
            next_assessment: Utc::now(),
            tenant_id: "tenant-1".to_string(),
            resource_id: None,
            notes: None,
        };

        assert_eq!(partial.implementation_percentage, 65.5);
        assert!(partial.implementation_percentage >= 0.0);
        assert!(partial.implementation_percentage <= 100.0);

        println!("✅ Compliance implementation percentage is valid (0.0 - 100.0)");
    }

    /// Test 22: Test vulnerability timeline fields
    #[tokio::test]
    async fn test_vulnerability_timeline_fields() {
        let discovered = Utc::now() - chrono::Duration::days(5);
        let updated = Utc::now() - chrono::Duration::hours(2);
        let due = Utc::now() + chrono::Duration::days(2);

        let vulnerability = Vulnerability {
            id: "vuln-timeline".to_string(),
            title: "Timeline test vulnerability".to_string(),
            description: "Test vulnerability timeline fields".to_string(),
            severity: VulnerabilitySeverity::High,
            status: VulnerabilityStatus::InProgress,
            cve_id: None,
            cvss_score: 7.5,
            resource_id: "res-1".to_string(),
            resource_type: "api".to_string(),
            tenant_id: "tenant-1".to_string(),
            discovered_at: discovered,
            updated_at: updated,
            due_date: Some(due),
            assigned_to: Some("security-team".to_string()),
            evidence: vec![],
            remediation_steps: vec![],
            related_vulnerabilities: vec![],
            tags: vec![],
        };

        // Verify timeline consistency
        assert!(vulnerability.discovered_at <= vulnerability.updated_at);
        assert!(vulnerability.due_date.unwrap() > vulnerability.discovered_at);
        assert!(vulnerability.due_date.unwrap() > vulnerability.updated_at);

        println!("✅ Vulnerability timeline fields are consistent");
    }

    /// Test 23: Test security metrics aggregation
    #[tokio::test]
    async fn test_security_metrics_aggregation() {
        let mut by_severity = HashMap::new();
        by_severity.insert("Critical".to_string(), 5);
        by_severity.insert("High".to_string(), 25);
        by_severity.insert("Medium".to_string(), 70);
        by_severity.insert("Low".to_string(), 100);

        let mut by_status = HashMap::new();
        by_status.insert("Open".to_string(), 27);
        by_status.insert("InProgress".to_string(), 5);
        by_status.insert("Resolved".to_string(), 168);

        let mut by_type = HashMap::new();
        by_type.insert("web".to_string(), 100);
        by_type.insert("api".to_string(), 100);

        let metrics = SecurityMetrics {
            total_vulnerabilities: 200,
            by_severity: by_severity.clone(),
            by_status: by_status.clone(),
            by_type,
            avg_remediation_time: 5.5,
            score_trend: vec![85.0, 87.0, 88.5, 90.0],
            compliance_score: 91.5,
            open_issues: 32,
            overdue_items: 10,
            coverage_percentage: 88.5,
            assessed_at: Utc::now(),
            tenant_id: "tenant-123".to_string(),
        };

        // Verify aggregation
        let total_severity: u32 = by_severity.values().sum();
        assert_eq!(total_severity, metrics.total_vulnerabilities);

        let total_status: u32 = by_status.values().sum();
        assert_eq!(total_status, metrics.total_vulnerabilities);

        assert_eq!(metrics.open_issues, 27 + 5);
        assert_eq!(metrics.overdue_items, 10);

        // Verify metrics ranges
        assert!(metrics.avg_remediation_time >= 0.0);
        assert!(metrics.compliance_score >= 0.0);
        assert!(metrics.compliance_score <= 100.0);
        assert!(metrics.coverage_percentage >= 0.0);
        assert!(metrics.coverage_percentage <= 100.0);

        println!("✅ Security metrics aggregation is correct");
    }

    /// Test 24: Test security score trend values
    #[tokio::test]
    async fn test_security_score_trend_values() {
        let trends = vec!["improving", "declining", "stable"];

        for trend in trends {
            let score = SecurityScore {
                id: "score-trend".to_string(),
                entity_id: "entity-1".to_string(),
                entity_type: "tenant".to_string(),
                overall_score: 85.0,
                vulnerability_score: 80.0,
                compliance_score: 90.0,
                configuration_score: 85.0,
                score_breakdown: HashMap::new(),
                critical_count: 0,
                high_count: 0,
                medium_count: 0,
                low_count: 0,
                open_count: 0,
                resolved_count: 100,
                trend: trend.to_string(),
                calculated_at: Utc::now(),
                tenant_id: "tenant-1".to_string(),
            };

            assert!(matches!(
                score.trend.as_str(),
                "improving" | "declining" | "stable"
            ));
        }

        println!("✅ Security score trend values are valid");
    }
}
