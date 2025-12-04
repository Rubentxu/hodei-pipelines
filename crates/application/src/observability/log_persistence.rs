//! Log Persistence Service
//!
//! Subscribes to LogChunkReceived events and persists them to the repository.

use hodei_pipelines_domain::Result;
use hodei_pipelines_ports::{
    EventSubscriber, PipelineExecutionRepository, SystemEvent, event_bus::LogEntry,
};
use std::sync::Arc;
use tracing::{error, info, warn};

/// Service for persisting execution logs
pub struct LogPersistenceService<R, S>
where
    R: PipelineExecutionRepository + Send + Sync + 'static,
    S: EventSubscriber + Send + Sync + 'static,
{
    execution_repo: Arc<R>,
    event_subscriber: Arc<S>,
    retention_limit: usize,
}

impl<R, S> LogPersistenceService<R, S>
where
    R: PipelineExecutionRepository + Send + Sync + 'static,
    S: EventSubscriber + Send + Sync + 'static,
{
    /// Create a new log persistence service
    pub fn new(execution_repo: Arc<R>, event_subscriber: Arc<S>, retention_limit: usize) -> Self {
        Self {
            execution_repo,
            event_subscriber,
            retention_limit,
        }
    }

    /// Start the service
    pub async fn start(&self) -> Result<()> {
        info!("Starting Log Persistence Service");

        let mut rx = self.event_subscriber.subscribe().await.map_err(|e| {
            hodei_pipelines_domain::DomainError::Infrastructure(format!(
                "Failed to subscribe to event bus: {}",
                e
            ))
        })?;

        let execution_repo = self.execution_repo.clone();
        let retention_limit = self.retention_limit;

        tokio::spawn(async move {
            info!("Log Persistence Service loop started");
            loop {
                match rx.recv().await {
                    Ok(event) => match event {
                        SystemEvent::LogChunkReceived(_log_entry) => {
                            // No-op: We don't persist chunks anymore, only stream them via NATS.
                            // Self::handle_log_entry(&execution_repo, log_entry).await;
                        }
                        SystemEvent::JobCompleted {
                            job_id,
                            exit_code: _,
                            compressed_logs: Some(logs),
                        } => {
                            Self::handle_job_completed(
                                &execution_repo,
                                job_id,
                                logs,
                                retention_limit,
                            )
                            .await;
                        }
                        _ => {}
                    },
                    Err(e) => {
                        error!("Error receiving event in Log Persistence Service: {}", e);
                        if e.to_string().contains("closed") {
                            break;
                        }
                    }
                }
            }
            warn!("Log Persistence Service loop ended");
        });

        Ok(())
    }

    async fn handle_job_completed(
        execution_repo: &Arc<R>,
        job_id: hodei_pipelines_domain::pipeline_execution::value_objects::job_definitions::JobId,
        logs: Vec<u8>,
        retention_limit: usize,
    ) {
        let job_id_str = job_id.to_string();
        match execution_repo.find_step_by_job_id(&job_id_str).await {
            Ok(Some((execution_id, step_id))) => {
                // 1. Save compressed logs
                if let Err(e) = execution_repo.save_compressed_logs(&step_id, logs).await {
                    error!(
                        "Failed to save compressed logs for job {} (step {}): {}",
                        job_id, step_id, e
                    );
                } else {
                    info!("Persisted compressed logs for job {}", job_id);
                }

                // 2. Prune old executions
                // We need to fetch the execution to get the pipeline_id
                match execution_repo.get_execution(&execution_id).await {
                    Ok(Some(execution)) => {
                        if let Err(e) = execution_repo
                            .prune_old_executions(&execution.pipeline_id, retention_limit)
                            .await
                        {
                            error!(
                                "Failed to prune old executions for pipeline {}: {}",
                                execution.pipeline_id, e
                            );
                        }
                    }
                    Ok(None) => {
                        warn!("Execution {} not found for pruning", execution_id);
                    }
                    Err(e) => {
                        error!(
                            "Failed to fetch execution {} for pruning: {}",
                            execution_id, e
                        );
                    }
                }
            }
            Ok(None) => {
                warn!(
                    "Received logs for unknown job {}. It might belong to a finished execution.",
                    job_id
                );
            }
            Err(e) => {
                error!("Failed to lookup step for job {}: {}", job_id, e);
            }
        }
    }

    #[allow(dead_code)]
    async fn handle_log_entry(execution_repo: &Arc<R>, log_entry: LogEntry) {
        // Deprecated: kept for reference or potential fallback
        let job_id = log_entry.job_id;
        let log_content = String::from_utf8_lossy(&log_entry.data).to_string();

        match execution_repo
            .find_step_by_job_id(&job_id.to_string())
            .await
        {
            Ok(Some((execution_id, step_id))) => {
                if let Err(e) = execution_repo
                    .append_log(&execution_id, &step_id, log_content)
                    .await
                {
                    error!(
                        "Failed to append log for job {} (execution {}, step {}): {}",
                        job_id, execution_id, step_id, e
                    );
                }
            }
            Ok(None) => {}
            Err(e) => {
                error!("Failed to lookup step for job {}: {}", job_id, e);
            }
        }
    }
}
