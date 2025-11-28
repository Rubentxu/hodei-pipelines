use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::{IntoParams, ToSchema};

use crate::AppState;

/// Grafana dashboard
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GrafanaDashboard {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub dashboard_type: DashboardType,
    pub panels: Vec<Panel>,
    pub tags: Vec<String>,
    pub refresh_interval: String,
    pub time_range: TimeRange,
    pub variables: HashMap<String, String>,
}

/// Type of dashboard
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DashboardType {
    System,
    Jobs,
    Workers,
    Costs,
    Performance,
    Custom,
}

/// Panel in a dashboard
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Panel {
    pub id: String,
    pub title: String,
    pub panel_type: PanelType,
    pub query: String,
    pub visualization: VisualizationType,
    pub position: PanelPosition,
    pub size: PanelSize,
    pub thresholds: Option<Vec<f64>>,
}

/// Type of panel
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PanelType {
    Query,
    Text,
    Table,
    Graph,
    Heatmap,
    Gauge,
}

/// Visualization type
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VisualizationType {
    Line,
    Bar,
    Pie,
    Stat,
    Gauge,
    Table,
    Heatmap,
}

impl std::fmt::Display for VisualizationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisualizationType::Line => write!(f, "timeseries"),
            VisualizationType::Bar => write!(f, "barchart"),
            VisualizationType::Pie => write!(f, "piechart"),
            VisualizationType::Stat => write!(f, "stat"),
            VisualizationType::Gauge => write!(f, "gauge"),
            VisualizationType::Table => write!(f, "table"),
            VisualizationType::Heatmap => write!(f, "heatmap"),
        }
    }
}

/// Panel position
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PanelPosition {
    pub x: u32,
    pub y: u32,
}

/// Panel size
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PanelSize {
    pub width: u32,
    pub height: u32,
}

/// Time range for dashboards
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeRange {
    pub from: String,
    pub to: String,
}

/// Grafana datasource
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GrafanaDatasource {
    pub id: String,
    pub name: String,
    pub datasource_type: DatasourceType,
    pub url: String,
    pub access: AccessMode,
    pub is_default: bool,
}

/// Type of datasource
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DatasourceType {
    Prometheus,
    InfluxDB,
    Elasticsearch,
    Graphite,
    CloudWatch,
}

/// Access mode
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessMode {
    Proxy,
    Direct,
}

/// Grafana configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GrafanaConfig {
    pub enabled: bool,
    pub url: String,
    pub api_key: Option<String>,
    pub organization: String,
    pub datasource_config: GrafanaDatasource,
}

/// Dashboard template
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardTemplate {
    pub name: String,
    pub description: String,
    pub dashboard_type: DashboardType,
    pub default_panels: Vec<Panel>,
    pub default_variables: HashMap<String, String>,
}

/// Service for Grafana dashboards
#[derive(Debug)]
pub struct GrafanaDashboardsService {
    /// Grafana configuration
    config: Arc<RwLock<GrafanaConfig>>,
    /// Registered dashboards
    dashboards: Arc<RwLock<HashMap<String, GrafanaDashboard>>>,
    /// Dashboard templates
    templates: Arc<RwLock<HashMap<String, DashboardTemplate>>>,
}

impl GrafanaDashboardsService {
    /// Create new Grafana dashboards service
    pub fn new() -> Self {
        let default_config = GrafanaConfig {
            enabled: true,
            url: "http://localhost:3000".to_string(),
            api_key: None,
            organization: "default".to_string(),
            datasource_config: GrafanaDatasource {
                id: "prometheus-datasource".to_string(),
                name: "Prometheus".to_string(),
                datasource_type: DatasourceType::Prometheus,
                url: "http://localhost:9090".to_string(),
                access: AccessMode::Proxy,
                is_default: true,
            },
        };

        let mut templates = HashMap::new();

        // System Overview Template
        templates.insert(
            "system-overview".to_string(),
            DashboardTemplate {
                name: "System Overview".to_string(),
                description: "Overview of system metrics and health".to_string(),
                dashboard_type: DashboardType::System,
                default_panels: vec![Panel {
                    id: "panel-1".to_string(),
                    title: "CPU Usage".to_string(),
                    panel_type: PanelType::Query,
                    query: "rate(cpu_usage_total[5m])".to_string(),
                    visualization: VisualizationType::Gauge,
                    position: PanelPosition { x: 0, y: 0 },
                    size: PanelSize {
                        width: 6,
                        height: 6,
                    },
                    thresholds: Some(vec![70.0, 90.0]),
                }],
                default_variables: HashMap::new(),
            },
        );

        Self {
            config: Arc::new(RwLock::new(default_config)),
            dashboards: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(templates)),
        }
    }

    /// Create dashboard from template
    pub async fn create_dashboard_from_template(
        &self,
        template_name: &str,
        title: &str,
        variables: Option<HashMap<String, String>>,
    ) -> Result<GrafanaDashboard, String> {
        let templates = self.templates.read().await;
        let template = templates
            .get(template_name)
            .ok_or_else(|| "Template not found".to_string())?;

        let panels = template.default_panels.clone();
        let dashboard_vars = variables.unwrap_or_else(|| template.default_variables.clone());

        let dashboard = GrafanaDashboard {
            id: format!("dash-{}", rand::random::<u64>()),
            title: title.to_string(),
            description: Some(template.description.clone()),
            dashboard_type: template.dashboard_type.clone(),
            panels,
            tags: vec!["template".to_string(), template_name.to_string()],
            refresh_interval: "30s".to_string(),
            time_range: TimeRange {
                from: "now-1h".to_string(),
                to: "now".to_string(),
            },
            variables: dashboard_vars,
        };

        Ok(dashboard)
    }

    /// Create dashboard
    pub async fn create_dashboard(&self, dashboard: GrafanaDashboard) -> Result<(), String> {
        let mut dashboards = self.dashboards.write().await;
        dashboards.insert(dashboard.id.clone(), dashboard);
        Ok(())
    }

    /// Get dashboard by ID
    pub async fn get_dashboard(&self, id: &str) -> Option<GrafanaDashboard> {
        let dashboards = self.dashboards.read().await;
        dashboards.get(id).cloned()
    }

    /// List all dashboards
    pub async fn list_dashboards(&self) -> Vec<GrafanaDashboard> {
        let dashboards = self.dashboards.read().await;
        dashboards.values().cloned().collect()
    }

    /// Update dashboard
    pub async fn update_dashboard(&self, dashboard: GrafanaDashboard) -> Result<(), String> {
        let mut dashboards = self.dashboards.write().await;
        dashboards.insert(dashboard.id.clone(), dashboard);
        Ok(())
    }

    /// Delete dashboard
    pub async fn delete_dashboard(&self, id: &str) -> Result<(), String> {
        let mut dashboards = self.dashboards.write().await;
        dashboards.remove(id);
        Ok(())
    }

    /// Get configuration
    pub async fn get_config(&self) -> GrafanaConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Update configuration
    pub async fn update_config(&self, config: GrafanaConfig) {
        let mut config_lock = self.config.write().await;
        *config_lock = config;
    }

    /// List dashboard templates
    pub async fn list_templates(&self) -> Vec<DashboardTemplate> {
        let templates = self.templates.read().await;
        templates.values().cloned().collect()
    }

    /// Get dashboard JSON for Grafana import
    pub async fn export_dashboard_json(&self, id: &str) -> Result<serde_json::Value, String> {
        let dashboard = self
            .get_dashboard(id)
            .await
            .ok_or_else(|| "Dashboard not found".to_string())?;

        // Mock Grafana dashboard JSON format
        let json = serde_json::json!({
            "dashboard": {
                "id": null,
                "title": dashboard.title,
                "tags": dashboard.tags,
                "timezone": "browser",
                "refresh": dashboard.refresh_interval,
                "time": {
                    "from": dashboard.time_range.from,
                    "to": dashboard.time_range.to
                },
                "panels": dashboard.panels.iter().map(|panel| {
                    serde_json::json!({
                        "id": panel.id,
                        "title": panel.title,
                        "type": format!("{}", panel.visualization),
                        "targets": [
                            {
                                "expr": panel.query
                            }
                        ],
                        "gridPos": {
                            "x": panel.position.x,
                            "y": panel.position.y,
                            "w": panel.size.width,
                            "h": panel.size.height
                        }
                    })
                }).collect::<Vec<_>>(),
                "variables": dashboard.variables.iter().map(|(k, v)| {
                    serde_json::json!({
                        "name": k,
                        "value": v
                    })
                }).collect::<Vec<_>>()
            },
            "overwrite": true,
            "inputs": [],
            "folderId": null,
            "message": "Updated by Hodei Pipelines"
        });

        Ok(json)
    }
}

/// Get dashboard by ID
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/grafana/dashboards/{id}",
    params(
        ("id" = String, Path, description = "Dashboard ID")
    ),
    responses(
        (status = 200, description = "Dashboard retrieved successfully", body = GrafanaDashboard),
        (status = 404, description = "Dashboard not found")
    ),
    tag = "Grafana Dashboards"
)]
pub async fn get_dashboard(
    Path(id): Path<String>,
    State(state): State<GrafanaDashboardsAppState>,
) -> Result<Json<GrafanaDashboard>, StatusCode> {
    let dashboard = state
        .service
        .get_dashboard(&id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(dashboard))
}

/// List all dashboards
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/grafana/dashboards",
    responses(
        (status = 200, description = "Dashboards retrieved successfully", body = Vec<GrafanaDashboard>)
    ),
    tag = "Grafana Dashboards"
)]
pub async fn list_dashboards(
    State(state): State<GrafanaDashboardsAppState>,
) -> Result<Json<Vec<GrafanaDashboard>>, StatusCode> {
    let dashboards = state.service.list_dashboards().await;
    Ok(Json(dashboards))
}

/// Create dashboard from template
#[allow(dead_code)] //#[utoipa::path(
    post,
    path = "/api/v1/grafana/dashboards/from-template/{template_name}",
    params(
        ("template_name" = String, Path, description = "Template name")
    ),
    request_body = CreateDashboardRequest,
    responses(
        (status = 201, description = "Dashboard created successfully", body = GrafanaDashboard),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Template not found")
    ),
    tag = "Grafana Dashboards"
)]
pub async fn create_dashboard_from_template(
    Path(template_name): Path<String>,
    State(state): State<GrafanaDashboardsAppState>,
    Json(request): Json<CreateDashboardRequest>,
) -> Result<Json<GrafanaDashboard>, StatusCode> {
    let dashboard = state
        .service
        .create_dashboard_from_template(&template_name, &request.title, request.variables)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(dashboard))
}

/// Create dashboard request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDashboardRequest {
    pub title: String,
    pub variables: Option<HashMap<String, String>>,
}

/// Create dashboard
#[allow(dead_code)] //#[utoipa::path(
    post,
    path = "/api/v1/grafana/dashboards",
    request_body = GrafanaDashboard,
    responses(
        (status = 201, description = "Dashboard created successfully"),
        (status = 400, description = "Invalid dashboard")
    ),
    tag = "Grafana Dashboards"
)]
pub async fn create_dashboard(
    State(state): State<GrafanaDashboardsAppState>,
    Json(dashboard): Json<GrafanaDashboard>,
) -> Result<Json<String>, StatusCode> {
    state
        .service
        .create_dashboard(dashboard)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json("Dashboard created successfully".to_string()))
}

/// Update dashboard
#[allow(dead_code)] //#[utoipa::path(
    put,
    path = "/api/v1/grafana/dashboards/{id}",
    params(
        ("id" = String, Path, description = "Dashboard ID")
    ),
    request_body = GrafanaDashboard,
    responses(
        (status = 200, description = "Dashboard updated successfully"),
        (status = 404, description = "Dashboard not found")
    ),
    tag = "Grafana Dashboards"
)]
pub async fn update_dashboard(
    Path(id): Path<String>,
    State(state): State<GrafanaDashboardsAppState>,
    Json(dashboard): Json<GrafanaDashboard>,
) -> Result<Json<String>, StatusCode> {
    if dashboard.id != id {
        return Err(StatusCode::BAD_REQUEST);
    }

    state
        .service
        .update_dashboard(dashboard)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json("Dashboard updated successfully".to_string()))
}

/// Delete dashboard
#[allow(dead_code)] //#[utoipa::path(
    delete,
    path = "/api/v1/grafana/dashboards/{id}",
    params(
        ("id" = String, Path, description = "Dashboard ID")
    ),
    responses(
        (status = 200, description = "Dashboard deleted successfully"),
        (status = 404, description = "Dashboard not found")
    ),
    tag = "Grafana Dashboards"
)]
pub async fn delete_dashboard(
    Path(id): Path<String>,
    State(state): State<GrafanaDashboardsAppState>,
) -> Result<Json<String>, StatusCode> {
    state
        .service
        .delete_dashboard(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json("Dashboard deleted successfully".to_string()))
}

/// Get Grafana configuration
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/grafana/config",
    responses(
        (status = 200, description = "Configuration retrieved successfully", body = GrafanaConfig)
    ),
    tag = "Grafana Dashboards"
)]
pub async fn get_grafana_config(
    State(state): State<GrafanaDashboardsAppState>,
) -> Result<Json<GrafanaConfig>, StatusCode> {
    let config = state.service.get_config().await;
    Ok(Json(config))
}

/// Update Grafana configuration
#[allow(dead_code)] //#[utoipa::path(
    put,
    path = "/api/v1/grafana/config",
    request_body = GrafanaConfig,
    responses(
        (status = 200, description = "Configuration updated successfully"),
        (status = 400, description = "Invalid configuration")
    ),
    tag = "Grafana Dashboards"
)]
pub async fn update_grafana_config(
    State(state): State<GrafanaDashboardsAppState>,
    Json(config): Json<GrafanaConfig>,
) -> Result<Json<String>, StatusCode> {
    state.service.update_config(config).await;
    Ok(Json("Configuration updated successfully".to_string()))
}

/// List dashboard templates
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/grafana/templates",
    responses(
        (status = 200, description = "Templates retrieved successfully", body = Vec<DashboardTemplate>)
    ),
    tag = "Grafana Dashboards"
)]
pub async fn list_templates(
    State(state): State<GrafanaDashboardsAppState>,
) -> Result<Json<Vec<DashboardTemplate>>, StatusCode> {
    let templates = state.service.list_templates().await;
    Ok(Json(templates))
}

/// Export dashboard JSON for Grafana import
#[allow(dead_code)] //#[utoipa::path(
    get,
    path = "/api/v1/grafana/dashboards/{id}/export",
    params(
        ("id" = String, Path, description = "Dashboard ID")
    ),
    responses(
        (status = 200, description = "Dashboard exported successfully"),
        (status = 404, description = "Dashboard not found")
    ),
    tag = "Grafana Dashboards"
)]
pub async fn export_dashboard(
    Path(id): Path<String>,
    State(state): State<GrafanaDashboardsAppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let json = state
        .service
        .export_dashboard_json(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(json))
}

/// Application state for Grafana Dashboards
#[derive(Clone)]
pub struct GrafanaDashboardsAppState {
    pub service: Arc<GrafanaDashboardsService>,
}

/// Grafana dashboards routes
pub fn grafana_dashboards_routes() -> Router<GrafanaDashboardsAppState> {
    Router::new()
        .route("/grafana/config", get(get_grafana_config))
        .route("/grafana/config", put(update_grafana_config))
        .route("/grafana/templates", get(list_templates))
        .route("/grafana/dashboards", get(list_dashboards))
        .route("/grafana/dashboards", post(create_dashboard))
        .route("/grafana/dashboards/{id}", get(get_dashboard))
        .route("/grafana/dashboards/{id}", put(update_dashboard))
        .route("/grafana/dashboards/{id}", delete(delete_dashboard))
        .route("/grafana/dashboards/{id}/export", get(export_dashboard))
        .route(
            "/grafana/dashboards/from-template/{template_name}",
            post(create_dashboard_from_template),
        )
}
