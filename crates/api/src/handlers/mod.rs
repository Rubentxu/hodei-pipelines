//! HTTP Handlers
//!
//! Request handlers for the API endpoints

use axum::{
    extract::{State, Path},
    http::StatusCode,
    response::{Json, IntoResponse},
    routing::{post, get, delete, put},
};
use serde::{Deserialize, Serialize};
use application::{JobService, ProviderService, EventOrchestrator};
use domain::job_execution::JobSpec;
use domain::provider_management::entities::ProviderStatus;
use domain::shared_kernel::{JobId, ProviderId, ProviderType, ProviderCapabilities};
use std::sync::Arc;
use tokio::sync::Mutex;

// ==================== REQUEST/RESPONSE DTOs ====================

#[derive(Deserialize, Debug)]
pub struct CreateJobRequest {
    pub name: String,
    pub command: Vec<String>,
    pub args: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct CreateJobResponse {
    pub job_id: String,
    pub status: String,
    pub name: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct JobResponse {
    pub job_id: String,
    pub name: String,
    pub command: Vec<String>,
    pub args: Vec<String>,
    pub status: String,
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct JobListResponse {
    pub jobs: Vec<JobResponse>,
    pub total: usize,
}

#[derive(Deserialize, Debug)]
pub struct RegisterProviderRequest {
    pub name: String,
    pub provider_type: String,
    pub max_concurrent_jobs: u32,
    pub supported_job_types: Vec<String>,
    pub memory_limit_mb: Option<u64>,
    pub cpu_limit: Option<f64>,
    pub config_endpoint: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct RegisterProviderResponse {
    pub provider_id: String,
    pub name: String,
    pub provider_type: String,
    pub status: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ProviderResponse {
    pub provider_id: String,
    pub name: String,
    pub provider_type: String,
    pub status: String,
    pub max_concurrent_jobs: u32,
    pub supported_job_types: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ProviderListResponse {
    pub providers: Vec<ProviderResponse>,
    pub total: usize,
}

#[derive(Serialize, Debug, Clone)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

// ==================== APPLICATION STATE ====================

#[derive(Clone)]
pub struct AppState {
    pub job_service: Arc<Mutex<JobService>>,
    pub provider_service: Arc<Mutex<ProviderService>>,
    pub event_orchestrator: Arc<Mutex<EventOrchestrator>>,
}

// ==================== JOB HANDLERS ====================

pub async fn create_job_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateJobRequest>,
) -> Result<Json<ApiResponse<CreateJobResponse>>, StatusCode> {
    // Validate input
    if request.name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let job_spec = JobSpec::new(request.name, request.command, request.args);

    let job_service = state.job_service.lock().await;
    let job = job_service.create_job(job_spec)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let event_orchestrator = state.event_orchestrator.lock().await;
    let _ = event_orchestrator.publish_job_created(
        job.id.clone(),
        job.spec.name.clone(),
        None,
    ).await;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(CreateJobResponse {
            job_id: job.id.to_string(),
            status: format!("{:?}", job.state),
            name: job.spec.name,
        }),
        error: None,
    }))
}

pub async fn list_jobs_handler(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<JobListResponse>>, StatusCode> {
    let job_service = state.job_service.lock().await;
    let jobs = job_service.list_jobs()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let job_responses: Vec<JobResponse> = jobs.into_iter().map(|job| JobResponse {
        job_id: job.id.to_string(),
        name: job.spec.name,
        command: job.spec.command,
        args: job.spec.args,
        status: format!("{:?}", job.state),
        created_at: None,
        completed_at: None,
    }).collect();

    let total = job_responses.len();

    Ok(Json(ApiResponse {
        success: true,
        data: Some(JobListResponse {
            jobs: job_responses,
            total,
        }),
        error: None,
    }))
}

pub async fn get_job_handler(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<ApiResponse<JobResponse>>, StatusCode> {
    let job_id = JobId::new(job_id);
    let job_service = state.job_service.lock().await;
    let job = job_service.get_job(&job_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(JobResponse {
            job_id: job.id.to_string(),
            name: job.spec.name,
            command: job.spec.command,
            args: job.spec.args,
            status: format!("{:?}", job.state),
            created_at: None,
            completed_at: None,
        }),
        error: None,
    }))
}

pub async fn execute_job_handler(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<ApiResponse<JobResponse>>, StatusCode> {
    let job_id = JobId::new(job_id);
    let mut job_service = state.job_service.lock().await;
    let job = job_service.execute_job(&job_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(JobResponse {
            job_id: job.id.to_string(),
            name: job.spec.name,
            command: job.spec.command,
            args: job.spec.args,
            status: format!("{:?}", job.state),
            created_at: None,
            completed_at: None,
        }),
        error: None,
    }))
}

pub async fn cancel_job_handler(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let job_id = JobId::new(job_id);
    let mut job_service = state.job_service.lock().await;
    job_service.cancel_job(&job_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    }))
}

// ==================== PROVIDER HANDLERS ====================

pub async fn register_provider_handler(
    State(state): State<AppState>,
    Json(request): Json<RegisterProviderRequest>,
) -> Result<Json<ApiResponse<RegisterProviderResponse>>, StatusCode> {
    let provider_id = ProviderId::new(format!("provider-{}", uuid::Uuid::new_v4()));
    let capabilities = ProviderCapabilities {
        max_concurrent_jobs: request.max_concurrent_jobs,
        supported_job_types: request.supported_job_types,
        memory_limit_mb: request.memory_limit_mb,
        cpu_limit: request.cpu_limit,
    };

    let provider_type = match request.provider_type.to_lowercase().as_str() {
        "docker" => ProviderType::Docker,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let config = domain::provider_management::value_objects::ProviderConfig::new(
        request.config_endpoint
    );

    let provider_service = state.provider_service.lock().await;
    let provider = provider_service.register_provider(
        provider_id,
        request.name,
        provider_type,
        capabilities,
        config,
    ).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(RegisterProviderResponse {
            provider_id: provider.id.to_string(),
            name: provider.name,
            provider_type: format!("{:?}", provider.provider_type),
            status: format!("{:?}", provider.status),
        }),
        error: None,
    }))
}

pub async fn list_providers_handler(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ProviderListResponse>>, StatusCode> {
    let provider_service = state.provider_service.lock().await;
    let providers = provider_service.list_providers()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let provider_responses: Vec<ProviderResponse> = providers.into_iter().map(|provider| ProviderResponse {
        provider_id: provider.id.to_string(),
        name: provider.name,
        provider_type: format!("{:?}", provider.provider_type),
        status: format!("{:?}", provider.status),
        max_concurrent_jobs: provider.capabilities.max_concurrent_jobs,
        supported_job_types: provider.capabilities.supported_job_types,
        created_at: None,
    }).collect();

    let total = provider_responses.len();

    Ok(Json(ApiResponse {
        success: true,
        data: Some(ProviderListResponse {
            providers: provider_responses,
            total,
        }),
        error: None,
    }))
}

pub async fn get_provider_handler(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Result<Json<ApiResponse<ProviderResponse>>, StatusCode> {
    let provider_id = ProviderId::new(provider_id);
    let provider_service = state.provider_service.lock().await;
    let provider = provider_service.get_provider(&provider_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(ProviderResponse {
            provider_id: provider.id.to_string(),
            name: provider.name,
            provider_type: format!("{:?}", provider.provider_type),
            status: format!("{:?}", provider.status),
            max_concurrent_jobs: provider.capabilities.max_concurrent_jobs,
            supported_job_types: provider.capabilities.supported_job_types,
            created_at: None,
        }),
        error: None,
    }))
}

pub async fn activate_provider_handler(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let provider_id = ProviderId::new(provider_id);
    let mut provider_service = state.provider_service.lock().await;
    provider_service.activate_provider(&provider_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    }))
}

pub async fn deactivate_provider_handler(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let provider_id = ProviderId::new(provider_id);
    let mut provider_service = state.provider_service.lock().await;
    provider_service.deactivate_provider(&provider_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    }))
}

pub async fn delete_provider_handler(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let provider_id = ProviderId::new(provider_id);
    let provider_service = state.provider_service.lock().await;
    provider_service.delete_provider(&provider_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    }))
}

// ==================== HEALTH CHECK ====================

pub async fn health_check_handler() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use super::*;
    use application::{JobService, ProviderService, EventOrchestrator};
    use domain::job_execution::JobRepository;
    use domain::provider_management::ProviderRepository;
    use domain::shared_kernel::EventPublisher;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock implementations
    struct MockJobRepository {
        jobs: Arc<Mutex<Vec<domain::job_execution::Job>>>,
    }

    impl MockJobRepository {
        fn new() -> Self {
            Self {
                jobs: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl JobRepository for MockJobRepository {
        async fn save(&self, job: &domain::job_execution::Job) -> domain::shared_kernel::DomainResult<()> {
            let mut jobs = self.jobs.lock().await;
            if let Some(index) = jobs.iter().position(|j| j.id == job.id) {
                jobs[index] = job.clone();
            } else {
                jobs.push(job.clone());
            }
            Ok(())
        }

        async fn find_by_id(&self, id: &domain::shared_kernel::JobId) -> domain::shared_kernel::DomainResult<Option<domain::job_execution::Job>> {
            let jobs = self.jobs.lock().await;
            Ok(jobs.iter().find(|j| j.id == *id).cloned())
        }

        async fn list(&self) -> domain::shared_kernel::DomainResult<Vec<domain::job_execution::Job>> {
            let jobs = self.jobs.lock().await;
            Ok(jobs.clone())
        }

        async fn delete(&self, id: &domain::shared_kernel::JobId) -> domain::shared_kernel::DomainResult<()> {
            let mut jobs = self.jobs.lock().await;
            jobs.retain(|j| j.id != *id);
            Ok(())
        }
    }

    struct MockProviderRepository {
        providers: Arc<Mutex<Vec<domain::provider_management::Provider>>>,
    }

    impl MockProviderRepository {
        fn new() -> Self {
            Self {
                providers: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl ProviderRepository for MockProviderRepository {
        async fn save(&self, provider: &domain::provider_management::Provider) -> domain::shared_kernel::DomainResult<()> {
            let mut providers = self.providers.lock().await;
            if let Some(index) = providers.iter().position(|p| p.id == provider.id) {
                providers[index] = provider.clone();
            } else {
                providers.push(provider.clone());
            }
            Ok(())
        }

        async fn find_by_id(&self, id: &domain::shared_kernel::ProviderId) -> domain::shared_kernel::DomainResult<Option<domain::provider_management::Provider>> {
            let providers = self.providers.lock().await;
            Ok(providers.iter().find(|p| p.id == *id).cloned())
        }

        async fn list(&self) -> domain::shared_kernel::DomainResult<Vec<domain::provider_management::Provider>> {
            let providers = self.providers.lock().await;
            Ok(providers.clone())
        }

        async fn delete(&self, id: &domain::shared_kernel::ProviderId) -> domain::shared_kernel::DomainResult<()> {
            let mut providers = self.providers.lock().await;
            providers.retain(|p| p.id != *id);
            Ok(())
        }
    }

    struct MockEventPublisher {
        events: Arc<Mutex<Vec<domain::shared_kernel::DomainEvent>>>,
    }

    impl MockEventPublisher {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl EventPublisher for MockEventPublisher {
        async fn publish(&self, event: &domain::shared_kernel::DomainEvent) -> Result<(), domain::shared_kernel::EventError> {
            let mut events = self.events.lock().await;
            events.push(event.clone());
            Ok(())
        }
    }

    // ==================== TDD RED PHASE ====================

    #[tokio::test]
    async fn test_create_job_handler_success() {
        let job_repo = Box::new(MockJobRepository::new()) as Box<dyn JobRepository>;
        let event_publisher = Box::new(MockEventPublisher::new()) as Box<dyn EventPublisher>;

        let state = AppState {
            job_service: Arc::new(Mutex::new(JobService::new(job_repo))),
            provider_service: Arc::new(Mutex::new(ProviderService::new(Box::new(MockProviderRepository::new())))),
            event_orchestrator: Arc::new(Mutex::new(EventOrchestrator::new(event_publisher))),
        };

        let request = CreateJobRequest {
            name: "test-job".to_string(),
            command: vec!["echo".to_string()],
            args: vec!["hello".to_string()],
        };

        let response = create_job_handler(State(state), Json(request))
            .await
            .unwrap();

        assert!(response.success);
        assert!(response.data.is_some());
        assert_eq!(response.data.as_ref().unwrap().name, "test-job");
    }

    #[tokio::test]
    async fn test_create_job_handler_empty_name() {
        let job_repo = Box::new(MockJobRepository::new()) as Box<dyn JobRepository>;
        let event_publisher = Box::new(MockEventPublisher::new()) as Box<dyn EventPublisher>;

        let state = AppState {
            job_service: Arc::new(Mutex::new(JobService::new(job_repo))),
            provider_service: Arc::new(Mutex::new(ProviderService::new(Box::new(MockProviderRepository::new())))),
            event_orchestrator: Arc::new(Mutex::new(EventOrchestrator::new(event_publisher))),
        };

        let request = CreateJobRequest {
            name: "".to_string(),
            command: vec!["echo".to_string()],
            args: vec![],
        };

        let result = create_job_handler(State(state), Json(request)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_jobs_handler() {
        let job_repo = Box::new(MockJobRepository::new()) as Box<dyn JobRepository>;
        let event_publisher = Box::new(MockEventPublisher::new()) as Box<dyn EventPublisher>;

        let state = AppState {
            job_service: Arc::new(Mutex::new(JobService::new(job_repo))),
            provider_service: Arc::new(Mutex::new(ProviderService::new(Box::new(MockProviderRepository::new())))),
            event_orchestrator: Arc::new(Mutex::new(EventOrchestrator::new(event_publisher))),
        };

        let response = list_jobs_handler(State(state))
            .await
            .unwrap();

        assert!(response.success);
        assert!(response.data.is_some());
        assert_eq!(response.data.as_ref().unwrap().total, 0);
    }

    #[tokio::test]
    async fn test_register_provider_handler_success() {
        let provider_repo = Box::new(MockProviderRepository::new()) as Box<dyn ProviderRepository>;
        let event_publisher = Box::new(MockEventPublisher::new()) as Box<dyn EventPublisher>;

        let state = AppState {
            job_service: Arc::new(Mutex::new(JobService::new(Box::new(MockJobRepository::new())))),
            provider_service: Arc::new(Mutex::new(ProviderService::new(provider_repo))),
            event_orchestrator: Arc::new(Mutex::new(EventOrchestrator::new(event_publisher))),
        };

        let request = RegisterProviderRequest {
            name: "Docker Provider".to_string(),
            provider_type: "docker".to_string(),
            max_concurrent_jobs: 10,
            supported_job_types: vec!["bash".to_string()],
            memory_limit_mb: Some(2048),
            cpu_limit: Some(2.0),
            config_endpoint: "http://localhost:2375".to_string(),
        };

        let response = register_provider_handler(State(state), Json(request))
            .await
            .unwrap();

        assert!(response.success);
        assert!(response.data.is_some());
        assert_eq!(response.data.as_ref().unwrap().name, "Docker Provider");
    }

    #[tokio::test]
    async fn test_get_job_not_found() {
        let job_repo = Box::new(MockJobRepository::new()) as Box<dyn JobRepository>;
        let event_publisher = Box::new(MockEventPublisher::new()) as Box<dyn EventPublisher>;

        let state = AppState {
            job_service: Arc::new(Mutex::new(JobService::new(job_repo))),
            provider_service: Arc::new(Mutex::new(ProviderService::new(Box::new(MockProviderRepository::new())))),
            event_orchestrator: Arc::new(Mutex::new(EventOrchestrator::new(event_publisher))),
        };

        let result = get_job_handler(State(state), Path("nonexistent".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::NOT_FOUND);
    }
}
