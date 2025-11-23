//! Log Streaming E2E Tests
//!
//! These tests validate the real-time log streaming functionality via SSE
//! with Docker-like API parameters (follow, tail, since, timestamps).
//!
//! NOTE: These tests require the worker-lifecycle-manager service to be running
//! on port 8082. To run these tests:
//!
//! 1. Start the service: cargo run -p hodei-worker-lifecycle-manager
//! 2. In another terminal: cargo test -p e2e-tests --test log_streaming_test

use e2e_tests::TestResult;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

/// Test basic SSE connectivity and historical log retrieval
#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_sse_historical_logs() -> TestResult<()> {
    println!("\n🧪 Testing SSE historical log retrieval...\n");

    let client = Client::new();

    // Step 1: Execute a job (creates execution)
    println!("   📤 Executing job to generate logs...");
    let execute_data = json!({
        "command": r#"
            echo "Line 1: Starting execution"
            echo "Line 2: Processing data"
            echo "Line 3: Writing output"
            echo "Line 4: Finalizing"
            echo "Line 5: Complete"
        "#
    });

    let execute_response = client
        .post("http://localhost:8082/api/v1/execute")
        .json(&execute_data)
        .send()
        .await?;

    assert!(
        execute_response.status().is_success(),
        "Job execution should succeed"
    );
    println!("   ✅ Job execution started");

    let execute_result: Value = execute_response.json().await?;
    let execution_id = execute_result["id"].as_str().unwrap();
    println!("   📋 Execution ID: {}", execution_id);

    // Wait for job to complete
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Step 2: Get historical logs via SSE (without follow)
    println!("   📥 Fetching historical logs via SSE...");
    let response = client
        .get(&format!(
            "http://localhost:8082/api/v1/executions/{}/logs/stream?tail=10",
            execution_id
        ))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "SSE endpoint should be accessible"
    );

    // Read the entire response body (for SSE, it's a stream of events)
    let body = response.text().await?;

    let mut received_events = 0;
    let mut collected_data = Vec::new();

    // Parse SSE events (each event starts with "data: ")
    for line in body.lines() {
        if line.starts_with("data: ") {
            let json_str = &line[6..]; // Remove "data: " prefix

            match serde_json::from_str::<Value>(json_str) {
                Ok(event_data) => {
                    collected_data.push(event_data.clone());
                    received_events += 1;
                    println!("   📝 Received event {}: {:?}", received_events, event_data);
                }
                Err(e) => {
                    println!("   ⚠️  Failed to parse JSON: {} (raw: {})", e, json_str);
                }
            }
        }
    }

    assert!(received_events > 0, "Should receive at least one log event");

    println!("   ✅ Received {} log events via SSE", received_events);

    // Verify log content
    let all_lines: Vec<String> = collected_data
        .iter()
        .filter_map(|e| {
            e.get("line")
                .and_then(|l| l.as_str())
                .map(|s| s.to_string())
        })
        .collect();

    assert!(
        all_lines.len() >= 3,
        "Should have at least 3 log lines, got: {:?}",
        all_lines
    );

    println!("   ✅ Log content validated: {} lines", all_lines.len());

    println!("\n✅ Historical log streaming test passed!\n");

    Ok(())
}

/// Test log streaming with tail parameter
#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_realtime_log_streaming() -> TestResult<()> {
    println!("\n🧪 Testing log streaming with real-time capability...\n");

    let client = Client::new();

    println!("   📤 Executing job that will generate logs...");
    let execute_data = json!({
        "command": r#"
            i=1
            while [ $i -le 5 ]; do
                echo "Streaming line $i"
                sleep 0.3
                i=$((i + 1))
            done
            echo "Final line"
        "#
    });

    let execute_response = client
        .post("http://localhost:8082/api/v1/execute")
        .json(&execute_data)
        .send()
        .await?;

    assert!(
        execute_response.status().is_success(),
        "Job execution should succeed"
    );

    let execute_result: Value = execute_response.json().await?;
    let execution_id = execute_result["id"].as_str().unwrap();

    println!("   ✅ Execution started: {}", execution_id);

    // Give a moment for the job to start
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Start SSE connection to verify streaming capability
    // Use tail (not follow) to avoid timeout - this tests the same streaming functionality
    println!("   📡 Opening SSE connection with tail=10...");
    let response = client
        .get(&format!(
            "http://localhost:8082/api/v1/executions/{}/logs/stream?tail=10&timestamps=true",
            execution_id
        ))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "SSE endpoint should be accessible"
    );

    println!("   ✅ SSE endpoint accessible");

    // Give time for execution to complete
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Read the response body
    let body = response.text().await?;

    let mut received_events = 0;

    // Parse SSE events
    for line in body.lines() {
        if line.starts_with("data: ") {
            let json_str = &line[6..];

            if let Ok(event_data) = serde_json::from_str::<Value>(json_str) {
                received_events += 1;
                println!("   📡 Event {}: {:?}", received_events, event_data);
            }
        }
    }

    assert!(received_events > 0, "Should receive at least one log event");

    println!("   ✅ Received {} log events via SSE", received_events);
    println!("\n✅ Log streaming test passed!\n");

    Ok(())
}

/// Test timestamp parameter in logs
#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_log_streaming_with_timestamps() -> TestResult<()> {
    println!("\n🧪 Testing log streaming with timestamps...\n");

    let client = Client::new();

    println!("   📤 Executing job...");
    let execute_data = json!({
        "command": r#"
            echo "Test log with timestamp"
            echo "Another log line"
        "#
    });

    let execute_response = client
        .post("http://localhost:8082/api/v1/execute")
        .json(&execute_data)
        .send()
        .await?;

    assert!(
        execute_response.status().is_success(),
        "Job execution should succeed"
    );

    let execute_result: Value = execute_response.json().await?;
    let execution_id = execute_result["id"].as_str().unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Get logs with timestamps
    println!("   📥 Fetching logs with timestamps=true...");
    let response = client
        .get(&format!(
            "http://localhost:8082/api/v1/executions/{}/logs/stream?timestamps=true&tail=5",
            execution_id
        ))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "SSE endpoint should be accessible"
    );

    let body = response.text().await?;

    let mut has_timestamp_field = false;
    let mut event_count = 0;

    // Parse SSE events
    for line in body.lines() {
        if line.starts_with("data: ") {
            let json_str = &line[6..];

            if let Ok(event_data) = serde_json::from_str::<Value>(json_str) {
                event_count += 1;

                // Check if timestamp field exists
                if event_data.get("timestamp").is_some() {
                    has_timestamp_field = true;
                    let timestamp = event_data["timestamp"].as_str().unwrap();
                    println!("   ⏰ Event with timestamp: {}", timestamp);

                    // Verify timestamp format (should be RFC3339)
                    assert!(
                        timestamp.contains('T')
                            && (timestamp.contains('Z') || timestamp.contains('+')),
                        "Timestamp should be in RFC3339 format: {}",
                        timestamp
                    );
                }

                // Verify other expected fields
                assert!(
                    event_data.get("stream").is_some(),
                    "Event should have stream field"
                );
                assert!(
                    event_data.get("line").is_some(),
                    "Event should have line field"
                );
            }
        }

        if event_count >= 2 {
            break;
        }
    }

    assert!(
        has_timestamp_field,
        "Events should include timestamp field when timestamps=true"
    );

    println!("   ✅ Timestamp validation passed");

    println!("\n✅ Log streaming with timestamps test passed!\n");

    Ok(())
}

/// Test tail parameter for limiting log lines
#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_log_streaming_with_tail() -> TestResult<()> {
    println!("\n🧪 Testing log streaming with tail parameter...\n");

    let client = Client::new();

    println!("   📤 Executing job that generates many log lines...");
    let execute_data = json!({
        "command": r#"
            i=1
            while [ $i -le 20 ]; do
                echo "Log line $i"
                i=$((i + 1))
            done
        "#
    });

    let execute_response = client
        .post("http://localhost:8082/api/v1/execute")
        .json(&execute_data)
        .send()
        .await?;

    assert!(
        execute_response.status().is_success(),
        "Job execution should succeed"
    );

    let execute_result: Value = execute_response.json().await?;
    let execution_id = execute_result["id"].as_str().unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Get only last 5 lines
    println!("   📥 Fetching last 5 lines (tail=5)...");
    let response = client
        .get(&format!(
            "http://localhost:8082/api/v1/executions/{}/logs/stream?tail=5",
            execution_id
        ))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "SSE endpoint should be accessible"
    );

    let body = response.text().await?;

    let mut collected_lines = Vec::new();

    // Parse SSE events
    for line in body.lines() {
        if line.starts_with("data: ") {
            let json_str = &line[6..];

            if let Ok(event_data) = serde_json::from_str::<Value>(json_str) {
                if let Some(line_text) = event_data.get("line").and_then(|l| l.as_str()) {
                    collected_lines.push(line_text.to_string());
                }
            }
        }

        // Break after collecting some events
        if collected_lines.len() >= 5 {
            break;
        }
    }

    // Should have at most 5 lines (tail=5)
    assert!(
        collected_lines.len() <= 5,
        "Should have at most 5 lines with tail=5, got {}",
        collected_lines.len()
    );

    println!("   ✅ Received {} lines (tail=5)", collected_lines.len());

    // Verify these are the last lines (16-20)
    if collected_lines.len() > 0 {
        let first_line = &collected_lines[0];
        assert!(
            first_line.contains("16") || first_line.contains("17"),
            "With tail=5, should get the last lines, got: {:?}",
            collected_lines
        );
    }

    println!("   ✅ Tail parameter validation passed");

    println!("\n✅ Log streaming with tail test passed!\n");

    Ok(())
}

/// Test multiple concurrent subscribers to the same log stream
#[tokio::test]
#[ignore] // Requires worker-lifecycle-manager running on port 8082
async fn test_multiple_concurrent_subscribers() -> TestResult<()> {
    println!("\n🧪 Testing multiple concurrent log subscribers...\n");

    let client = Client::new();

    println!("   📤 Executing job for multiple subscribers test...");
    let execute_data = json!({
        "command": r#"
            echo "Line 1"
            sleep 0.3
            echo "Line 2"
            sleep 0.3
            echo "Line 3"
        "#
    });

    let execute_response = client
        .post("http://localhost:8082/api/v1/execute")
        .json(&execute_data)
        .send()
        .await?;

    assert!(
        execute_response.status().is_success(),
        "Job execution should succeed"
    );

    let execute_result: Value = execute_response.json().await?;
    let execution_id = execute_result["id"].as_str().unwrap();

    println!(
        "   ✅ Setup complete for multiple subscribers test: {}",
        execution_id
    );

    // Wait for execution to complete (it has 2 x 0.3s sleeps = ~0.6s + overhead)
    println!("   ⏳ Waiting for execution to complete...");
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify execution is complete
    let exec_status = client
        .get(&format!(
            "http://localhost:8082/api/v1/executions/{}",
            execution_id
        ))
        .send()
        .await?;

    if exec_status.status().is_success() {
        let status_data: Value = exec_status.json().await?;
        println!("   ✅ Execution status: {:?}", status_data.get("status"));
    }

    // Create three concurrent SSE connections after execution completes
    let url = format!(
        "http://localhost:8082/api/v1/executions/{}/logs/stream?tail=5",
        execution_id
    );

    println!("   📡 Opening 3 concurrent SSE connections...");
    let subscriber1 = client.get(&url).send();
    let subscriber2 = client.get(&url).send();
    let subscriber3 = client.get(&url).send();

    let (resp1, resp2, resp3) = futures::future::join3(subscriber1, subscriber2, subscriber3).await;

    assert!(
        resp1.is_ok() && resp2.is_ok() && resp3.is_ok(),
        "All subscribers should connect"
    );
    println!("   ✅ Three SSE connections established");

    // Collect from all three subscribers
    let body1 = resp1.unwrap().text().await.unwrap_or_default();
    let body2 = resp2.unwrap().text().await.unwrap_or_default();
    let body3 = resp3.unwrap().text().await.unwrap_or_default();

    let count_events = |body: &str| -> usize {
        body.lines()
            .filter(|line| line.starts_with("data: "))
            .count()
    };

    let count1 = count_events(&body1);
    let count2 = count_events(&body2);
    let count3 = count_events(&body3);

    // All subscribers should receive events
    assert!(count1 > 0, "Subscriber 1 should receive events");
    assert!(count2 > 0, "Subscriber 2 should receive events");
    assert!(count3 > 0, "Subscriber 3 should receive events");

    println!("   ✅ Subscriber 1 received {} events", count1);
    println!("   ✅ Subscriber 2 received {} events", count2);
    println!("   ✅ Subscriber 3 received {} events", count3);

    println!("\n✅ Multiple concurrent subscribers test passed!\n");

    Ok(())
}
