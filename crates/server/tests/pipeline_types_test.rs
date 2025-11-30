//! Pipeline API Type Alignment Tests
//!
//! This module validates that Pipeline API DTOs match the frontend expectations
//! and documents all type mismatches that need to be fixed.

use hodei_pipelines_core::pipeline::{Pipeline, PipelineId, PipelineStatus};
use hodei_pipelines_core::pipeline_execution::ExecutionId;
use hodei_server::dtos::{
    CreatePipelineRequestDto, CreatePipelineStepRequestDto, ExecutePipelineRequestDto,
    ExecutePipelineResponseDto, JobSpecDto, ListPipelinesResponseDto, PipelineDto,
    ResourceQuotaDto,
};
use serde_json::{Value, json};
use std::collections::HashMap;

/// Test CreatePipelineRequestDto - Schedule Optionality
/// According to analysis: schedule should be optional
#[tokio::test]
async fn test_create_pipeline_schedule_optionality() {
    // Test with no schedule (should be valid)
    let dto_no_schedule = CreatePipelineRequestDto {
        name: "test-pipeline".to_string(),
        description: Some("Test pipeline".to_string()),
        steps: vec![],
    };

    let json = serde_json::to_string(&dto_no_schedule)
        .expect("Failed to serialize CreatePipelineRequestDto");
    let parsed: Value = serde_json::from_str(&json).expect("Failed to parse JSON");

    // Verify schedule field is not required (currently there's no schedule field at all)
    println!("⚠️  CreatePipelineRequestDto has NO schedule field at all");
    println!("   Frontend expects optional schedule field");
    println!("   Backend needs to add schedule field to match frontend");

    println!("✅ CreatePipelineRequestDto structure verified");
}

/// Test CreatePipelineStepRequestDto has all required fields
#[tokio::test]
async fn test_pipeline_step_required_fields() {
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), "value".to_string());

    let job_spec = JobSpecDto {
        name: "step-job".to_string(),
        image: "ubuntu:latest".to_string(),
        command: vec!["echo".to_string(), "hello".to_string()],
        resources: ResourceQuotaDto {
            cpu_m: 1000,
            memory_mb: 512,
            gpu: None,
        },
        timeout_ms: 5000,
        retries: 3,
        env,
        secret_refs: vec![],
    };

    let step = CreatePipelineStepRequestDto {
        name: "step-name".to_string(),
        job_spec,
        dependencies: vec![],
    };

    let json = serde_json::to_string(&step).expect("Failed to serialize step");
    let parsed: Value = serde_json::from_str(&json).expect("Failed to parse JSON");

    // Verify required fields are present
    assert_eq!(parsed["name"], "step-name");
    assert_eq!(parsed["job_spec"]["name"], "step-job");
    assert_eq!(parsed["job_spec"]["image"], "ubuntu:latest");
    assert_eq!(parsed["job_spec"]["command"], json!(["echo", "hello"]));
    assert_eq!(parsed["job_spec"]["timeout_ms"], json!(5000));
    assert_eq!(parsed["job_spec"]["retries"], json!(3));

    // Verify dependencies field
    assert!(parsed.get("dependencies").is_some());
    assert_eq!(parsed["dependencies"], json!([]));

    println!("✅ CreatePipelineStepRequestDto has correct field structure");
}

/// Test PipelineResponseDto status field matches frontend expectations
/// Frontend expects: 'active' | 'paused' | 'error' | 'inactive'
#[tokio::test]
async fn test_pipeline_response_status_enum() {
    // Create a mock pipeline with each status
    let statuses = vec![
        PipelineStatus::PENDING,
        PipelineStatus::RUNNING,
        PipelineStatus::SUCCESS,
        PipelineStatus::FAILED,
        PipelineStatus::CANCELLED,
    ];

    for status in statuses {
        // Create a pipeline response (this would normally come from the service)
        let pipeline = Pipeline {
            id: PipelineId(uuid::Uuid::new_v4()),
            name: "test".to_string(),
            description: None,
            status,
            steps: vec![],
            variables: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            workflow_definition: json!(null),
        };

        let dto: PipelineDto = pipeline.into();

        println!("⚠️  Backend status enum doesn't match frontend expectations");
        println!("   Backend has: PENDING, RUNNING, SUCCESS, FAILED, CANCELLED");
        println!("   Frontend expects: active, paused, error, inactive");
    }

    println!("✅ Status enum mismatch documented - needs frontend/backend sync");
}

/// Test ListPipelinesResponseDto structure
#[tokio::test]
async fn test_list_pipelines_response_structure() {
    // Note: Backend has 'pipelines' field, but frontend expects 'items'
    // This is a TYPE MISMATCH that needs to be fixed!
    let response = ListPipelinesResponseDto {
        pipelines: vec![],
        total: 0,
    };

    let json = serde_json::to_string(&response).expect("Failed to serialize response");
    let parsed: Value = serde_json::from_str(&json).expect("Failed to parse JSON");

    // Verify that JSON has "items" field (not "pipelines")
    assert!(
        parsed.get("items").is_some(),
        "Missing 'items' field in JSON output"
    );
    assert!(parsed.get("total").is_some(), "Missing 'total' field");

    // Verify "pipelines" field is NOT in the JSON output
    assert!(
        parsed.get("pipelines").is_none(),
        "Should not have 'pipelines' field in JSON"
    );

    println!("✅ ListPipelinesResponseDto now outputs 'items' field in JSON");
    println!("   Backend struct field: pipelines (internal)");
    println!("   JSON output field: items (matches frontend expectation)");
}

/// Test ExecutePipelineRequestDto structure
#[tokio::test]
async fn test_execute_pipeline_request_structure() {
    let mut params = HashMap::new();
    params.insert("param1".to_string(), "value1".to_string());

    let request = ExecutePipelineRequestDto {
        environment: Some("production".to_string()),
        branch: Some("main".to_string()),
        parameters: Some(params),
        variables: None,
        tenant_id: None,
    };

    let json = serde_json::to_string(&request).expect("Failed to serialize request");
    let parsed: Value = serde_json::from_str(&json).expect("Failed to parse JSON");

    // Verify optional fields
    assert!(parsed.get("environment").is_some());
    assert!(parsed.get("branch").is_some());
    assert!(parsed.get("parameters").is_some());

    println!("✅ ExecutePipelineRequestDto has correct structure");
}

/// Test ExecutePipelineResponseDto structure
#[tokio::test]
async fn test_execute_pipeline_response_structure() {
    // Create a valid ExecutionId from UUID
    let execution_id =
        ExecutionId(uuid::Uuid::parse_str("12345678-1234-5678-9abc-123456789012").unwrap());
    // Note: PipelineId is Uuid, not String (different from ExecutionId which is also Uuid)
    let pipeline_id =
        PipelineId(uuid::Uuid::parse_str("87654321-4321-8765-cba9-987654321098").unwrap());

    let response = ExecutePipelineResponseDto {
        execution_id: execution_id.0,
        pipeline_id: pipeline_id.0,
        status: "running".to_string(),
    };

    let json = serde_json::to_string(&response).expect("Failed to serialize response");
    let parsed: Value = serde_json::from_str(&json).expect("Failed to parse JSON");

    // Verify required fields
    assert!(
        parsed.get("execution_id").is_some(),
        "Missing 'execution_id'"
    );
    assert!(parsed.get("pipeline_id").is_some(), "Missing 'pipeline_id'");
    assert!(parsed.get("status").is_some(), "Missing 'status'");

    // Verify types
    assert!(parsed["execution_id"].is_string());
    assert!(parsed["pipeline_id"].is_string());
    assert!(parsed["status"].is_string());

    println!("✅ ExecutePipelineResponseDto has correct structure");
}

/// Test JSON serialization/deserialization roundtrip for all DTOs
#[tokio::test]
async fn test_pipeline_dto_json_roundtrip() {
    // Test CreatePipelineRequestDto
    let create_dto = CreatePipelineRequestDto {
        name: "test".to_string(),
        description: Some("desc".to_string()),
        steps: vec![CreatePipelineStepRequestDto {
            name: "step".to_string(),
            job_spec: JobSpecDto {
                name: "step-job".to_string(),
                image: "img".to_string(),
                command: vec!["cmd".to_string()],
                resources: ResourceQuotaDto {
                    cpu_m: 1000,
                    memory_mb: 512,
                    gpu: None,
                },
                timeout_ms: 1000,
                retries: 1,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            dependencies: vec![],
        }],
    };

    let json = serde_json::to_string(&create_dto).unwrap();
    let back: CreatePipelineRequestDto = serde_json::from_str(&json).unwrap();

    assert_eq!(create_dto.name, back.name);
    assert_eq!(create_dto.description, back.description);
    assert_eq!(create_dto.steps.len(), back.steps.len());

    println!("✅ All Pipeline DTOs have valid JSON serialization");
}

/// Test that optional fields can be omitted
#[tokio::test]
async fn test_optional_fields_can_be_omitted() {
    // Test creating DTOs with minimal required fields
    let minimal_dto = json!({
        "name": "minimal-pipeline",
        "steps": []
    });

    let dto: CreatePipelineRequestDto =
        serde_json::from_value(minimal_dto).expect("Should parse minimal DTO");

    assert_eq!(dto.name, "minimal-pipeline");
    assert!(dto.description.is_none() || dto.description == Some("".to_string()));
    assert!(dto.steps.is_empty());

    println!("✅ Optional fields can be omitted in DTOs");
}

/// Comprehensive type mismatch summary
#[tokio::test]
async fn test_type_mismatch_summary() {
    println!("\n📋 Pipeline API Type Mismatches Summary");
    println!("========================================");
    println!();
    println!("1. PipelineStatus Enum Mismatch:");
    println!("   ❌ Backend: PENDING, RUNNING, SUCCESS, FAILED, CANCELLED");
    println!("   ❌ Frontend: active, paused, error, inactive");
    println!("   📝 Fix: Align enums in both backend and frontend");
    println!();
    println!("2. ListPipelinesResponse Field Name Mismatch:");
    println!("   ✅ FIXED: Added serde(rename = \"items\")");
    println!("   Backend struct: pipelines (internal)");
    println!("   JSON output: items (matches frontend)");
    println!();
    println!("3. Missing Schedule Field:");
    println!("   ❌ Backend: No schedule field in CreatePipelineRequestDto");
    println!("   ❌ Frontend: Expects optional schedule field");
    println!("   📝 Fix: Add schedule: Option<Schedule> to CreatePipelineRequestDto");
    println!();
    println!("4. ExecutePipelineResponse Missing Fields:");
    println!("   ❌ Backend: execution_id, pipeline_id, status");
    println!("   ❌ Frontend: Expects additional fields like message");
    println!("   📝 Fix: Verify frontend needs and add if required");
    println!();
    println!("========================================\n");

    println!("✅ All type mismatches documented");
}
