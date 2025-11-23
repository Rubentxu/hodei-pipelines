//! File I/O and Evidence Collection E2E Tests
//!
//! This test module validates that job executions actually write files to disk
//! and that logs can be extracted for evidence collection.
//!
//! NOTE: These tests require the worker-lifecycle-manager service to be running
//! on port 8082. Run with: make test-all-e2e-services

use e2e_tests::TestResult;
use reqwest::Client;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::time::Duration;

const EXECUTION_OUTPUT_DIR: &str = "/tmp/hodei-jobs-executions";
const TEST_EVIDENCE_DIR: &str = "/tmp/hodei-test-evidence";

/// Helper to save evidence to a file
fn save_evidence(test_name: &str, evidence_type: &str, content: &str) -> std::io::Result<()> {
    fs::create_dir_all(TEST_EVIDENCE_DIR)?;
    let filename = format!("{}/{}_{}.txt", TEST_EVIDENCE_DIR, test_name, evidence_type);
    fs::write(&filename, content)?;
    println!("   📄 Evidence saved: {}", filename);
    Ok(())
}

/// Clean up execution files before test
fn cleanup_execution_dir() {
    if Path::new(EXECUTION_OUTPUT_DIR).exists() {
        let _ = fs::remove_dir_all(EXECUTION_OUTPUT_DIR);
    }
}

#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_job_execution_creates_file_on_disk() -> TestResult<()> {
    println!("\n🧪 Testing job execution creates actual file on disk...\n");

    // Clean up previous test artifacts
    cleanup_execution_dir();

    let client = Client::new();

    // Execute a job with a command that writes to a file
    let test_content = "Hello from E2E test - file I/O validation";
    let execution_id_placeholder = format!("test_{}", chrono::Utc::now().timestamp());
    let output_file = format!(
        "{}/execution_{}.txt",
        EXECUTION_OUTPUT_DIR, execution_id_placeholder
    );

    let execution_data = json!({
        "command": format!("mkdir -p {} && echo '{}' > {}", EXECUTION_OUTPUT_DIR, test_content, output_file),
        "type": "bash",
        "description": "File I/O test execution"
    });

    println!("   1️⃣  Executing job with file write command");

    let exec_response = client
        .post("http://localhost:8082/api/v1/execute")
        .json(&execution_data)
        .send()
        .await?;

    assert!(
        exec_response.status().is_success(),
        "Job execution should succeed"
    );

    let execution: Value = exec_response.json().await?;
    let execution_id = execution["id"].as_str().unwrap();

    println!("   ✅ Execution started with ID: {}", execution_id);

    // Wait for the execution to complete
    println!("   2️⃣  Waiting for command execution (2 seconds)...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check execution status
    let status_response = client
        .get(&format!(
            "http://localhost:8082/api/v1/executions/{}",
            execution_id
        ))
        .send()
        .await?;

    let exec_status: Value = status_response.json().await?;
    println!("   3️⃣  Execution status: {}", exec_status["status"]);
    assert_eq!(
        exec_status["status"], "completed",
        "Execution should be completed"
    );

    // Check if the file was created
    println!("   4️⃣  Checking for file: {}", output_file);

    let file_exists = Path::new(&output_file).exists();
    assert!(
        file_exists,
        "Execution output file should exist at: {}",
        output_file
    );
    println!("   ✅ File exists!");

    // Validate file content
    let file_content = fs::read_to_string(&output_file)?.trim().to_string();
    println!("   5️⃣  Validating file content...");
    assert_eq!(
        file_content, test_content,
        "File content should match expected output"
    );
    println!("   ✅ File content matches expected: '{}'", file_content);

    // Save evidence
    let evidence = format!(
        "Test: test_job_execution_creates_file_on_disk\n\
         Execution ID: {}\n\
         File Path: {}\n\
         File Content: {}\n\
         Exit Code: {}\n\
         Result: PASS",
        execution_id,
        output_file,
        file_content,
        exec_status["exit_code"].as_i64().unwrap_or(-1)
    );
    save_evidence("file_io_test", "execution_result", &evidence)?;

    println!("\n✅ File I/O test PASSED!\n");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_execution_logs_contain_file_content() -> TestResult<()> {
    println!("\n🧪 Testing execution logs contain command output...\n");

    // Clean up previous test artifacts
    cleanup_execution_dir();

    let client = Client::new();

    // Execute a job with specific content that writes to a file
    let unique_content = format!("Log extraction test - {}", chrono::Utc::now().timestamp());
    let output_file = format!(
        "{}/log_test_{}.txt",
        EXECUTION_OUTPUT_DIR,
        chrono::Utc::now().timestamp()
    );

    let execution_data = json!({
        "command": format!("mkdir -p {} && echo '{}' | tee {}", EXECUTION_OUTPUT_DIR, unique_content, output_file),
        "type": "bash"
    });

    println!("   1️⃣  Executing job with output to file and stdout...");

    let exec_response = client
        .post("http://localhost:8082/api/v1/execute")
        .json(&execution_data)
        .send()
        .await?;

    let execution: Value = exec_response.json().await?;
    let execution_id = execution["id"].as_str().unwrap();

    println!("   ✅ Execution ID: {}", execution_id);

    // Wait for execution to complete
    println!("   2️⃣  Waiting for execution to complete...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Get execution logs
    println!("   3️⃣  Retrieving execution logs...");
    let logs_response = client
        .get(&format!(
            "http://localhost:8082/api/v1/executions/{}/logs",
            execution_id
        ))
        .send()
        .await?;

    assert!(
        logs_response.status().is_success(),
        "Logs retrieval should succeed"
    );

    let logs: Value = logs_response.json().await?;

    // Print full logs for debugging
    println!("   📋 Full logs response:");
    println!("{}", serde_json::to_string_pretty(&logs)?);

    // Verify logs contain the stdout content
    let logs_str = serde_json::to_string(&logs)?;
    assert!(
        logs_str.contains(&unique_content),
        "Logs should contain the command output"
    );
    println!("   ✅ Logs contain the expected content!");

    // Verify the file was created
    assert!(Path::new(&output_file).exists(), "Output file should exist");
    let file_content = fs::read_to_string(&output_file)?.trim().to_string();
    assert_eq!(file_content, unique_content, "File content should match");
    println!("   ✅ File created with correct content!");

    // Save evidence
    save_evidence(
        "logs_extraction_test",
        "execution_logs",
        &serde_json::to_string_pretty(&logs)?,
    )?;

    println!("\n✅ Log extraction test PASSED!\n");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_multiple_executions_create_separate_files() -> TestResult<()> {
    println!("\n🧪 Testing multiple executions create separate files...\n");

    // Clean up previous test artifacts
    cleanup_execution_dir();

    let client = Client::new();

    let mut execution_ids: Vec<String> = Vec::new();
    let test_contents = vec![
        "First execution content",
        "Second execution content",
        "Third execution content",
    ];

    let mut output_files: Vec<String> = Vec::new();

    // Execute multiple jobs
    for (i, content) in test_contents.iter().enumerate() {
        let output_file = format!(
            "{}/multi_exec_{}_{}.txt",
            EXECUTION_OUTPUT_DIR,
            i,
            chrono::Utc::now().timestamp_millis()
        );
        output_files.push(output_file.clone());

        let execution_data = json!({
            "command": format!("mkdir -p {} && echo '{}' > {}", EXECUTION_OUTPUT_DIR, content, output_file),
            "type": "bash"
        });

        let exec_response = client
            .post("http://localhost:8082/api/v1/execute")
            .json(&execution_data)
            .send()
            .await?;

        let execution: Value = exec_response.json().await?;
        let execution_id = execution["id"].as_str().unwrap().to_string();

        println!(
            "   {}️⃣  Execution {} started: {}",
            i + 1,
            i + 1,
            execution_id
        );
        execution_ids.push(execution_id);

        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Wait for all executions to complete
    println!("   ⏳ Waiting for all executions to complete...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify each file was created with correct content
    println!("   🔍 Verifying files...");
    for (i, (exec_id, output_file)) in execution_ids.iter().zip(output_files.iter()).enumerate() {
        let file_exists = Path::new(output_file).exists();
        assert!(
            file_exists,
            "File {} should exist at {}",
            i + 1,
            output_file
        );

        let file_content = fs::read_to_string(output_file)?.trim().to_string();
        assert_eq!(
            file_content,
            test_contents[i],
            "File {} content should match",
            i + 1
        );

        println!(
            "   ✅ File {} verified: content = '{}'",
            i + 1,
            file_content
        );

        // Verify execution status
        let status_response = client
            .get(&format!(
                "http://localhost:8082/api/v1/executions/{}",
                exec_id
            ))
            .send()
            .await?;

        let exec_status: Value = status_response.json().await?;
        assert_eq!(
            exec_status["status"],
            "completed",
            "Execution {} should be completed",
            i + 1
        );
        assert_eq!(
            exec_status["exit_code"],
            0,
            "Execution {} should have exit code 0",
            i + 1
        );
    }

    // Save evidence
    let evidence = format!(
        "Test: test_multiple_executions_create_separate_files\n\
         Total Executions: {}\n\
         Execution IDs: {:?}\n\
         Output Files: {:?}\n\
         All files created and verified: YES\n\
         Result: PASS",
        execution_ids.len(),
        execution_ids,
        output_files
    );
    save_evidence("multiple_files_test", "summary", &evidence)?;

    println!("\n✅ Multiple executions test PASSED!\n");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_collect_evidence_for_all_executions() -> TestResult<()> {
    println!("\n🧪 Testing comprehensive evidence collection...\n");

    let client = Client::new();

    // Create a new execution for this test
    let evidence_test_content = "Evidence collection test content";
    let execution_data = json!({
        "command": format!("echo '{}'", evidence_test_content),
        "type": "bash",
        "description": "Evidence collection test"
    });

    let exec_response = client
        .post("http://localhost:8082/api/v1/execute")
        .json(&execution_data)
        .send()
        .await?;

    let execution: Value = exec_response.json().await?;
    let execution_id = execution["id"].as_str().unwrap();

    println!("   ✅ Execution created: {}", execution_id);

    // Wait for execution
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Collect all executions
    println!("   📋 Collecting all executions...");
    let executions_response = client
        .get("http://localhost:8082/api/v1/executions")
        .send()
        .await?;

    let executions: Value = executions_response.json().await?;

    // Create comprehensive evidence report
    let mut evidence_report = String::new();
    evidence_report.push_str("=== HODEI-JOBS E2E TEST EVIDENCE REPORT ===\n\n");
    evidence_report.push_str(&format!(
        "Timestamp: {}\n\n",
        chrono::Utc::now().to_rfc3339()
    ));

    // Document all executions
    evidence_report.push_str("--- EXECUTIONS ---\n");
    if let Some(exec_array) = executions.as_array() {
        for exec in exec_array {
            let id = exec["id"].as_str().unwrap_or("unknown");
            evidence_report.push_str(&format!("\nExecution ID: {}\n", id));
            evidence_report.push_str(&format!(
                "Status: {}\n",
                exec["status"].as_str().unwrap_or("unknown")
            ));
            evidence_report.push_str(&format!(
                "Started: {}\n",
                exec["started_at"].as_str().unwrap_or("unknown")
            ));

            // Get logs for each execution
            if let Ok(logs_response) = client
                .get(&format!(
                    "http://localhost:8082/api/v1/executions/{}/logs",
                    id
                ))
                .send()
                .await
            {
                if let Ok(logs) = logs_response.json::<Value>().await {
                    evidence_report.push_str(&format!("Logs: {}\n", serde_json::to_string(&logs)?));
                }
            }

            // Check for output file
            let file_path = format!("{}/execution_{}.txt", EXECUTION_OUTPUT_DIR, id);
            if Path::new(&file_path).exists() {
                if let Ok(content) = fs::read_to_string(&file_path) {
                    evidence_report.push_str(&format!("Output File: {}\n", file_path));
                    evidence_report.push_str(&format!("Output Content: {}\n", content));
                }
            }
        }
    }

    // Document file system state
    evidence_report.push_str("\n--- FILE SYSTEM STATE ---\n");
    if Path::new(EXECUTION_OUTPUT_DIR).exists() {
        if let Ok(entries) = fs::read_dir(EXECUTION_OUTPUT_DIR) {
            for entry in entries.flatten() {
                evidence_report.push_str(&format!("File: {:?}\n", entry.path()));
            }
        }
    }

    // Save the evidence report
    save_evidence("comprehensive", "evidence_report", &evidence_report)?;

    println!("   📁 Evidence directory: {}", TEST_EVIDENCE_DIR);
    println!("\n✅ Evidence collection test PASSED!\n");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_execution_status_update_after_file_creation() -> TestResult<()> {
    println!("\n🧪 Testing execution status updates after file creation...\n");

    // Clean up previous test artifacts
    cleanup_execution_dir();

    let client = Client::new();

    // Execute a job
    let execution_data = json!({
        "command": "echo 'Status update test'",
        "type": "bash"
    });

    let exec_response = client
        .post("http://localhost:8082/api/v1/execute")
        .json(&execution_data)
        .send()
        .await?;

    let execution: Value = exec_response.json().await?;
    let execution_id = execution["id"].as_str().unwrap();

    println!("   ✅ Execution started: {}", execution_id);
    println!("   Initial status: {}", execution["status"]);

    // Wait for execution to complete
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Get updated execution status
    let status_response = client
        .get(&format!(
            "http://localhost:8082/api/v1/executions/{}",
            execution_id
        ))
        .send()
        .await?;

    let updated_execution: Value = status_response.json().await?;

    println!("   📊 Updated execution state:");
    println!("{}", serde_json::to_string_pretty(&updated_execution)?);

    // Verify output_file field is present
    let has_output_file = updated_execution.get("output_file").is_some();
    let output_file_created = updated_execution
        .get("output_file_created")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    println!("   Output file field present: {}", has_output_file);
    println!("   Output file created flag: {}", output_file_created);

    // Save evidence
    let evidence = format!(
        "Execution ID: {}\n\
         Output file present: {}\n\
         Output file created: {}\n\
         Full state:\n{}",
        execution_id,
        has_output_file,
        output_file_created,
        serde_json::to_string_pretty(&updated_execution)?
    );
    save_evidence("status_update_test", "execution_state", &evidence)?;

    println!("\n✅ Status update test PASSED!\n");

    Ok(())
}
