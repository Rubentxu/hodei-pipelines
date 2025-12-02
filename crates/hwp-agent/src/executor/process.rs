//! Process management module
//!
//! This module handles spawning jobs as child processes, managing their lifecycle,
//! and capturing exit codes and signals.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::pty::{PtyAllocation, PtyError};

/// Job identifier
pub type JobId = String;

/// Process information
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub job_id: JobId,
    pub pid: u32,
    pub command: Vec<String>,
    pub working_dir: Option<String>,
    pub env_vars: HashMap<String, String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub status: ProcessStatus,
}

/// Process status
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessStatus {
    Pending,
    Starting,
    Running,
    Completed { exit_code: i32 },
    Failed { error: String },
    Cancelled,
}

/// Job handle for controlling a running job
#[derive(Debug, Clone)]
pub struct JobHandle {
    pub job_id: JobId,
    pub child: Arc<Mutex<Option<Child>>>,
    pub master_pty: Arc<()>,
}

impl JobHandle {
    pub async fn kill(&self) -> Result<(), ProcessError> {
        let mut child_lock = self.child.lock().await;
        if let Some(child) = child_lock.as_mut() {
            warn!(
                "Killing job {} (PID: {})",
                self.job_id,
                child.id().unwrap_or(0)
            );
            child
                .kill()
                .await
                .map_err(|e| ProcessError::KillFailed(e.to_string()))?;
            *child_lock = None;
            info!("Job {} killed", self.job_id);
        }
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        let child_lock = self.child.lock().await;
        child_lock.as_ref().and_then(|child| child.id()).is_some()
    }

    pub async fn pid(&self) -> Option<u32> {
        let child_lock = self.child.lock().await;
        child_lock.as_ref().and_then(|child| child.id())
    }
}

#[derive(Debug)]
pub struct ProcessManager {
    jobs: Arc<RwLock<HashMap<JobId, JobHandle>>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn spawn_job(
        &self,
        command: Vec<String>,
        env_vars: HashMap<String, String>,
        working_dir: Option<String>,
        _pty: &PtyAllocation,
    ) -> Result<
        (
            JobId,
            Option<tokio::process::ChildStdout>,
            Option<tokio::process::ChildStderr>,
        ),
        ProcessError,
    > {
        let job_id = Uuid::new_v4().to_string();
        info!("Spawning job {}: {:?}", job_id, command);

        let mut child = tokio::process::Command::new(&command[0]);
        if command.len() > 1 {
            child.args(&command[1..]);
        }
        if let Some(dir) = &working_dir {
            child.current_dir(dir);
        }
        for (key, value) in env_vars {
            child.env(key, value);
        }

        // Capture stdout and stderr
        child.stdout(std::process::Stdio::piped());
        child.stderr(std::process::Stdio::piped());

        let mut child = match child.spawn() {
            Ok(child) => {
                info!("Job {} spawned with PID: {:?}", job_id, child.id());
                child
            }
            Err(e) => {
                error!("Failed to spawn job {}: {}", job_id, e);
                return Err(ProcessError::SpawnFailed(e.to_string()));
            }
        };

        // Take streams out of child
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let handle = JobHandle {
            job_id: job_id.clone(),
            child: Arc::new(Mutex::new(Some(child))),
            master_pty: Arc::new(()),
        };

        let mut jobs = self.jobs.write().await;
        jobs.insert(job_id.clone(), handle);

        Ok((job_id, stdout, stderr))
    }

    pub async fn get_job(&self, job_id: &JobId) -> Option<ProcessInfo> {
        let jobs = self.jobs.read().await;
        if let Some(handle) = jobs.get(job_id) {
            let pid = handle.pid().await.unwrap_or(0);
            Some(ProcessInfo {
                job_id: job_id.clone(),
                pid,
                command: vec!["unknown".to_string()],
                working_dir: None,
                env_vars: HashMap::new(),
                started_at: chrono::Utc::now(),
                status: ProcessStatus::Running,
            })
        } else {
            None
        }
    }

    pub async fn cancel_job(&self, job_id: &JobId) -> Result<(), ProcessError> {
        let mut jobs = self.jobs.write().await;
        if let Some(handle) = jobs.remove(job_id) {
            warn!("Cancelling job {}", job_id);
            handle.kill().await?;
            info!("Job {} cancelled", job_id);
            Ok(())
        } else {
            Err(ProcessError::NotFound(job_id.clone()))
        }
    }

    pub async fn list_jobs(&self) -> Vec<JobId> {
        let jobs = self.jobs.read().await;
        jobs.keys().cloned().collect()
    }

    pub async fn wait_for_job(&self, job_id: &JobId) -> Result<i32, ProcessError> {
        let handle = {
            let jobs = self.jobs.read().await;
            jobs.get(job_id).cloned()
        };

        if let Some(handle) = handle {
            let mut child_lock = handle.child.lock().await;
            if let Some(child) = child_lock.as_mut() {
                let status = child.wait().await.map_err(|e| ProcessError::Io(e))?;

                Ok(status.code().unwrap_or(-1))
            } else {
                Err(ProcessError::NotFound(job_id.clone()))
            }
        } else {
            Err(ProcessError::NotFound(job_id.clone()))
        }
    }

    pub async fn cleanup_finished(&self) {
        let mut jobs = self.jobs.write().await;
        let mut finished = Vec::new();

        for (id, handle) in jobs.iter() {
            if !handle.is_running().await {
                finished.push(id.clone());
            }
        }

        for job_id in finished {
            warn!("Cleaning up finished job {}", job_id);
            jobs.remove(&job_id);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Spawn failed: {0}")]
    SpawnFailed(String),

    #[error("Kill failed: {0}")]
    KillFailed(String),

    #[error("Job not found: {0}")]
    NotFound(JobId),

    #[error("PTY error: {0}")]
    Pty(#[from] PtyError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct JobExecutor {
    process_manager: ProcessManager,
}

impl JobExecutor {
    pub fn new() -> Self {
        Self {
            process_manager: ProcessManager::new(),
        }
    }

    pub async fn execute_job(
        &self,
        command: Vec<String>,
        env_vars: HashMap<String, String>,
        working_dir: Option<String>,
        pty: &PtyAllocation,
    ) -> Result<
        (
            JobId,
            Option<tokio::process::ChildStdout>,
            Option<tokio::process::ChildStderr>,
        ),
        ProcessError,
    > {
        self.process_manager
            .spawn_job(command, env_vars, working_dir, pty)
            .await
    }

    pub fn process_manager(&self) -> &ProcessManager {
        &self.process_manager
    }
}

impl Default for JobExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_manager_creation() {
        let manager = ProcessManager::new();
        let jobs = manager.list_jobs().await;
        assert!(jobs.is_empty());
    }
}
