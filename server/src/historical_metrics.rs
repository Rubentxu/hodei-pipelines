use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::{IntoParams, ToSchema};

use crate::AppState;

/// Historical metric data structure
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HistoricalMetric {
    pub timestamp: DateTime<Utc>,
    pub metric_name: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub unit: String,
    pub tags: HashMap<String, String>,
}

/// Type of historical metric
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    Resource,
    Job,
    Worker,
    Queue,
    Cost,
    Scaling,
}

/// Time range for historical queries
#[derive(Debug, Clone, Serialize, Deserialize, IntoParams)]
pub struct TimeRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub interval: Option<AggregationInterval>,
}

/// Aggregation interval for time series data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AggregationInterval {
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

/// Historical metrics query response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HistoricalMetricsResponse {
    pub metrics: Vec<HistoricalMetric>,
    pub total_count: usize,
    pub time_range: TimeRange,
    pub aggregation_interval: AggregationInterval,
}

/// Metric statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricStatistics {
    pub min_value: f64,
    pub max_value: f64,
    pub avg_value: f64,
    pub median_value: f64,
    pub std_dev: f64,
    pub percentile_95: f64,
    pub percentile_99: f64,
    pub count: usize,
}

/// Metric trends and patterns
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricTrend {
    pub trend_direction: TrendDirection,
    pub growth_rate: f64,
    pub seasonality_detected: bool,
    pub anomaly_count: usize,
    pub forecast: Option<MetricForecast>,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
    Volatile,
}

/// Metric forecast
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricForecast {
    pub predicted_value: f64,
    pub confidence_interval: (f64, f64),
    pub forecast_date: DateTime<Utc>,
}

/// Historical metrics comparison
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricsComparison {
    pub period_a: TimeRange,
    pub period_b: TimeRange,
    pub metrics_a: Vec<MetricStatistics>,
    pub metrics_b: Vec<MetricStatistics>,
    pub comparison: HashMap<String, f64>,
}

/// Service for historical metrics management
#[derive(Debug)]
pub struct HistoricalMetricsService {
    /// Historical metrics data
    metrics: Arc<RwLock<Vec<HistoricalMetric>>>,
}

impl HistoricalMetricsService {
    /// Create new historical metrics service
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add historical metric
    pub async fn add_metric(&self, metric: HistoricalMetric) {
        let mut metrics = self.metrics.write().await;
        metrics.push(metric);
    }

    /// Query historical metrics
    pub async fn query_metrics(
        &self,
        metric_name: Option<String>,
        metric_type: Option<MetricType>,
        time_range: TimeRange,
        aggregation_interval: AggregationInterval,
    ) -> Vec<HistoricalMetric> {
        let metrics = self.metrics.read().await;
        let start = time_range
            .start
            .unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));
        let end = time_range.end.unwrap_or(Utc::now());

        let filtered: Vec<_> = metrics
            .iter()
            .filter(|m| {
                m.timestamp >= start
                    && m.timestamp <= end
                    && metric_name.as_ref().map_or(true, |n| &m.metric_name == n)
                    && metric_type.as_ref().map_or(true, |t| m.metric_type == *t)
            })
            .cloned()
            .collect();

        // Aggregate by interval
        self.aggregate_metrics(filtered, aggregation_interval)
    }

    /// Calculate metric statistics
    pub async fn calculate_statistics(
        &self,
        metric_name: &str,
        time_range: TimeRange,
    ) -> Option<MetricStatistics> {
        let metrics = self.metrics.read().await;
        let start = time_range
            .start
            .unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));
        let end = time_range.end.unwrap_or(Utc::now());

        let values: Vec<f64> = metrics
            .iter()
            .filter(|m| m.timestamp >= start && m.timestamp <= end && m.metric_name == metric_name)
            .map(|m| m.value)
            .collect();

        if values.is_empty() {
            return None;
        }

        let count = values.len();
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let sum: f64 = values.iter().sum();
        let avg = sum / count as f64;
        let variance: f64 = values.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        let median = if count % 2 == 0 {
            (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0
        } else {
            sorted[count / 2]
        };

        let percentile_95 = sorted[(count as f64 * 0.95) as usize].min(sorted[count - 1]);
        let percentile_99 = sorted[(count as f64 * 0.99) as usize].min(sorted[count - 1]);

        Some(MetricStatistics {
            min_value: sorted[0],
            max_value: sorted[count - 1],
            avg_value: avg,
            median_value: median,
            std_dev,
            percentile_95,
            percentile_99,
            count,
        })
    }

    /// Analyze metric trends
    pub async fn analyze_trends(
        &self,
        metric_name: &str,
        time_range: TimeRange,
    ) -> Option<MetricTrend> {
        let metrics = self.metrics.read().await;
        let start = time_range
            .start
            .unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
        let end = time_range.end.unwrap_or(Utc::now());

        let values: Vec<(DateTime<Utc>, f64)> = metrics
            .iter()
            .filter(|m| m.timestamp >= start && m.timestamp <= end && m.metric_name == metric_name)
            .map(|m| (m.timestamp, m.value))
            .collect();

        if values.len() < 2 {
            return None;
        }

        // Calculate trend
        let first_value = values[0].1;
        let last_value = values[values.len() - 1].1;
        let growth_rate = if first_value != 0.0 {
            ((last_value - first_value) / first_value) * 100.0
        } else {
            0.0
        };

        let trend_direction = if growth_rate > 5.0 {
            TrendDirection::Increasing
        } else if growth_rate < -5.0 {
            TrendDirection::Decreasing
        } else if growth_rate.abs() < 1.0 {
            TrendDirection::Stable
        } else {
            TrendDirection::Volatile
        };

        // Simple anomaly detection (values beyond 2 standard deviations)
        let values_only: Vec<f64> = values.iter().map(|(_, v)| *v).collect();
        let avg: f64 = values_only.iter().sum::<f64>() / values_only.len() as f64;
        let variance: f64 =
            values_only.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / values_only.len() as f64;
        let std_dev = variance.sqrt();

        let anomaly_count = values_only
            .iter()
            .filter(|v| (*v - avg).abs() > 2.0 * std_dev)
            .count();

        // Simple seasonality detection
        let seasonality_detected = self.detect_seasonality(&values_only);

        // Simple forecast
        let forecast = Some(MetricForecast {
            predicted_value: last_value + (growth_rate / 100.0 * last_value),
            confidence_interval: (last_value - std_dev, last_value + std_dev),
            forecast_date: end + chrono::Duration::days(7),
        });

        Some(MetricTrend {
            trend_direction,
            growth_rate,
            seasonality_detected,
            anomaly_count,
            forecast,
        })
    }

    /// Compare metrics between two time periods
    pub async fn compare_periods(
        &self,
        metric_name: &str,
        period_a: TimeRange,
        period_b: TimeRange,
    ) -> Option<MetricsComparison> {
        let stats_a = self
            .calculate_statistics(metric_name, period_a.clone())
            .await?;
        let stats_b = self
            .calculate_statistics(metric_name, period_b.clone())
            .await?;

        let mut comparison = HashMap::new();
        comparison.insert(
            "avg_change_percent".to_string(),
            ((stats_b.avg_value - stats_a.avg_value) / stats_a.avg_value) * 100.0,
        );
        comparison.insert(
            "max_change_percent".to_string(),
            ((stats_b.max_value - stats_a.max_value) / stats_a.max_value) * 100.0,
        );
        comparison.insert(
            "count_change_percent".to_string(),
            ((stats_b.count as f64 - stats_a.count as f64) / stats_a.count as f64) * 100.0,
        );

        Some(MetricsComparison {
            period_a,
            period_b,
            metrics_a: vec![stats_a],
            metrics_b: vec![stats_b],
            comparison,
        })
    }

    /// Aggregate metrics by interval
    fn aggregate_metrics(
        &self,
        metrics: Vec<HistoricalMetric>,
        interval: AggregationInterval,
    ) -> Vec<HistoricalMetric> {
        if metrics.is_empty() {
            return vec![];
        }

        let mut grouped: HashMap<String, Vec<f64>> = HashMap::new();

        for metric in metrics {
            let bucket = self.get_time_bucket(metric.timestamp, &interval);
            grouped
                .entry(bucket)
                .or_insert_with(Vec::new)
                .push(metric.value);
        }

        let mut aggregated = Vec::new();
        let now = Utc::now();

        for (bucket, values) in grouped {
            let avg_value: f64 = values.iter().sum::<f64>() / values.len() as f64;
            aggregated.push(HistoricalMetric {
                timestamp: now, // Use current time for aggregated data
                metric_name: "aggregated".to_string(),
                metric_type: MetricType::Resource,
                value: avg_value,
                unit: "aggregated".to_string(),
                tags: HashMap::from([("bucket".to_string(), bucket)]),
            });
        }

        aggregated
    }

    /// Get time bucket string for aggregation
    fn get_time_bucket(&self, timestamp: DateTime<Utc>, interval: &AggregationInterval) -> String {
        match interval {
            AggregationInterval::Minute => timestamp.format("%Y-%m-%d %H:%M").to_string(),
            AggregationInterval::Hour => timestamp.format("%Y-%m-%d %H:00").to_string(),
            AggregationInterval::Day => timestamp.format("%Y-%m-%d").to_string(),
            AggregationInterval::Week => {
                // Simplified: just use week number approximation
                let days_since_epoch = timestamp.timestamp() / 86400;
                let week_num = (days_since_epoch / 7) % 52;
                format!("{}-W{}", timestamp.format("%Y"), week_num)
            }
            AggregationInterval::Month => timestamp.format("%Y-%m").to_string(),
            AggregationInterval::Quarter => {
                // Simplified quarter calculation
                let quarter = ((timestamp.timestamp() / 2592000) % 4) + 1;
                format!("{}-Q{}", timestamp.format("%Y"), quarter)
            }
            AggregationInterval::Year => timestamp.format("%Y").to_string(),
        }
    }

    /// Simple seasonality detection
    fn detect_seasonality(&self, values: &[f64]) -> bool {
        if values.len() < 24 {
            return false;
        }

        // Simple check: if there are repeating patterns every ~24 hours
        // For a real implementation, use Fourier transform or autocorrelation
        let window = 24;
        if values.len() < window * 2 {
            return false;
        }

        let mut pattern_matches = 0;
        for i in 0..window.min(values.len() / 2) {
            let val1 = values[i];
            let val2 = values[i + window];
            if (val1 - val2).abs() / val1 < 0.1 {
                pattern_matches += 1;
            }
        }

        pattern_matches > window / 4
    }
}

/// Get historical metrics for a specific metric
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/metrics/historical/{metric_name}",
    params(
        ("metric_name" = String, Path, description = "Name of the metric"),
        TimeRange
    ),
    responses(
        (status = 200, description = "Historical metrics retrieved successfully", body = HistoricalMetricsResponse),
        (status = 400, description = "Invalid time range"),
        (status = 404, description = "Metric not found")
    ),
    tag = "Historical Metrics"
)]
pub async fn get_historical_metrics(
    Path(metric_name): Path<String>,
    time_range: axum::extract::Query<TimeRange>,
    State(state): State<HistoricalMetricsAppState>,
) -> Result<Json<HistoricalMetricsResponse>, StatusCode> {
    let time_range_owned = time_range.0;
    let time_range_for_response = time_range_owned.clone();
    let aggregation_interval = time_range_owned
        .interval
        .clone()
        .unwrap_or(AggregationInterval::Hour);

    let metrics = state
        .service
        .query_metrics(
            Some(metric_name),
            None,
            time_range_owned,
            aggregation_interval.clone(),
        )
        .await;

    let metrics_count = metrics.len();
    Ok(Json(HistoricalMetricsResponse {
        metrics,
        total_count: metrics_count,
        time_range: time_range_for_response,
        aggregation_interval,
    }))
}

/// Get metric statistics
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/metrics/historical/{metric_name}/statistics",
    params(
        ("metric_name" = String, Path, description = "Name of the metric"),
        TimeRange
    ),
    responses(
        (status = 200, description = "Metric statistics retrieved successfully", body = MetricStatistics),
        (status = 404, description = "Metric not found")
    ),
    tag = "Historical Metrics"
)]
pub async fn get_metric_statistics(
    Path(metric_name): Path<String>,
    time_range: axum::extract::Query<TimeRange>,
    State(state): State<HistoricalMetricsAppState>,
) -> Result<Json<MetricStatistics>, StatusCode> {
    let statistics = state
        .service
        .calculate_statistics(&metric_name, time_range.0)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(statistics))
}

/// Get metric trends
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/metrics/historical/{metric_name}/trends",
    params(
        ("metric_name" = String, Path, description = "Name of the metric"),
        TimeRange
    ),
    responses(
        (status = 200, description = "Metric trends retrieved successfully", body = MetricTrend),
        (status = 404, description = "Metric not found")
    ),
    tag = "Historical Metrics"
)]
pub async fn get_metric_trends(
    Path(metric_name): Path<String>,
    time_range: axum::extract::Query<TimeRange>,
    State(state): State<HistoricalMetricsAppState>,
) -> Result<Json<MetricTrend>, StatusCode> {
    let trend = state
        .service
        .analyze_trends(&metric_name, time_range.0)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(trend))
}

/// Compare metrics between periods
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/metrics/historical/{metric_name}/compare",
    params(
        ("metric_name" = String, Path, description = "Name of the metric"),
    ),
    responses(
        (status = 200, description = "Metrics comparison retrieved successfully", body = MetricsComparison),
        (status = 404, description = "Metric not found")
    ),
    tag = "Historical Metrics"
)]
pub async fn compare_metrics(
    Path(metric_name): Path<String>,
    State(state): State<HistoricalMetricsAppState>,
) -> Result<Json<MetricsComparison>, StatusCode> {
    let now = Utc::now();
    let period_a = TimeRange {
        start: Some(now - chrono::Duration::days(30)),
        end: Some(now - chrono::Duration::days(15)),
        interval: None,
    };
    let period_b = TimeRange {
        start: Some(now - chrono::Duration::days(14)),
        end: Some(now),
        interval: None,
    };

    let comparison = state
        .service
        .compare_periods(&metric_name, period_a, period_b)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(comparison))
}

/// Application state for Historical Metrics
#[derive(Clone)]
pub struct HistoricalMetricsAppState {
    pub service: Arc<HistoricalMetricsService>,
}

/// Historical metrics routes
pub fn historical_metrics_routes() -> Router<HistoricalMetricsAppState> {
    Router::new()
        .route(
            "/metrics/historical/:metric_name",
            get(get_historical_metrics),
        )
        .route(
            "/metrics/historical/:metric_name/statistics",
            get(get_metric_statistics),
        )
        .route(
            "/metrics/historical/:metric_name/trends",
            get(get_metric_trends),
        )
        .route(
            "/metrics/historical/:metric_name/compare",
            get(compare_metrics),
        )
}
