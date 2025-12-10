//! In-Memory Repositories
//!
//! Test implementations for repositories using in-memory storage

use domain::job_execution::Job;
use domain::provider_management::Provider;
use domain::shared_kernel::{DomainResult, JobId, ProviderId};

pub struct InMemoryJobRepository {
    jobs: std::sync::Mutex<std::collections::HashMap<String, Job>>,
}

impl InMemoryJobRepository {
    pub fn new() -> Self {
        Self {
            jobs: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl domain::job_execution::JobRepository for InMemoryJobRepository {
    async fn save(&self, job: &Job) -> DomainResult<()> {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.insert(job.id.to_string(), job.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &JobId) -> DomainResult<Option<Job>> {
        let jobs = self.jobs.lock().unwrap();
        Ok(jobs.get(&id.to_string()).cloned())
    }

    async fn list(&self) -> DomainResult<Vec<Job>> {
        let jobs = self.jobs.lock().unwrap();
        Ok(jobs.values().cloned().collect())
    }

    async fn delete(&self, id: &JobId) -> DomainResult<()> {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.remove(&id.to_string());
        Ok(())
    }
}

pub struct InMemoryProviderRepository {
    providers: std::sync::Mutex<std::collections::HashMap<String, Provider>>,
}

impl InMemoryProviderRepository {
    pub fn new() -> Self {
        Self {
            providers: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl domain::provider_management::ProviderRepository for InMemoryProviderRepository {
    async fn save(&self, provider: &Provider) -> DomainResult<()> {
        let mut providers = self.providers.lock().unwrap();
        providers.insert(provider.id.to_string(), provider.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &ProviderId) -> DomainResult<Option<Provider>> {
        let providers = self.providers.lock().unwrap();
        Ok(providers.get(&id.to_string()).cloned())
    }

    async fn list(&self) -> DomainResult<Vec<Provider>> {
        let providers = self.providers.lock().unwrap();
        Ok(providers.values().cloned().collect())
    }

    async fn delete(&self, id: &ProviderId) -> DomainResult<()> {
        let mut providers = self.providers.lock().unwrap();
        providers.remove(&id.to_string());
        Ok(())
    }
}
