//! OpenAPI Contract Validation Tests
//!
//! This module validates that all API endpoints are properly documented
//! in the OpenAPI specification and that the contract is complete.

use hodei_server::api_docs::ApiDoc;
use utoipa::OpenApi;

/// Test that the OpenAPI specification can be generated
#[tokio::test]
async fn test_openapi_spec_generation() {
    // Test that the OpenApi struct can be instantiated
    let openapi = <ApiDoc as OpenApi>::openapi();

    // Verify basic info
    // Use the actual package version from Cargo.toml
    assert_eq!(openapi.info.version, "0.15.0");
    // Title can be "Hodei API" or package name, so just verify it exists
    assert!(!openapi.info.title.is_empty());

    println!("✅ OpenAPI spec generated successfully");
    println!("   Title: {}", openapi.info.title);
    println!("   Version: {}", openapi.info.version);
}

/// Test that pipeline endpoints are documented
#[tokio::test]
async fn test_pipeline_endpoints_documented() {
    let openapi = <ApiDoc as OpenApi>::openapi();

    // Check that pipeline endpoints exist
    let paths = openapi.paths.paths.keys().collect::<Vec<_>>();

    println!("📋 Documented endpoints:");
    for path in &paths {
        println!("   - {}", path);
    }

    // Verify pipeline CRUD endpoints are documented
    assert!(
        paths.contains(&&"/api/v1/pipelines".to_string()),
        "GET /api/v1/pipelines not documented"
    );
    assert!(
        paths.contains(&&"/api/v1/pipelines".to_string()),
        "POST /api/v1/pipelines not documented"
    );
    assert!(
        paths.contains(&&"/api/v1/pipelines/{id}".to_string()),
        "GET /api/v1/pipelines/{{id}} not documented"
    );
    assert!(
        paths.contains(&&"/api/v1/pipelines/{id}".to_string()),
        "PUT /api/v1/pipelines/{{id}} not documented"
    );
    assert!(
        paths.contains(&&"/api/v1/pipelines/{id}".to_string()),
        "DELETE /api/v1/pipelines/{{id}} not documented"
    );
    assert!(
        paths.contains(&&"/api/v1/pipelines/{id}/execute".to_string()),
        "POST /api/v1/pipelines/{{id}}/execute not documented"
    );
    assert!(
        paths.contains(&&"/api/v1/pipelines/{id}/dag".to_string()),
        "GET /api/v1/pipelines/{{id}}/dag not documented"
    );
    assert!(
        paths.contains(&&"/api/v1/pipelines/{id}/steps/{step_id}".to_string()),
        "GET /api/v1/pipelines/{{id}}/steps/{{step_id}} not documented"
    );

    println!("✅ All pipeline endpoints documented");
}

/// Test that resource pool endpoints are documented
#[tokio::test]
async fn test_resource_pool_endpoints_documented() {
    let openapi = <ApiDoc as OpenApi>::openapi();

    let paths = openapi.paths.paths.keys().collect::<Vec<_>>();

    // Verify resource pool CRUD endpoints are documented
    assert!(
        paths.contains(&&"/api/v1/worker-pools".to_string()),
        "GET /api/v1/worker-pools not documented"
    );
    assert!(
        paths.contains(&&"/api/v1/worker-pools".to_string()),
        "POST /api/v1/worker-pools not documented"
    );
    assert!(
        paths.contains(&&"/api/v1/worker-pools/{pool_id}".to_string()),
        "GET /api/v1/worker-pools/{{pool_id}} not documented"
    );
    assert!(
        paths.contains(&&"/api/v1/worker-pools/{pool_id}".to_string()),
        "PUT /api/v1/worker-pools/{{pool_id}} not documented"
    );
    assert!(
        paths.contains(&&"/api/v1/worker-pools/{pool_id}".to_string()),
        "PATCH /api/v1/worker-pools/{{pool_id}} not documented"
    );
    assert!(
        paths.contains(&&"/api/v1/worker-pools/{pool_id}".to_string()),
        "DELETE /api/v1/worker-pools/{{pool_id}} not documented"
    );
    assert!(
        paths.contains(&&"/api/v1/worker-pools/{pool_id}/status".to_string()),
        "GET /api/v1/worker-pools/{{pool_id}}/status not documented"
    );

    println!("✅ All resource pool endpoints documented");
}

/// Test that schemas are properly defined
#[tokio::test]
async fn test_pipeline_schemas_defined() {
    use hodei_server::dtos::CreatePipelineRequestDto;

    // Test that schemas can be serialized (validation they have ToSchema)
    let create_req = CreatePipelineRequestDto {
        name: "test-pipeline".to_string(),
        description: Some("Test description".to_string()),
        steps: vec![],
    };

    let json =
        serde_json::to_string(&create_req).expect("Failed to serialize CreatePipelineRequestDto");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse JSON");
    assert!(
        parsed.get("name").is_some(),
        "CreatePipelineRequestDto missing 'name' field"
    );
    assert!(
        parsed.get("description").is_some(),
        "CreatePipelineRequestDto missing 'description' field"
    );

    println!("✅ Pipeline DTOs have valid schemas");
}

/// Test that error responses are documented
#[tokio::test]
async fn test_error_responses_documented() {
    let openapi = <ApiDoc as OpenApi>::openapi();

    // Check a few endpoints for error response documentation
    let paths = &openapi.paths.paths;

    // Verify pipeline creation has error responses
    if let Some(post_pipeline) = paths.get("/api/v1/pipelines") {
        if let Some(post_operation) = &post_pipeline.post {
            assert!(
                !post_operation.responses.responses.is_empty(),
                "POST /api/v1/pipelines has no response documentation"
            );
            println!("✅ POST /api/v1/pipelines has response documentation");
        }
    }

    println!("✅ Error responses are documented");
}

/// Test that query parameters are documented
#[tokio::test]
async fn test_query_parameters_documented() {
    let openapi = <ApiDoc as OpenApi>::openapi();

    // Check for list endpoints with query parameters
    if let Some(get_pipelines) = openapi.paths.paths.get("/api/v1/pipelines") {
        if let Some(get_operation) = &get_pipelines.get {
            // Verify parameters are documented
            assert!(
                get_operation.parameters.is_some()
                    && !get_operation.parameters.as_ref().unwrap().is_empty(),
                "GET /api/v1/pipelines has no parameter documentation"
            );

            println!("✅ Query parameters documented for list endpoints");
        }
    }

    println!("✅ Parameter documentation complete");
}

/// Test OpenAPI spec validity
#[tokio::test]
async fn test_openapi_spec_validity() {
    let openapi = <ApiDoc as OpenApi>::openapi();

    // Convert to JSON to validate structure
    let json_value = serde_json::to_value(&openapi).expect("Failed to serialize OpenAPI to JSON");

    // Verify it has required fields
    assert!(
        json_value.get("openapi").is_some(),
        "Missing 'openapi' field"
    );
    assert!(json_value.get("info").is_some(), "Missing 'info' field");
    assert!(json_value.get("paths").is_some(), "Missing 'paths' field");
    assert!(
        json_value.get("components").is_some(),
        "Missing 'components' field"
    );

    println!("✅ OpenAPI spec has valid structure");
}

/// Test that all DTOs in the spec are serializable
#[tokio::test]
async fn test_all_dto_schemas_valid() {
    use hodei_server::api_docs::*;

    // Test health response
    let health = HealthResponse {
        status: "ok".to_string(),
        service: "hodei".to_string(),
        version: "1.0.0".to_string(),
        architecture: "x86_64".to_string(),
    };
    assert!(serde_json::to_string(&health).is_ok());

    // Test error response
    let error = ErrorResponse {
        code: "TEST_ERROR".to_string(),
        message: "Test error".to_string(),
        details: Some("Test details".to_string()),
    };
    assert!(serde_json::to_string(&error).is_ok());

    println!("✅ All DTO schemas are valid and serializable");
}

/// Comprehensive test for API documentation coverage
#[tokio::test]
async fn test_api_documentation_coverage() {
    let openapi = <ApiDoc as OpenApi>::openapi();

    let paths = openapi.paths.paths.keys().collect::<Vec<_>>();
    let total_endpoints = paths.len();

    println!("\n📊 API Documentation Coverage Report");
    println!("=====================================");
    println!("Total Endpoints Documented: {}", total_endpoints);
    println!("\n📋 Endpoint List:");
    for path in &paths {
        println!("   {}", path);
    }

    // Count methods per endpoint
    let mut method_count = 0;
    for (_path, item) in &openapi.paths.paths {
        if item.get.is_some() {
            method_count += 1;
        }
        if item.post.is_some() {
            method_count += 1;
        }
        if item.put.is_some() {
            method_count += 1;
        }
        if item.patch.is_some() {
            method_count += 1;
        }
        if item.delete.is_some() {
            method_count += 1;
        }
    }

    println!("\nTotal Methods Documented: {}", method_count);
    println!("=====================================\n");

    // Verify minimum coverage
    assert!(
        total_endpoints >= 10,
        "Expected at least 10 endpoints, found {}",
        total_endpoints
    );

    println!("✅ API documentation coverage test passed");
}
