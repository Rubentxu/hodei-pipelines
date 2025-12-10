//! Application Service for Job Management

use domain::job_execution::{Job, JobRepository, JobSpec};

pub struct JobService {
    job_repo: Box<dyn JobRepository>,
}

impl JobService {
    pub fn new(job_repo: Box<dyn JobRepository>) -> Self {
        Self { job_repo }
    }
}
