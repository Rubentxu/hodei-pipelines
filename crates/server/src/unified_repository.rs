use async_trait::async_trait;
use hodei_pipelines_adapters::{
    PostgreSqlJobRepository, PostgreSqlPipelineRepository, PostgreSqlRoleRepository,
};
use hodei_pipelines_core::{Job, JobId, JobState, Pipeline, PipelineId, Result};
use hodei_pipelines_ports::{JobRepository, PipelineRepository, RoleRepository};
use std::sync::Arc;

#[derive(Clone)]
pub struct UnifiedRepository {
    pub job_repo: Arc<PostgreSqlJobRepository>,
    pub pipeline_repo: Arc<PostgreSqlPipelineRepository>,
    pub role_repo: Arc<PostgreSqlRoleRepository>,
}

impl UnifiedRepository {
    pub fn new(
        job_repo: Arc<PostgreSqlJobRepository>,
        pipeline_repo: Arc<PostgreSqlPipelineRepository>,
        role_repo: Arc<PostgreSqlRoleRepository>,
    ) -> Self {
        Self {
            job_repo,
            pipeline_repo,
            role_repo,
        }
    }
}

#[async_trait]
impl JobRepository for UnifiedRepository {
    async fn save_job(&self, job: &Job) -> Result<()> {
        self.job_repo.save_job(job).await
    }

    async fn get_job(&self, job_id: &JobId) -> Result<Option<Job>> {
        self.job_repo.get_job(job_id).await
    }

    async fn get_pending_jobs(&self) -> Result<Vec<Job>> {
        self.job_repo.get_pending_jobs().await
    }

    async fn get_running_jobs(&self) -> Result<Vec<Job>> {
        self.job_repo.get_running_jobs().await
    }

    async fn delete_job(&self, job_id: &JobId) -> Result<()> {
        self.job_repo.delete_job(job_id).await
    }

    async fn compare_and_swap_status(
        &self,
        job_id: &JobId,
        expected_state: &str,
        new_state: &str,
    ) -> Result<bool> {
        self.job_repo
            .compare_and_swap_status(job_id, expected_state, new_state)
            .await
    }

    async fn assign_worker(
        &self,
        job_id: &JobId,
        worker_id: &hodei_pipelines_core::WorkerId,
    ) -> Result<()> {
        self.job_repo.assign_worker(job_id, worker_id).await
    }

    async fn set_job_start_time(
        &self,
        job_id: &JobId,
        start_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        self.job_repo.set_job_start_time(job_id, start_time).await
    }

    async fn set_job_finish_time(
        &self,
        job_id: &JobId,
        finish_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        self.job_repo.set_job_finish_time(job_id, finish_time).await
    }

    async fn set_job_duration(&self, job_id: &JobId, duration_ms: i64) -> Result<()> {
        self.job_repo.set_job_duration(job_id, duration_ms).await
    }

    async fn create_job(&self, job_spec: hodei_pipelines_core::job::JobSpec) -> Result<JobId> {
        self.job_repo.create_job(job_spec).await
    }

    async fn update_job_state(&self, job_id: &JobId, state: JobState) -> Result<()> {
        self.job_repo.update_job_state(job_id, state).await
    }

    async fn list_jobs(&self) -> Result<Vec<Job>> {
        self.job_repo.list_jobs().await
    }
}

#[async_trait]
impl PipelineRepository for UnifiedRepository {
    async fn save_pipeline(&self, pipeline: &Pipeline) -> Result<()> {
        self.pipeline_repo.save_pipeline(pipeline).await
    }

    async fn get_pipeline(&self, pipeline_id: &PipelineId) -> Result<Option<Pipeline>> {
        self.pipeline_repo.get_pipeline(pipeline_id).await
    }

    async fn get_all_pipelines(&self) -> Result<Vec<Pipeline>> {
        self.pipeline_repo.get_all_pipelines().await
    }

    async fn delete_pipeline(&self, pipeline_id: &PipelineId) -> Result<()> {
        self.pipeline_repo.delete_pipeline(pipeline_id).await
    }
}

#[async_trait]
impl RoleRepository for UnifiedRepository {
    async fn save_role(&self, role: &hodei_pipelines_core::security::RoleEntity) -> Result<()> {
        self.role_repo.save_role(role).await
    }

    async fn get_role(
        &self,
        id: &hodei_pipelines_core::security::RoleId,
    ) -> Result<Option<hodei_pipelines_core::security::RoleEntity>> {
        self.role_repo.get_role(id).await
    }

    async fn get_role_by_name(
        &self,
        name: &str,
    ) -> Result<Option<hodei_pipelines_core::security::RoleEntity>> {
        self.role_repo.get_role_by_name(name).await
    }

    async fn list_all_roles(&self) -> Result<Vec<hodei_pipelines_core::security::RoleEntity>> {
        self.role_repo.list_all_roles().await
    }

    async fn delete_role(&self, id: &hodei_pipelines_core::security::RoleId) -> Result<()> {
        self.role_repo.delete_role(id).await
    }

    async fn exists(&self, id: &hodei_pipelines_core::security::RoleId) -> Result<bool> {
        self.role_repo.exists(id).await
    }
}
