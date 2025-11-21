//! Automatic Credential Rotation System
//! 
//! This module provides a complete automatic rotation system for credentials
//! with support for time-based, event-based, and manual rotation strategies.

use super::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration};

/// Rotation task state
#[derive(Debug, Clone)]
enum RotationTaskState {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Rotation task information
#[derive(Debug, Clone)]
struct RotationTask {
    id: uuid::Uuid,
    credential_name: String,
    strategy: RotationStrategy,
    state: RotationTaskState,
    created_at: chrono::DateTime<chrono::Utc>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    result: Option<RotationResult>,
    retry_count: u32,
    max_retries: u32,
}

/// Rotation event trigger
#[derive(Debug, Clone)]
pub struct RotationEvent {
    pub event_type: RotationEventType,
    pub credential_name: Option<String>,
    pub metadata: HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Types of rotation events
#[derive(Debug, Clone)]
pub enum RotationEventType {
    CredentialAccessed,
    UnauthorizedAccess,
    SecurityBreach,
    ComplianceCheck,
    ManualTrigger,
    ScheduledRotation,
    KeyRotation,
    CertificateExpiryWarning,
}

/// Rotation statistics
#[derive(Debug, Clone)]
struct RotationStatistics {
    total_rotations: u64,
    successful_rotations: u64,
    failed_rotations: u64,
    last_rotation: Option<chrono::DateTime<chrono::Utc>>,
    average_rotation_time: Duration,
    active_tasks: u32,
}

/// Rotation engine configuration
#[derive(Debug, Clone)]
pub struct RotationEngineConfig {
    pub max_concurrent_rotations: usize,
    pub default_retry_count: u32,
    pub rotation_timeout: Duration,
    pub audit_retention_days: u32,
    pub enable_metrics: bool,
    pub health_check_interval: Duration,
}

impl Default for RotationEngineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_rotations: 5,
            default_retry_count: 3,
            rotation_timeout: Duration::from_secs(300), // 5 minutes
            audit_retention_days: 90,
            enable_metrics: true,
            health_check_interval: Duration::from_secs(60),
        }
    }
}

/// Main rotation engine
#[derive(Debug)]
pub struct RotationEngine {
    credential_provider: Arc<dyn CredentialProvider + Send + Sync>,
    tasks: Arc<Mutex<HashMap<uuid::Uuid, RotationTask>>>,
    config: RotationEngineConfig,
    statistics: Arc<RwLock<RotationStatistics>>,
    audit_log: Arc<Mutex<Vec<RotationAuditEvent>>>,
    rotation_strategies: Arc<RwLock<HashMap<String, RotationStrategy>>>,
    event_handlers: Arc<RwLock<Vec<Box<dyn RotationEventHandler + Send + Sync>>>>,
    health_status: Arc<RwLock<RotationHealthStatus>>,
    shutdown_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

/// Rotation health status
#[derive(Debug, Clone)]
struct RotationHealthStatus {
    is_healthy: bool,
    last_health_check: chrono::DateTime<chrono::Utc>,
    issues: Vec<String>,
    running_tasks: u32,
}

/// Rotation audit event
#[derive(Debug, Clone)]
struct RotationAuditEvent {
    task_id: uuid::Uuid,
    credential_name: String,
    event_type: RotationAuditEventType,
    details: HashMap<String, String>,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
enum RotationAuditEventType {
    TaskCreated,
    TaskStarted,
    TaskCompleted,
    TaskFailed,
    TaskRetried,
    TaskCancelled,
    StrategyUpdated,
    EventTriggered,
}

impl RotationEngine {
    /// Create new rotation engine
    pub fn new(
        credential_provider: Arc<dyn CredentialProvider + Send + Sync>,
        config: RotationEngineConfig,
    ) -> Self {
        Self {
            credential_provider,
            tasks: Arc::new(Mutex::new(HashMap::new())),
            config,
            statistics: Arc::new(RwLock::new(RotationStatistics {
                total_rotations: 0,
                successful_rotations: 0,
                failed_rotations: 0,
                last_rotation: None,
                average_rotation_time: Duration::from_secs(0),
                active_tasks: 0,
            })),
            audit_log: Arc::new(Mutex::new(Vec::new())),
            rotation_strategies: Arc::new(RwLock::new(HashMap::new())),
            event_handlers: Arc::new(RwLock::new(Vec::new())),
            health_status: Arc::new(RwLock::new(RotationHealthStatus {
                is_healthy: true,
                last_health_check: chrono::Utc::now(),
                issues: Vec::new(),
                running_tasks: 0,
            })),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the rotation engine
    pub async fn start(&self) -> Result<(), CredentialError> {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        
        {
            let mut tx = self.shutdown_tx.lock().await;
            *tx = Some(shutdown_tx);
        }

        // Start background tasks
        let rotation_engine = self.clone();
        let rotation_task = tokio::spawn(async move {
            rotation_engine.rotation_loop(shutdown_rx).await;
        });

        let health_engine = self.clone();
        let health_task = tokio::spawn(async move {
            health_engine.health_check_loop().await;
        });

        // Wait for completion
        tokio::select! {
            result = rotation_task => {
                if let Err(e) = result {
                    eprintln!("Rotation task failed: {:?}", e);
                }
            }
            result = health_task => {
                if let Err(e) = result {
                    eprintln!("Health check task failed: {:?}", e);
                }
            }
        }

        Ok(())
    }

    /// Stop the rotation engine gracefully
    pub async fn stop(&self) -> Result<(), CredentialError> {
        let mut shutdown_tx = self.shutdown_tx.lock().await;
        if let Some(tx) = shutdown_tx.take() {
            let _ = tx.send(());
        }
        Ok(())
    }

    /// Clone with arc references
    fn clone(&self) -> Self {
        Self {
            credential_provider: self.credential_provider.clone(),
            tasks: self.tasks.clone(),
            config: self.config.clone(),
            statistics: self.statistics.clone(),
            audit_log: self.audit_log.clone(),
            rotation_strategies: self.rotation_strategies.clone(),
            event_handlers: self.event_handlers.clone(),
            health_status: self.health_status.clone(),
            shutdown_tx: self.shutdown_tx.clone(),
        }
    }

    /// Main rotation loop
    async fn rotation_loop(&self, shutdown_rx: tokio::sync::oneshot::Receiver<()>) {
        let mut time_based_timer = interval(Duration::from_secs(60)); // Check every minute
        let mut shutdown_signal = shutdown_rx;

        loop {
            tokio::select! {
                _ = time_based_timer.tick() => {
                    self.process_time_based_rotations().await;
                    self.process_pending_tasks().await;
                }
                _ = &mut shutdown_signal => {
                    break;
                }
            }
        }

        // Graceful shutdown: wait for active tasks to complete
        let timeout = tokio::time::sleep(Duration::from_secs(60));
        tokio::select! {
            _ = timeout => {
                // Force shutdown
            }
            _ = self.wait_for_active_tasks() => {
                // All tasks completed
            }
        }
    }

    /// Health check loop
    async fn health_check_loop(&self) {
        let mut timer = interval(self.config.health_check_interval);
        
        loop {
            timer.tick().await;
            
            // Check engine health
            let active_tasks = self.get_active_task_count().await;
            let mut issues = Vec::new();
            
            if active_tasks > self.config.max_concurrent_rotations {
                issues.push(format!("Too many active tasks: {}", active_tasks));
            }
            
            // Update health status
            {
                let mut status = self.health_status.write().await;
                status.is_healthy = issues.is_empty();
                status.last_health_check = chrono::Utc::now();
                status.issues = issues.clone();
                status.running_tasks = active_tasks;
            }
        }
    }

    /// Process time-based rotations
    async fn process_time_based_rotations(&self) {
        let strategies = self.rotation_strategies.read().await;
        let now = chrono::Utc::now();

        for (credential_name, strategy) in strategies.iter() {
            if let RotationStrategy::TimeBased(time_based) = strategy {
                // Check if rotation is due
                if let Ok(current_credential) = self.credential_provider.get_credential(credential_name, None).await {
                    let next_rotation = current_credential.credential.updated_at + time_based.rotation_interval;
                    
                    if now >= next_rotation {
                        let _ = self.schedule_rotation(credential_name, strategy.clone()).await;
                    }
                }
            }
        }
    }

    /// Process pending rotation tasks
    async fn process_pending_tasks(&self) {
        let mut tasks = self.tasks.lock().await;
        
        // Collect pending tasks
        let pending_tasks: Vec<uuid::Uuid> = tasks.values()
            .filter(|task| matches!(task.state, RotationTaskState::Pending))
            .map(|task| task.id)
            .collect();

        // Process each pending task
        for task_id in pending_tasks {
            let active_count = tasks.values().filter(|t| matches!(t.state, RotationTaskState::InProgress)).count();
            
            if active_count < self.config.max_concurrent_rotations {
                if let Some(task) = tasks.get_mut(&task_id) {
                    task.state = RotationTaskState::InProgress;
                    task.started_at = Some(chrono::Utc::now());
                    
                    // Start task execution in background
                    let engine_clone = self.clone();
                    let task_clone = task.clone();
                    tokio::spawn(async move {
                        engine_clone.execute_rotation_task(task_clone).await;
                    });
                }
            }
        }
    }

    /// Execute rotation task
    async fn execute_rotation_task(&self, task: RotationTask) {
        let start_time = chrono::Utc::now();
        
        // Update task state
        {
            let mut tasks = self.tasks.lock().await;
            if let Some(existing_task) = tasks.get_mut(&task.id) {
                existing_task.state = RotationTaskState::InProgress;
                existing_task.started_at = Some(start_time);
            }
        }

        // Audit task start
        self.audit_event(task.id.clone(), &task.credential_name, RotationAuditEventType::TaskStarted).await;

        let result = self.perform_rotation(&task.credential_name, &task.strategy).await;

        // Update task state and statistics
        {
            let mut tasks = self.tasks.lock().await;
            let mut task_update = tasks.get_mut(&task.id).unwrap();
            
            match result {
                Ok(rotation_result) => {
                    task_update.state = RotationTaskState::Completed;
                    task_update.completed_at = Some(chrono::Utc::now());
                    task_update.result = Some(rotation_result);
                    
                    // Update statistics
                    self.update_statistics(true, start_time).await;
                    
                    // Audit completion
                    self.audit_event(task.id.clone(), &task.credential_name, RotationAuditEventType::TaskCompleted).await;
                }
                Err(error) => {
                    task_update.retry_count += 1;
                    
                    if task_update.retry_count < task_update.max_retries {
                        task_update.state = RotationTaskState::Pending;
                        self.audit_event(task.id.clone(), &task.credential_name, RotationAuditEventType::TaskRetried).await;
                    } else {
                        task_update.state = RotationTaskState::Failed;
                        task_update.completed_at = Some(chrono::Utc::now());
                        self.update_statistics(false, start_time).await;
                        self.audit_event(task.id.clone(), &task.credential_name, RotationAuditEventType::TaskFailed).await;
                    }
                }
            }
        }
    }

    /// Perform actual rotation
    async fn perform_rotation(&self, credential_name: &str, strategy: &RotationStrategy) -> Result<RotationResult, CredentialError> {
        match self.credential_provider.rotate_credential(credential_name, strategy).await {
            Ok(result) => {
                // Trigger event handlers
                let event = RotationEvent {
                    event_type: RotationEventType::ScheduledRotation,
                    credential_name: Some(credential_name.to_string()),
                    metadata: HashMap::new(),
                    timestamp: chrono::Utc::now(),
                };
                
                self.trigger_event_handlers(&event).await;
                
                Ok(result)
            }
            Err(error) => {
                // Trigger failure event handlers
                let event = RotationEvent {
                    event_type: RotationEventType::SecurityBreach,
                    credential_name: Some(credential_name.to_string()),
                    metadata: HashMap::from([
                        ("error".to_string(), error.to_string()),
                        ("failure_type".to_string(), "rotation_failed".to_string()),
                    ]),
                    timestamp: chrono::Utc::now(),
                };
                
                self.trigger_event_handlers(&event).await;
                
                Err(error)
            }
        }
    }

    /// Schedule rotation task
    pub async fn schedule_rotation(&self, credential_name: &str, strategy: RotationStrategy) -> Result<uuid::Uuid, CredentialError> {
        let task = RotationTask {
            id: uuid::Uuid::new_v4(),
            credential_name: credential_name.to_string(),
            strategy,
            state: RotationTaskState::Pending,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            result: None,
            retry_count: 0,
            max_retries: self.config.default_retry_count,
        };

        {
            let mut tasks = self.tasks.lock().await;
            tasks.insert(task.id, task.clone());
        }

        // Audit task creation
        self.audit_event(task.id.clone(), credential_name, RotationAuditEventType::TaskCreated).await;

        Ok(task.id)
    }

    /// Trigger event-based rotation
    pub async fn trigger_event_rotation(&self, event: RotationEvent) -> Result<(), CredentialError> {
        // Audit event
        if let Some(ref credential_name) = event.credential_name {
            self.audit_event(uuid::Uuid::new_v4(), credential_name, RotationAuditEventType::EventTriggered).await;
        }

        // Trigger event handlers
        self.trigger_event_handlers(&event).await;

        // Schedule rotation if it's an event that requires it
        if matches!(event.event_type, RotationEventType::UnauthorizedAccess | 
                   RotationEventType::SecurityBreach | 
                   RotationEventType::ManualTrigger) {
            if let Some(credential_name) = &event.credential_name {
                // Use event-based strategy
                let strategy = RotationStrategy::EventBased(EventBasedRotation {
                    triggers: vec![event.event_type],
                    max_concurrent_rotations: 1,
                });
                
                let _ = self.schedule_rotation(credential_name, strategy).await;
            }
        }

        Ok(())
    }

    /// Update rotation strategy for a credential
    pub async fn update_rotation_strategy(&self, credential_name: &str, strategy: RotationStrategy) -> Result<(), CredentialError> {
        let mut strategies = self.rotation_strategies.write().await;
        strategies.insert(credential_name.to_string(), strategy.clone());
        
        // Audit strategy update
        self.audit_event(uuid::Uuid::new_v4(), credential_name, RotationAuditEventType::StrategyUpdated).await;

        Ok(())
    }

    /// Add event handler
    pub async fn add_event_handler(&self, handler: Box<dyn RotationEventHandler + Send + Sync>) -> Result<(), CredentialError> {
        let mut handlers = self.event_handlers.write().await;
        handlers.push(handler);
        Ok(())
    }

    /// Get rotation status
    pub async fn get_status(&self) -> RotationEngineStatus {
        let tasks = self.tasks.lock().await;
        let statistics = self.statistics.read().await;
        let health_status = self.health_status.read().await;

        RotationEngineStatus {
            total_tasks: tasks.len() as u32,
            pending_tasks: tasks.values().filter(|t| matches!(t.state, RotationTaskState::Pending)).count() as u32,
            active_tasks: tasks.values().filter(|t| matches!(t.state, RotationTaskState::InProgress)).count() as u32,
            completed_tasks: tasks.values().filter(|t| matches!(t.state, RotationTaskState::Completed)).count() as u32,
            failed_tasks: tasks.values().filter(|t| matches!(t.state, RotationTaskState::Failed)).count() as u32,
            statistics: statistics.clone(),
            health_status: health_status.clone(),
            configured_credentials: self.rotation_strategies.read().await.len() as u32,
        }
    }

    /// Get task details
    pub async fn get_task(&self, task_id: &uuid::Uuid) -> Option<RotationTask> {
        let tasks = self.tasks.lock().await;
        tasks.get(task_id).cloned()
    }

    /// List all tasks
    pub async fn list_tasks(&self, filter: Option<RotationTaskState>) -> Vec<RotationTask> {
        let tasks = self.tasks.lock().await;
        let mut task_list: Vec<RotationTask> = tasks.values().cloned().collect();
        
        if let Some(state) = filter {
            task_list.retain(|task| matches!(task.state, state));
        }
        
        task_list
    }

    /// Cancel rotation task
    pub async fn cancel_task(&self, task_id: &uuid::Uuid) -> Result<(), CredentialError> {
        let mut tasks = self.tasks.lock().await;
        if let Some(task) = tasks.get_mut(task_id) {
            match task.state {
                RotationTaskState::Pending | RotationTaskState::InProgress => {
                    task.state = RotationTaskState::Cancelled;
                    self.audit_event(task.id.clone(), &task.credential_name, RotationAuditEventType::TaskCancelled).await;
                    Ok(())
                }
                RotationTaskState::Completed | RotationTaskState::Failed | RotationTaskState::Cancelled => {
                    Err(CredentialError::PermissionDenied {
                        name: "Cannot cancel completed or failed task".to_string()
                    })
                }
            }
        } else {
            Err(CredentialError::NotFound {
                name: format!("Task {}", task_id)
            })
        }
    }

    // Helper methods
    async fn get_active_task_count(&self) -> u32 {
        let tasks = self.tasks.lock().await;
        tasks.values().filter(|t| matches!(t.state, RotationTaskState::InProgress)).count() as u32
    }

    async fn wait_for_active_tasks(&self) {
        loop {
            let active_count = self.get_active_task_count().await;
            if active_count == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    async fn update_statistics(&self, success: bool, start_time: chrono::DateTime<chrono::Utc>) {
        let mut stats = self.statistics.write().await;
        let duration = chrono::Utc::now() - start_time;
        let duration_secs = duration.num_seconds() as u64;

        stats.total_rotations += 1;
        if success {
            stats.successful_rotations += 1;
        } else {
            stats.failed_rotations += 1;
        }
        stats.last_rotation = Some(chrono::Utc::now());

        // Update average time (simple moving average)
        let current_avg = stats.average_rotation_time.as_secs();
        let new_avg = (current_avg * (stats.total_rotations - 1) + duration_secs) / stats.total_rotations;
        stats.average_rotation_time = Duration::from_secs(new_avg);
    }

    async fn audit_event(&self, task_id: uuid::Uuid, credential_name: &str, event_type: RotationAuditEventType) {
        let audit_event = RotationAuditEvent {
            task_id,
            credential_name: credential_name.to_string(),
            event_type,
            details: HashMap::new(),
            timestamp: chrono::Utc::now(),
        };

        let mut log = self.audit_log.lock().await;
        log.push(audit_event);

        // Limit audit log size
        if log.len() > 10000 {
            log.drain(0..5000);
        }
    }

    async fn trigger_event_handlers(&self, event: &RotationEvent) {
        let handlers = self.event_handlers.read().await;
        for handler in handlers.iter() {
            let _ = handler.handle_event(event).await;
        }
    }
}

/// Rotation event handler trait
#[async_trait]
pub trait RotationEventHandler: Send + Sync {
    async fn handle_event(&self, event: &RotationEvent) -> Result<(), CredentialError>;
}

/// Rotation engine status
#[derive(Debug, Clone)]
pub struct RotationEngineStatus {
    pub total_tasks: u32,
    pub pending_tasks: u32,
    pub active_tasks: u32,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
    pub statistics: RotationStatistics,
    pub health_status: RotationHealthStatus,
    pub configured_credentials: u32,
}

/// Built-in rotation event handlers

/// Keycloak rotation event handler
pub struct KeycloakRotationHandler {
    keycloak_client: Arc<super::keycloak::KeycloakClient>,
}

impl KeycloakRotationHandler {
    pub fn new(keycloak_client: Arc<super::keycloak::KeycloakClient>) -> Self {
        Self { keycloak_client }
    }
}

#[async_trait::async_trait]
impl RotationEventHandler for KeycloakRotationHandler {
    async fn handle_event(&self, event: &RotationEvent) -> Result<(), CredentialError> {
        match event.event_type {
            RotationEventType::UnauthorizedAccess => {
                // Trigger immediate token refresh
                {
                    let mut cache = self.keycloak_client.token_cache.write().map_err(|_| CredentialError::InternalError)?;
                    cache.clear();
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// AWS Secrets Manager rotation event handler
pub struct AWSRotationHandler {
    // Would include AWS-specific configuration
    #[allow(dead_code)]
    region: String,
}

impl AWSRotationHandler {
    pub fn new(region: String) -> Self {
        Self { region }
    }
}

#[async_trait::async_trait]
impl RotationEventHandler for AWSRotationHandler {
    async fn handle_event(&self, event: &RotationEvent) -> Result<(), CredentialError> {
        match event.event_type {
            RotationEventType::ComplianceCheck => {
                // Log compliance check event
                println!("AWS compliance check triggered for credential: {:?}", event.credential_name);
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// Vault rotation event handler
pub struct VaultRotationHandler {
    // Would include Vault-specific configuration
    #[allow(dead_code)]
    vault_url: String,
}

impl VaultRotationHandler {
    pub fn new(vault_url: String) -> Self {
        Self { vault_url }
    }
}

#[async_trait::async_trait]
impl RotationEventHandler for VaultRotationHandler {
    async fn handle_event(&self, event: &RotationEvent) -> Result<(), CredentialError> {
        match event.event_type {
            RotationEventType::KeyRotation => {
                // Trigger key rotation in Vault transit engine
                println!("Vault key rotation triggered for credential: {:?}", event.credential_name);
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
