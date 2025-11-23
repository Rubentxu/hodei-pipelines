//! gRPC Client for HWP Agent
//!
//! This module implements the gRPC client that connects to the HWP server
//! and handles bidirectional streaming for job execution, logging, and monitoring.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{RwLock, mpsc};
use tonic::{Request, Response, Status, Streaming, transport::Channel};
use tracing::{error, info, warn};

use hwp_proto::hwp::{self, WorkerServiceClient};
use hwp_proto::hwp::{AssignJobRequest, JobAccepted, JobResult, LogEntry};

use crate::connection::auth::AuthInterceptor;
use crate::executor::{JobExecutor, ProcessManager};
use crate::{AgentError, Config, Result};

/// gRPC client wrapper
#[derive(Debug)]
pub struct Client {
    config: Config,
    channel: Option<Channel>,
    interceptor: AuthInterceptor,
    process_manager: Arc<ProcessManager>,
}

impl Client {
    /// Create a new client
    pub fn new(config: Config) -> Self {
        Self {
            config,
            channel: None,
            interceptor: AuthInterceptor::new(),
            process_manager: Arc::new(ProcessManager::new()),
        }
    }

    /// Connect to the server
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to server at {}", self.config.server_url);

        let channel = Channel::from_shared(self.config.server_url.clone())
            .map_err(|e| AgentError::Connection(e.to_string()))?
            .connect()
            .await
            .map_err(|e| AgentError::Connection(e.to_string()))?;

        self.channel = Some(channel);
        info!("Successfully connected to server");
        Ok(())
    }

    /// Handle the bidirectional stream
    pub async fn handle_stream(&mut self) -> Result<()> {
        let channel = self
            .channel
            .as_ref()
            .ok_or_else(|| AgentError::Connection("Not connected".to_string()))?;

        // Create WorkerServiceClient with interceptor
        let mut client =
            WorkerServiceClient::with_interceptor(channel.clone(), self.interceptor.clone());

        // Register this worker with the server
        info!("Registering worker with server");
        let registration = hwp::WorkerRegistration {
            worker_id: self.config.worker_id.clone(),
            capabilities: vec!["docker".to_string(), "kubernetes".to_string()],
        };

        let _status: Response<hwp::WorkerStatus> = client
            .register_worker(Request::new(registration))
            .await
            .map_err(|e| AgentError::Connection(format!("Registration failed: {}", e)))?;

        info!("Worker registered successfully");

        // Setup bidirectional streaming
        info!("Starting bidirectional stream for job assignments and logs");

        // Channel for sending logs to server
        let (log_sender, mut log_receiver) = mpsc::channel::<LogEntry>(100);

        // Spawn task to process log stream
        let client_clone = client.clone();
        let log_task = tokio::spawn(async move {
            while let Some(log_entry) = log_receiver.recv().await {
                if let Err(e) = client_clone
                    .stream_logs(Request::new(tokio_stream::wrappers::ReceiverStream::new(
                        tokio_stream::wrappers::mpsc::ReceiverStream::new(mpsc::channel(1).1),
                    )))
                    .await
                {
                    error!("Failed to send log: {}", e);
                }
            }
            Ok::<_, Status>(())
        });

        // Setup channel for receiving job assignments
        let (job_sender, job_receiver) = mpsc::channel::<AssignJobRequest>(100);

        // Create stream request
        let mut stream = client
            .assign_job_stream(Request::new(tokio_stream::wrappers::ReceiverStream::new(
                job_receiver,
            )))
            .await
            .map_err(|e| AgentError::Stream(format!("Failed to open stream: {}", e)))?
            .into_inner();

        // Main loop: receive and process job assignments
        loop {
            tokio::select! {
                // Receive job assignments from server
                result = stream.message() => {
                    match result {
                        Ok(Some(job_request)) => {
                            info!("Received job assignment: {}", job_request.job_id);
                            self.handle_job_request(
                                job_request,
                                log_sender.clone(),
                            ).await;
                        }
                        Ok(None) => {
                            warn!("Stream closed by server");
                            break;
                        }
                        Err(e) => {
                            error!("Error receiving job: {}", e);
                            break;
                        }
                    }
                }
                // Check for shutdown
                _ = tokio::signal::ctrl_c() => {
                    info!("Received Ctrl-C, shutting down stream");
                    break;
                }
            }
        }

        info!("Stream handler ended");
        Ok(())
    }

    /// Handle a job assignment
    async fn handle_job_request(
        &mut self,
        job_request: AssignJobRequest,
        log_sender: mpsc::Sender<LogEntry>,
    ) {
        let job_id = job_request.job_id.clone();
        let command = job_request.command.clone();

        // Send acknowledgment
        if let Err(e) = log_sender
            .send(LogEntry {
                job_id: job_id.clone(),
                data: format!("Job {} accepted", job_id),
            })
            .await
        {
            error!("Failed to send job acknowledgment: {}", e);
        }

        // Spawn task to execute job
        let process_manager = self.process_manager.clone();
        let log_sender_clone = log_sender.clone();

        tokio::spawn(async move {
            let result =
                Self::execute_job(&process_manager, job_id.clone(), command, log_sender_clone)
                    .await;

            match result {
                Ok(exit_code) => {
                    info!("Job {} completed with exit code {}", job_id, exit_code);
                    // Send job result (in real implementation, send to server)
                }
                Err(e) => {
                    error!("Job {} failed: {}", job_id, e);
                    // Send job failure (in real implementation, send to server)
                }
            }
        });
    }

    /// Execute a job
    async fn execute_job(
        process_manager: &ProcessManager,
        job_id: String,
        command: Vec<String>,
        log_sender: mpsc::Sender<LogEntry>,
    ) -> Result<i32, String> {
        info!("Executing job {}: {:?}", job_id, command);

        // Convert command to proper format for ProcessManager
        let mut env_vars = HashMap::new();
        let working_dir = None;

        // Spawn the job
        let job_handle_result = process_manager
            .spawn_job(command, env_vars, working_dir)
            .await
            .map_err(|e| e.to_string())?;

        let pid = process_manager
            .get_job(&job_handle_result)
            .await
            .ok_or("Failed to get job info")?
            .pid;

        info!("Job {} started with PID: {}", job_id, pid);

        // In a full implementation, we would:
        // 1. Wait for the process to complete
        // 2. Capture stdout/stderr
        // 3. Send logs to server via log_sender
        // 4. Return exit code

        // For now, just simulate execution
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send final log
        if let Err(e) = log_sender
            .send(LogEntry {
                job_id: job_id.clone(),
                data: format!("Job {} completed", job_id),
            })
            .await
        {
            error!("Failed to send final log: {}", e);
        }

        Ok(0) // Success exit code
    }

    /// Get the server URL
    pub fn server_url(&self) -> &str {
        &self.config.server_url
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.channel.is_some()
    }

    /// Get process manager reference
    pub fn process_manager(&self) -> &ProcessManager {
        &self.process_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let config = Config::default();
        let client = Client::new(config);
        assert!(!client.is_connected());
    }
}
