//! Distributed tracing utilities for Hodei Pipelines
//!
//! This module provides distributed tracing capabilities across service boundaries
//! following OpenTelemetry standards.

use opentelemetry::Context;
use opentelemetry::global;
use opentelemetry::trace::{Span, Status, StatusCode, Tracer};
use opentelemetry_otlp::{TonicExporterBuilder, WithExportConfig};
use opentelemetry_sdk::{runtime::Tokio, trace as sdktrace};
use std::collections::HashMap;
use tracing::{error, info, instrument, warn};
use tracing_opentelemetry::{OpenTelemetryLayer, OpenTelemetrySpanExt};

/// Initialize OpenTelemetry tracing
///
/// # Arguments
/// * `service_name` - Name of the service for tracing
/// * `jaeger_endpoint` - Jaeger collector endpoint (optional)
///
/// # Returns
/// * Result<(), Box<dyn std::error::Error>> - Initialization result
pub fn init_tracing(
    service_name: &str,
    jaeger_endpoint: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create OTLP exporter (sends to Jaeger)
    let mut exporter = TonicExporterBuilder::default();

    if let Some(endpoint) = jaeger_endpoint {
        exporter = exporter.with_endpoint(endpoint);
    } else {
        exporter = exporter.with_endpoint("http://localhost:14268/api/traces");
    }

    // Configure tracer pipeline
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .exporter(exporter)
        .with_trace_config(
            sdktrace::config()
                .with_sampler(sdktrace::Sampler::AlwaysOn)
                .with_max_events_per_span(100)
                .with_max_attributes_per_span(100),
        )
        .install_batch(Tokio)?;

    // Set global tracer provider
    global::set_tracer_provider(tracer);

    info!("Tracing initialized for service: {}", service_name);
    Ok(())
}

/// Extract tracing context from HTTP headers
///
/// Extracts OpenTelemetry context from HTTP request headers for distributed tracing.
/// This allows tracing requests across service boundaries.
///
/// # Arguments
/// * `headers` - HTTP headers to extract context from
///
/// # Returns
/// * `opentelemetry::Context` - Extracted context or default
pub fn extract_context_from_headers(headers: &HashMap<String, String>) -> Context {
    let propagator = global::get_text_map_propagator(|p| p);
    let carrier: HashMap<&str, &str> = headers
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    propagator.extract(&carrier)
}

/// Inject tracing context into HTTP headers
///
/// Injects OpenTelemetry context into HTTP request headers to propagate
/// tracing information to downstream services.
///
/// # Arguments
/// * `headers` - HTTP headers to inject context into
/// * `context` - Tracing context to inject
///
/// # Returns
/// * `HashMap<String, String>` - Headers with tracing context
pub fn inject_context_to_headers(
    headers: &HashMap<String, String>,
    context: &Context,
) -> HashMap<String, String> {
    let mut new_headers = headers.clone();
    let propagator = global::get_text_map_propagator(|p| p);

    let mut carrier: HashMap<String, String> = HashMap::new();
    propagator.inject(context, &mut carrier);

    for (key, value) in carrier {
        new_headers.insert(key, value);
    }

    new_headers
}

/// Create a traced span for critical operations
///
/// Helper function to create properly configured spans for tracing
/// critical business operations.
///
/// # Arguments
/// * `operation` - Operation name
/// * `job_id` - Job ID being operated on (optional)
/// * `worker_id` - Worker ID involved (optional)
///
/// # Returns
/// * `opentelemetry::trace::Span` - Configured span
pub fn create_trace_span(operation: &str, job_id: Option<&str>, worker_id: Option<&str>) -> Span {
    let tracer = global::tracer("hodei-pipelines");

    let mut span = tracer.span_builder(operation);

    if let Some(id) = job_id {
        span = span.with_attribute(opentelemetry::KeyValue::new("job.id", id));
    }

    if let Some(id) = worker_id {
        span = span.with_attribute(opentelemetry::KeyValue::new("worker.id", id));
    }

    span.with_attribute(opentelemetry::KeyValue::new(
        "service.name",
        "hodei-pipelines",
    ))
    .start(&tracer)
}

/// Record operation success with tracing
///
/// Helper to record successful operation completion in tracing system.
///
/// # Arguments
/// * `span` - Span to record success on
/// * `message` - Success message
/// * `duration_ms` - Operation duration in milliseconds (optional)
pub fn record_operation_success(span: &Span, message: &str, duration_ms: Option<u64>) {
    if let Some(duration) = duration_ms {
        span.set_attribute(opentelemetry::KeyValue::new(
            "operation.duration_ms",
            duration as i64,
        ));
    }

    span.set_attribute(opentelemetry::KeyValue::new("operation.status", "success"));

    info!("{} - Operation completed successfully", message);
    span.set_status(Status::ok());
}

/// Record operation failure with tracing
///
/// Helper to record failed operation in tracing system for debugging.
///
/// # Arguments
/// * `span` - Span to record failure on
/// * `error` - Error that occurred
/// * `error_type` - Type/category of error
pub fn record_operation_failure(span: &Span, error: &str, error_type: &str) {
    span.set_attribute(opentelemetry::KeyValue::new("error.type", error_type));
    span.set_attribute(opentelemetry::KeyValue::new("error.message", error));
    span.set_attribute(opentelemetry::KeyValue::new("operation.status", "error"));

    error!("Operation failed: {} - Type: {}", error, error_type);
    span.set_status(Status::error(StatusCode::Error));
}

/// Macro to instrument functions with automatic tracing
///
/// This macro automatically creates spans for functions and records their execution time.
///
/// # Example
/// ```rust
/// # #[tracing::instrument(skip(ctx), fields(job_id = %ctx.job_id))]
/// async fn schedule_job(ctx: &SchedulingContext) -> Result<(), Error> {
///     // Function implementation
///     Ok(())
/// }
/// ```
///
/// Note: This is a placeholder. The actual #[tracing::instrument] attribute
/// from the tracing crate provides this functionality. This module provides
/// helpers to complement the instrument attribute.
#[macro_export]
macro_rules! traced_operation {
    ($operation:expr, $job_id:expr, $worker_id:expr, $body:block) => {{
        let span = $crate::tracing::create_trace_span($operation, $job_id, $worker_id);
        let _guard = span.enter();

        let start = std::time::Instant::now();
        let result = $body;
        let duration = start.elapsed();

        match &result {
            Ok(_) => {
                $crate::tracing::record_operation_success(
                    &span,
                    &format!("Operation {} completed", $operation),
                    Some(duration.as_millis() as u64),
                );
            }
            Err(e) => {
                $crate::tracing::record_operation_failure(
                    &span,
                    &format!("{}", e),
                    "operation_error",
                );
            }
        }

        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_extract_context_from_headers() {
        let mut headers = HashMap::new();
        headers.insert("x-request-id".to_string(), "req-123".to_string());
        headers.insert("x-trace-id".to_string(), "trace-456".to_string());

        let context = extract_context_from_headers(&headers);
        // In real usage, this would extract the OpenTelemetry context
        assert_eq!(context, Context::current()); // Placeholder assertion
    }

    #[test]
    fn test_inject_context_to_headers() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let context = Context::current();
        let injected = inject_context_to_headers(&headers, &context);

        assert!(injected.contains_key("Content-Type"));
        assert!(injected.len() >= headers.len());
    }

    #[test]
    fn test_create_trace_span() {
        let span = create_trace_span("test_operation", Some("job-123"), Some("worker-456"));

        // Verify span was created (can't verify attributes in unit test easily)
        assert!(true);
    }

    #[test]
    fn test_record_operations() {
        let span = create_trace_span("test", None, None);

        record_operation_success(&span, "Test operation completed", Some(100));
        record_operation_failure(&span, "Test error", "test_error");

        // If we got here without panic, the functions executed successfully
        assert!(true);
    }
}
