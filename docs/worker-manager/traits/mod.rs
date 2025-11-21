//! Core traits y tipos para el Worker Manager abstraction layer
//!
//! Este módulo proporciona la interfaz abstracta que permite a diferentes
//! providers de infraestructura (Kubernetes, Docker, ECS, etc.) ofrecer
//! servicios uniformes de gestión de workers para el sistema CI/CD distribuido.

pub mod worker_manager_provider;
pub mod error_types;
pub mod types;

// Re-export todas las interfaces principales
pub use worker_manager_provider::*;
pub use error_types::*;
pub use types::*;

// Traits adicionales para funcionalidades específicas
mod additional_traits {
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use uuid::Uuid;

    use super::*;

    /// Trait para management de secretos en workers
    #[async_trait]
    pub trait SecretManager: Send + Sync {
        /// Crea un secreto temporal para el worker
        async fn create_worker_secret(
            &self,
            worker_id: &WorkerId,
            secret_spec: SecretSpec,
        ) -> Result<String, ProviderError>;

        /// Elimina secretos del worker
        async fn cleanup_worker_secrets(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

        /// Obtiene el estado de secretos de un worker
        async fn get_worker_secrets_status(&self, worker_id: &WorkerId) -> Result<Vec<SecretStatus>, ProviderError>;
    }

    /// Especificación de secreto para worker
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SecretSpec {
        pub name: String,
        pub data: HashMap<String, Vec<u8>>,
        pub mount_strategy: SecretMountStrategy,
        pub environment_vars: Option<HashMap<String, String>>,
    }

    /// Estado de un secreto
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SecretStatus {
        pub name: String,
        pub size_bytes: usize,
        pub mounted: bool,
        pub created_at: chrono::DateTime<chrono::Utc>,
    }

    /// Trait para networking y conectividad
    #[async_trait]
    pub trait NetworkManager: Send + Sync {
        /// Configura la red para un worker
        async fn setup_worker_network(
            &self,
            worker_id: &WorkerId,
            network_config: NetworkConfiguration,
        ) -> Result<NetworkInfo, ProviderError>;

        /// Elimina la configuración de red del worker
        async fn cleanup_worker_network(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

        /// Obtiene información de conectividad del worker
        async fn get_worker_network_info(&self, worker_id: &WorkerId) -> Result<NetworkInfo, ProviderError>;

        /// Ejecuta port forwarding para un worker
        async fn create_port_forward(
            &self,
            worker_id: &WorkerId,
            local_port: u16,
            remote_port: u16,
            protocol: NetworkProtocol,
        ) -> Result<PortForwardInfo, ProviderError>;

        /// Elimina un port forward existente
        async fn remove_port_forward(&self, worker_id: &WorkerId, local_port: u16) -> Result<(), ProviderError>;
    }

    /// Configuración de red para worker
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NetworkConfiguration {
        pub network_type: NetworkType,
        pub subnet: Option<String>,
        pub security_groups: Option<Vec<String>>,
        pub dns_settings: Option<DnsSettings>,
        pub proxy_settings: Option<ProxySettings>,
    }

    /// Información de red del worker
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NetworkInfo {
        pub worker_id: WorkerId,
        pub ip_address: String,
        pub hostname: String,
        pub network_interface: String,
        pub dns_servers: Vec<String>,
        pub created_at: chrono::DateTime<chrono::Utc>,
    }

    /// Configuración de DNS
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DnsSettings {
        pub nameservers: Vec<String>,
        pub search_domains: Vec<String>,
        pub options: Option<Vec<String>>,
    }

    /// Configuración de proxy
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ProxySettings {
        pub http_proxy: Option<String>,
        pub https_proxy: Option<String>,
        pub no_proxy: Option<Vec<String>>,
    }

    /// Protocolos de red soportados
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum NetworkProtocol {
        Tcp,
        Udp,
        Http,
        Https,
    }

    /// Información de port forward
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PortForwardInfo {
        pub local_port: u16,
        pub remote_port: u16,
        pub protocol: NetworkProtocol,
        pub tunnel_url: String,
        pub created_at: chrono::DateTime<chrono::Utc>,
    }

    /// Trait para management de volúmenes
    #[async_trait]
    pub trait VolumeManager: Send + Sync {
        /// Monta un volumen para el worker
        async fn mount_volume(
            &self,
            worker_id: &WorkerId,
            volume_config: VolumeMountConfiguration,
        ) -> Result<String, ProviderError>;

        /// Desmonta un volumen del worker
        async fn unmount_volume(&self, worker_id: &WorkerId, mount_path: String) -> Result<(), ProviderError>;

        /// Lista los volúmenes montados para un worker
        async fn list_worker_volumes(&self, worker_id: &WorkerId) -> Result<Vec<VolumeInfo>, ProviderError>;

        /// Expande un volumen dinámico
        async fn expand_volume(
            &self,
            volume_name: String,
            new_size_gb: u64,
        ) -> Result<(), ProviderError>;
    }

    /// Configuración de montaje de volumen
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct VolumeMountConfiguration {
        pub volume_source: VolumeSource,
        pub mount_path: String,
        pub read_only: bool,
        pub mount_options: Option<Vec<String>>,
    }

    /// Información de volumen montado
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct VolumeInfo {
        pub name: String,
        pub mount_path: String,
        pub size_gb: u64,
        pub used_gb: f64,
        pub volume_type: String,
        pub created_at: chrono::DateTime<chrono::Utc>,
    }

    /// Trait para monitoring y métricas
    #[async_trait]
    pub trait MonitoringManager: Send + Sync {
        /// Inicia métricas de monitoreo para un worker
        async fn start_monitoring(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

        /// Detiene métricas de monitoreo para un worker
        async fn stop_monitoring(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

        /// Obtiene métricas de rendimiento del worker
        async fn get_worker_metrics(&self, worker_id: &WorkerId) -> Result<WorkerMetrics, ProviderError>;

        /// Configura alertas para un worker
        async fn configure_alerts(
            &self,
            worker_id: &WorkerId,
            alert_config: AlertConfiguration,
        ) -> Result<(), ProviderError>;
    }

    /// Métricas de rendimiento de worker
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WorkerMetrics {
        pub worker_id: WorkerId,
        pub cpu_usage_percent: f64,
        pub memory_usage_mb: u64,
        pub memory_usage_percent: f64,
        pub disk_io_read: u64,
        pub disk_io_write: u64,
        pub network_rx_bytes: u64,
        pub network_tx_bytes: u64,
        pub uptime_seconds: u64,
        pub timestamp: chrono::DateTime<chrono::Utc>,
    }

    /// Configuración de alertas
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AlertConfiguration {
        pub cpu_threshold_percent: Option<f64>,
        pub memory_threshold_percent: Option<f64>,
        pub disk_threshold_percent: Option<f64>,
        pub custom_metrics: Option<HashMap<String, f64>>,
        pub notification_channels: Vec<String>,
    }

    /// Trait para backup y recuperación
    #[async_trait]
    pub trait BackupManager: Send + Sync {
        /// Crea backup del estado del worker
        async fn create_worker_backup(
            &self,
            worker_id: &WorkerId,
            backup_config: BackupConfiguration,
        ) -> Result<String, ProviderError>;

        /// Restaura worker desde backup
        async fn restore_worker_from_backup(
            &self,
            backup_id: String,
            restore_config: RestoreConfiguration,
        ) -> Result<WorkerId, ProviderError>;

        /// Lista backups disponibles para un worker
        async fn list_worker_backups(&self, worker_id: &WorkerId) -> Result<Vec<BackupInfo>, ProviderError>;

        /// Elimina un backup
        async fn delete_backup(&self, backup_id: String) -> Result<(), ProviderError>;
    }

    /// Configuración de backup
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BackupConfiguration {
        pub include_volumes: bool,
        pub include_secrets: bool,
        pub compression: bool,
        pub retention_days: u32,
    }

    /// Configuración de restauración
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RestoreConfiguration {
        pub restore_volumes: bool,
        pub restore_secrets: bool,
        pub overwrite_existing: bool,
    }

    /// Información de backup
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BackupInfo {
        pub backup_id: String,
        pub worker_id: WorkerId,
        pub size_bytes: u64,
        pub status: BackupStatus,
        pub created_at: chrono::DateTime<chrono::Utc>,
        pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    /// Estados de backup
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum BackupStatus {
        Pending,
        InProgress,
        Completed,
        Failed,
        Expired,
    }

    /// Trait para gestión de lifecycle de workers
    #[async_trait]
    pub trait LifecycleManager: Send + Sync {
        /// Inicia el proceso de creación del worker
        async fn begin_worker_creation(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

        /// Finaliza el proceso de creación del worker
        async fn complete_worker_creation(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

        /// Inicia el proceso de terminación del worker
        async fn begin_worker_termination(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

        /// Finaliza el proceso de terminación del worker
        async fn complete_worker_termination(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

        /// Pausa un worker en ejecución
        async fn pause_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

        /// Reanuda un worker pausado
        async fn resume_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

        /// Suspende un worker temporalmente
        async fn suspend_worker(&self, worker_id: &WorkerId, reason: String) -> Result<(), ProviderError>;

        /// Reanuda un worker suspendido
        async fn resume_suspended_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;
    }
}

pub use additional_traits::*;