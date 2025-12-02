//! gRPC Client for HWP Agent
//!
//! This module implements the gRPC client that connects to the HWP server
//! and handles bidirectional streaming for job execution, logging, and monitoring.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;
use tonic::{Request, Response};
use tracing::{error, info, warn};

use hodei_pipelines_proto::pb::agent_message::Payload as AgentPayload;
use hodei_pipelines_proto::pb::server_message::Payload as ServerPayload;
use hodei_pipelines_proto::{
    AgentMessage, AssignJobRequest, JobAccepted, JobResult, LogEntry, ServerMessage,
    WorkerRegistration, WorkerServiceClient, WorkerStatus,
};

use hodei_pipelines_ports::{WorkerClient, WorkerClientError};

use crate::connection::auth::AuthInterceptor;
use crate::executor::ProcessManager;
use crate::executor::pty::{PtyAllocation, PtySizeConfig};
use crate::{AgentError, Config, Result};

// Re-export tonic types for TLS
use flate2::Compression;
use flate2::write::GzEncoder;
use std::io::Write;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tonic::transport::{Certificate, Identity};

/// gRPC client wrapper
#[derive(Debug, Clone)]
pub struct Client {
    config: Config,
    channel: Option<Channel>,
    interceptor: AuthInterceptor,
    process_manager: Arc<ProcessManager>,
    /// Active jobs being executed
    active_jobs: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl Client {
    /// Create a new client
    pub fn new(config: Config) -> Self {
        Self {
            config,
            channel: None,
            interceptor: AuthInterceptor::new(),
            process_manager: Arc::new(ProcessManager::new()),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connect to the server
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to server at {}", self.config.server_url);

        let mut endpoint = Channel::from_shared(self.config.server_url.clone())
            .map_err(|e| AgentError::Connection(e.to_string()))?;

        if self.config.tls_enabled {
            info!("Configuring mTLS connection");
            let cert_path = self
                .config
                .tls_cert_path
                .as_ref()
                .ok_or_else(|| AgentError::Connection("Missing TLS cert path".to_string()))?;
            let key_path = self
                .config
                .tls_key_path
                .as_ref()
                .ok_or_else(|| AgentError::Connection("Missing TLS key path".to_string()))?;
            let ca_path = self
                .config
                .tls_ca_path
                .as_ref()
                .ok_or_else(|| AgentError::Connection("Missing TLS CA path".to_string()))?;

            let cert = std::fs::read_to_string(cert_path)
                .map_err(|e| AgentError::Connection(format!("Failed to read cert: {}", e)))?;
            let key = std::fs::read_to_string(key_path)
                .map_err(|e| AgentError::Connection(format!("Failed to read key: {}", e)))?;
            let ca = std::fs::read_to_string(ca_path)
                .map_err(|e| AgentError::Connection(format!("Failed to read CA: {}", e)))?;

            let identity = Identity::from_pem(cert, key);
            let ca_cert = Certificate::from_pem(ca);

            let tls_config = tonic::transport::ClientTlsConfig::new()
                .domain_name("localhost")
                .identity(identity)
                .ca_certificate(ca_cert);

            endpoint = endpoint
                .tls_config(tls_config)
                .map_err(|e| AgentError::Connection(format!("Failed to configure TLS: {}", e)))?;
        }

        let channel = endpoint
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

        // Create the gRPC client with auth interceptor
        let mut client =
            WorkerServiceClient::with_interceptor(channel.clone(), self.interceptor.clone());

        // Create the outgoing channel for messages from agent to server
        let (_tx, rx) = mpsc::channel(100);
        let outbound = ReceiverStream::new(rx);

        // Connect to the server with the bidirectional stream
        let response = client
            .job_stream(outbound)
            .await
            .map_err(|e| AgentError::Connection(e.to_string()))?;

        let _inbound = response.into_inner();
        info!("Bidirectional stream established");

        // Send initial registration
        let registration = WorkerRegistration {
            worker_id: self.config.worker_id.clone(),
            capabilities: vec!["linux".to_string(), "docker".to_string(), "pty".to_string()],
        };

        let response: Response<WorkerStatus> = client
            .register_worker(Request::new(registration))
            .await
            .map_err(|e| AgentError::Connection(format!("Registration failed: {}", e)))?;

        info!(
            "Worker registered successfully with state: {}",
            response.get_ref().state
        );

        // Setup bidirectional streaming
        info!("Starting bidirectional stream for job assignments");

        // Channel for sending messages to server (logs, job results, etc.)
        let (tx, rx) = mpsc::channel::<AgentMessage>(100);

        // Create the bidirectional stream
        let request_stream = ReceiverStream::new(rx);
        let mut request = Request::new(request_stream);
        request
            .metadata_mut()
            .insert("worker-id", self.config.worker_id.parse().unwrap());

        let mut response_stream = client
            .job_stream(request)
            .await
            .map_err(|e| AgentError::Stream(format!("Failed to open JobStream: {}", e)))?
            .into_inner();

        info!("Bidirectional stream established, waiting for job assignments");

        // Main loop: receive messages from server and process them
        loop {
            tokio::select! {
                // Receive messages from server (job assignments, cancellations)
                result = response_stream.next() => {
                    match result {
                        Some(Ok(server_msg)) => {
                            if let Err(e) = self.handle_server_message(server_msg, tx.clone()).await {
                                error!("Error handling server message: {}", e);
                            }
                        }
                        Some(Err(e)) => {
                            error!("Stream error: {}", e);
                            return Err(AgentError::Stream(format!("Stream error: {}", e)));
                        }
                        None => {
                            warn!("Stream closed by server");
                            break;
                        }
                    }
                }
                // Check for shutdown signal
                _ = tokio::signal::ctrl_c() => {
                    info!("Received Ctrl-C, shutting down stream");
                    break;
                }
            }
        }

        info!("Stream handler ended");
        Ok(())
    }

    /// Handle a message from the server
    async fn handle_server_message(
        &mut self,
        server_msg: ServerMessage,
        tx: mpsc::Sender<AgentMessage>,
    ) -> Result<()> {
        match server_msg.payload {
            Some(ServerPayload::AssignJob(job_request)) => {
                info!("Received job assignment: {}", job_request.job_id);
                self.handle_job_assignment(job_request, tx).await?;
            }
            Some(ServerPayload::CancelJob(cancel_request)) => {
                warn!("Received job cancellation: {}", cancel_request.job_id);
                self.handle_job_cancellation(cancel_request.job_id).await?;
            }
            None => {
                warn!("Received empty server message");
            }
        }
        Ok(())
    }

    /// Handle a job assignment from the server
    async fn handle_job_assignment(
        &mut self,
        job_request: AssignJobRequest,
        tx: mpsc::Sender<AgentMessage>,
    ) -> Result<()> {
        let job_id = job_request.job_id.clone();
        let job_id_for_tracking = job_id.clone();

        // Send job accepted acknowledgment
        let ack = AgentMessage {
            payload: Some(AgentPayload::JobAccepted(JobAccepted {
                job_id: job_id.clone(),
            })),
        };

        if let Err(e) = tx.send(ack).await {
            error!("Failed to send job acceptance: {}", e);
            return Err(AgentError::Connection(format!(
                "Failed to send job acceptance: {}",
                e
            )));
        }

        info!("Job {} accepted, starting execution", job_id);

        // Spawn a task to execute the job
        let process_manager = self.process_manager.clone();
        let active_jobs = self.active_jobs.clone();
        let tx_clone = tx.clone();

        let job_handle = tokio::spawn(async move {
            let result = Self::execute_job(
                &process_manager,
                job_id.clone(),
                job_request,
                tx_clone.clone(),
            )
            .await;

            // Send job result
            let job_result = match result {
                Ok((exit_code, compressed_logs)) => {
                    info!("Job {} completed with exit code {}", job_id, exit_code);
                    AgentMessage {
                        payload: Some(AgentPayload::JobResult(JobResult {
                            job_id: job_id.clone(),
                            exit_code,
                            stdout: "".to_string(),
                            stderr: "".to_string(),
                            compressed_logs,
                        })),
                    }
                }
                Err(e) => {
                    error!("Job {} failed: {}", job_id, e);
                    AgentMessage {
                        payload: Some(AgentPayload::JobResult(JobResult {
                            job_id: job_id.clone(),
                            exit_code: -1,
                            stdout: "".to_string(),
                            stderr: e.to_string(),
                            compressed_logs: vec![],
                        })),
                    }
                }
            };

            // Send final result
            if let Err(e) = tx_clone.send(job_result).await {
                error!("Failed to send job result: {}", e);
            }

            // Remove from active jobs
            active_jobs.write().await.remove(&job_id);
        });

        // Track the job
        self.active_jobs
            .write()
            .await
            .insert(job_id_for_tracking, job_handle);

        Ok(())
    }

    /// Execute a job using ProcessManager
    async fn execute_job(
        process_manager: &ProcessManager,
        job_id: String,
        job_request: AssignJobRequest,
        tx: mpsc::Sender<AgentMessage>,
    ) -> std::result::Result<(i32, Vec<u8>), String> {
        let job_spec = job_request
            .job_spec
            .as_ref()
            .ok_or_else(|| "Missing job_spec in AssignJobRequest".to_string())?;

        info!(
            "Executing job {}: image={}, command={:?}",
            job_id, job_spec.image, job_spec.command
        );

        // Send initial log
        let _ = tx
            .send(AgentMessage {
                payload: Some(AgentPayload::LogEntry(LogEntry {
                    job_id: job_id.clone(),
                    data: format!("Starting job: {}", job_spec.name),
                })),
            })
            .await;

        // Prepare command
        let command = if job_spec.command.is_empty() {
            vec!["echo".to_string(), "No command specified".to_string()]
        } else {
            job_spec.command.clone()
        };

        // Create PTY allocation (stub for now)
        let pty_allocation = PtyAllocation::new(PtySizeConfig::default())
            .map_err(|e| format!("Failed to create PTY: {}", e))?;

        // Create log file
        let log_file_path = format!("/tmp/hodei_job_{}.log", job_id);
        let log_file = tokio::fs::File::create(&log_file_path)
            .await
            .map_err(|e| format!("Failed to create log file: {}", e))?;
        let mut log_writer = tokio::io::BufWriter::new(log_file);

        // Spawn the job
        let env_vars = HashMap::new();
        let working_dir = None;

        let (spawned_job_id, stdout, stderr) = process_manager
            .spawn_job(command.clone(), env_vars, working_dir, &pty_allocation)
            .await
            .map_err(|e| format!("Failed to spawn job: {}", e))?;

        // Channels for log processing
        let (log_tx, mut log_rx) = mpsc::channel::<String>(100);

        // Stdout reader
        if let Some(out) = stdout {
            let log_tx = log_tx.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncBufReadExt;
                let mut reader = tokio::io::BufReader::new(out).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if let Err(_) = log_tx.send(line).await {
                        break;
                    }
                }
            });
        }

        // Stderr reader
        if let Some(err) = stderr {
            let log_tx = log_tx.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncBufReadExt;
                let mut reader = tokio::io::BufReader::new(err).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if let Err(_) = log_tx.send(line).await {
                        break;
                    }
                }
            });
        }

        // Drop original sender to allow loop to finish when readers are done
        drop(log_tx);

        // Log processor task
        let tx_clone = tx.clone();
        let job_id_clone = job_id.clone();

        let log_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            while let Some(line) = log_rx.recv().await {
                // Write to file
                if let Err(e) = log_writer.write_all(format!("{}\n", line).as_bytes()).await {
                    error!("Failed to write to log file: {}", e);
                }
                if let Err(e) = log_writer.flush().await {
                    error!("Failed to flush log file: {}", e);
                }

                // Send to gRPC (Real-time)
                let _ = tx_clone
                    .send(AgentMessage {
                        payload: Some(AgentPayload::LogEntry(LogEntry {
                            job_id: job_id_clone.clone(),
                            data: line,
                        })),
                    })
                    .await;
            }
            // Ensure everything is written
            let _ = log_writer.flush().await;
        });

        // Wait for the job to complete
        let exit_code = process_manager
            .wait_for_job(&spawned_job_id)
            .await
            .map_err(|e| format!("Failed to wait for job: {}", e))?;

        // Wait for log processing to finish
        let _ = log_task.await;

        // Compress logs
        let log_content = tokio::fs::read(&log_file_path)
            .await
            .map_err(|e| format!("Failed to read log file: {}", e))?;

        let compressed_logs = tokio::task::spawn_blocking(move || {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&log_content).map_err(|e| e.to_string())?;
            encoder.finish().map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("Compression task failed: {}", e))?
        .map_err(|e| format!("Compression failed: {}", e))?;

        // Cleanup log file
        let _ = tokio::fs::remove_file(&log_file_path).await;

        // Send completion log (optional, maybe just rely on status)
        let _ = tx
            .send(AgentMessage {
                payload: Some(AgentPayload::LogEntry(LogEntry {
                    job_id: job_id.clone(),
                    data: format!("Job completed with exit code: {}", exit_code),
                })),
            })
            .await;

        Ok((exit_code, compressed_logs))
    }

    /// Handle a job cancellation request
    async fn handle_job_cancellation(&mut self, job_id: String) -> Result<()> {
        info!("Cancelling job: {}", job_id);

        // Look up the job and abort it
        let mut jobs = self.active_jobs.write().await;
        if let Some(handle) = jobs.remove(&job_id) {
            handle.abort();
            info!("Job {} cancelled", job_id);
        } else {
            warn!("Job {} not found in active jobs", job_id);
        }

        Ok(())
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

use async_trait::async_trait;

#[async_trait]
impl WorkerClient for Client {
    async fn assign_job(
        &self,
        _worker_id: &hodei_pipelines_core::WorkerId,
        _job_id: &hodei_pipelines_core::JobId,
        _job_spec: &hodei_pipelines_core::JobSpec,
    ) -> std::result::Result<(), WorkerClientError> {
        Err(WorkerClientError::NotAvailable)
    }

    async fn cancel_job(
        &self,
        _worker_id: &hodei_pipelines_core::WorkerId,
        _job_id: &hodei_pipelines_core::JobId,
    ) -> std::result::Result<(), WorkerClientError> {
        Err(WorkerClientError::NotAvailable)
    }

    async fn get_worker_status(
        &self,
        _worker_id: &hodei_pipelines_core::WorkerId,
    ) -> std::result::Result<hodei_pipelines_core::WorkerStatus, WorkerClientError> {
        Err(WorkerClientError::NotAvailable)
    }

    async fn send_heartbeat(
        &self,
        worker_id: &hodei_pipelines_core::WorkerId,
        resource_usage: &hodei_pipelines_core::ResourceUsage,
    ) -> std::result::Result<(), WorkerClientError> {
        let channel = self
            .channel
            .as_ref()
            .ok_or_else(|| WorkerClientError::Connection("Not connected".to_string()))?;

        let mut client =
            WorkerServiceClient::with_interceptor(channel.clone(), self.interceptor.clone());

        let proto_usage = hodei_pipelines_proto::ResourceUsage {
            cpu_usage_m: resource_usage.cpu_usage_m,
            memory_usage_mb: resource_usage.memory_usage_mb,
            active_jobs: resource_usage.active_jobs,
            disk_read_mb: resource_usage.disk_read_mb,
            disk_write_mb: resource_usage.disk_write_mb,
            network_sent_mb: resource_usage.network_sent_mb,
            network_received_mb: resource_usage.network_received_mb,
            gpu_utilization_percent: resource_usage.gpu_utilization_percent,
            timestamp: resource_usage.timestamp,
        };

        let request = hodei_pipelines_proto::HeartbeatRequest {
            worker_id: worker_id.to_string(),
            timestamp: resource_usage.timestamp,
            resource_usage: Some(proto_usage),
        };

        client
            .heartbeat(Request::new(request))
            .await
            .map_err(|e| WorkerClientError::Communication(e.to_string()))?;

        Ok(())
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

    #[tokio::test]
    async fn test_compression_logic() {
        use flate2::read::GzDecoder;
        use std::io::Read;

        // 1. Create dummy log data
        let log_data = "This is a test log line.\nAnd another one.\n";
        let log_bytes = log_data.as_bytes();

        // 2. Compress it (simulating what execute_job does)
        let compressed_logs = tokio::task::spawn_blocking(move || {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(log_bytes).unwrap();
            encoder.finish().unwrap()
        })
        .await
        .unwrap();

        // 3. Decompress it
        let mut decoder = GzDecoder::new(&compressed_logs[..]);
        let mut decompressed_string = String::new();
        decoder.read_to_string(&mut decompressed_string).unwrap();

        // 4. Verify
        assert_eq!(decompressed_string, log_data);
        assert!(compressed_logs.len() > 0);
    }
}
