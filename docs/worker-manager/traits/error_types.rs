//! Tipos de error específicos para el Worker Manager
//!
//! Este módulo define todas las categorías de errores que pueden ocurrir
//! durante la gestión de workers en diferentes providers de infraestructura.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Resultado de una operación del provider
pub type ProviderResult<T> = Result<T, ProviderError>;

/// Códigos de error estandarizados
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCode {
    /// Error genérico de infraestructura
    Infrastructure,
    /// Error de autenticación/autorización
    Authentication,
    /// Error de autorización
    Authorization,
    /// Recurso no encontrado
    NotFound,
    /// Recurso ya existe
    AlreadyExists,
    /// Error de validación
    Validation,
    /// Error de límite de recursos
    ResourceExhausted,
    /// Timeout en operación
    Timeout,
    /// Operación cancelada
    Cancelled,
    /// Error de red
    Network,
    /// Error de configuración
    Configuration,
    /// Error de compatibilidad
    Compatibility,
    /// Error interno del sistema
    Internal,
    /// Error de credenciales
    Credentials,
    /// Error de secretos
    Secrets,
    /// Error de volumen/montaje
    Volume,
    /// Error de red
    NetworkError,
    /// Error de seguridad
    Security,
    /// Error de rate limiting
    RateLimit,
    /// Error específico del provider
    ProviderSpecific(String),
}

/// Categorías de error para manejo específico
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Errores de validación que deben ser corregidos por el cliente
    ClientValidation,
    /// Errores de autenticación/autorización
    Authorization,
    /// Errores temporales de infraestructura que se pueden reintentar
    TransientInfrastructure,
    /// Errores permanentes de infraestructura
    PermanentInfrastructure,
    /// Errores de configuración del sistema
    Configuration,
    /// Errores de límites de recursos
    ResourceLimits,
    /// Errores internos del sistema
    Internal,
}

/// Severidad del error
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Error crítico que requiere atención inmediata
    Critical,
    /// Error alto que puede afectar la funcionalidad
    High,
    /// Error medio que puede degradar el rendimiento
    Medium,
    /// Error bajo que tiene impacto mínimo
    Low,
}

/// Contexto adicional para el error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Identificador de la operación que falló
    pub operation_id: Option<String>,
    /// Worker ID asociado con el error (si aplica)
    pub worker_id: Option<String>,
    /// Provider que generó el error
    pub provider: Option<String>,
    /// Configuración específica en el momento del error
    pub configuration: Option<HashMap<String, String>>,
    /// Timestamp del error
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Error principal del Worker Manager
#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum ProviderError {
    /// Error de infraestructura genérico
    #[error("Infrastructure error: {message}")]
    Infrastructure {
        message: String,
        code: ErrorCode,
        context: ErrorContext,
    },

    /// Error de autenticación/autorización
    #[error("Authentication error: {message}")]
    Authentication {
        message: String,
        context: ErrorContext,
    },

    /// Error de autorización
    #[error("Authorization error: {message}")]
    Authorization {
        message: String,
        context: ErrorContext,
    },

    /// Recurso no encontrado
    #[error("Resource not found: {message}")]
    NotFound {
        message: String,
        resource_type: String,
        resource_id: String,
        context: ErrorContext,
    },

    /// Recurso ya existe
    #[error("Resource already exists: {message}")]
    AlreadyExists {
        message: String,
        resource_type: String,
        resource_id: String,
        context: ErrorContext,
    },

    /// Error de validación
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field_errors: HashMap<String, String>,
        context: ErrorContext,
    },

    /// Error de límite de recursos
    #[error("Resource exhausted: {message}")]
    ResourceExhausted {
        message: String,
        resource_type: String,
        current_usage: u64,
        limit: u64,
        context: ErrorContext,
    },

    /// Timeout en operación
    #[error("Operation timeout: {message}")]
    Timeout {
        message: String,
        operation: String,
        timeout_duration: std::time::Duration,
        context: ErrorContext,
    },

    /// Operación cancelada
    #[error("Operation cancelled: {message}")]
    Cancelled {
        message: String,
        operation: String,
        context: ErrorContext,
    },

    /// Error de red
    #[error("Network error: {message}")]
    Network {
        message: String,
        endpoint: Option<String>,
        status_code: Option<u16>,
        context: ErrorContext,
    },

    /// Error de configuración
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        config_path: Option<String>,
        context: ErrorContext,
    },

    /// Error de compatibilidad
    #[error("Compatibility error: {message}")]
    Compatibility {
        message: String,
        current_version: String,
        required_version: String,
        context: ErrorContext,
    },

    /// Error interno del sistema
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        source: Option<String>,
        context: ErrorContext,
    },

    /// Error de credenciales
    #[error("Credentials error: {message}")]
    Credentials {
        message: String,
        credential_type: String,
        context: ErrorContext,
    },

    /// Error de secretos
    #[error("Secrets error: {message}")]
    Secrets {
        message: String,
        secret_name: Option<String>,
        context: ErrorContext,
    },

    /// Error de volumen/montaje
    #[error("Volume error: {message}")]
    Volume {
        message: String,
        volume_name: Option<String>,
        context: ErrorContext,
    },

    /// Error de red
    #[error("Network error: {message}")]
    NetworkError {
        message: String,
        network_name: Option<String>,
        context: ErrorContext,
    },

    /// Error de seguridad
    #[error("Security error: {message}")]
    Security {
        message: String,
        security_context: Option<String>,
        context: ErrorContext,
    },

    /// Error de rate limiting
    #[error("Rate limit error: {message}")]
    RateLimit {
        message: String,
        limit_type: Option<String>,
        reset_time: Option<chrono::DateTime<chrono::Utc>>,
        context: ErrorContext,
    },

    /// Error específico del provider
    #[error("Provider error: {message}")]
    ProviderSpecific {
        message: String,
        provider: String,
        provider_code: Option<String>,
        context: ErrorContext,
    },
}

impl ProviderError {
    /// Crea un error de infraestructura con contexto básico
    pub fn infrastructure(message: String, provider: String) -> Self {
        Self::Infrastructure {
            message,
            code: ErrorCode::Infrastructure,
            context: ErrorContext {
                operation_id: None,
                worker_id: None,
                provider: Some(provider),
                configuration: None,
                timestamp: chrono::Utc::now(),
            },
        }
    }

    /// Crea un error de autenticación
    pub fn authentication(message: String, provider: String) -> Self {
        Self::Authentication {
            message,
            context: ErrorContext {
                operation_id: None,
                worker_id: None,
                provider: Some(provider),
                configuration: None,
                timestamp: chrono::Utc::now(),
            },
        }
    }

    /// Crea un error de autorización
    pub fn authorization(message: String, provider: String) -> Self {
        Self::Authorization {
            message,
            context: ErrorContext {
                operation_id: None,
                worker_id: None,
                provider: Some(provider),
                configuration: None,
                timestamp: chrono::Utc::now(),
            },
        }
    }

    /// Crea un error de recurso no encontrado
    pub fn not_found(
        message: String,
        resource_type: String,
        resource_id: String,
        provider: String,
    ) -> Self {
        Self::NotFound {
            message,
            resource_type,
            resource_id,
            context: ErrorContext {
                operation_id: None,
                worker_id: Some(resource_id),
                provider: Some(provider),
                configuration: None,
                timestamp: chrono::Utc::now(),
            },
        }
    }

    /// Crea un error de validación
    pub fn validation(
        message: String,
        field_errors: HashMap<String, String>,
        provider: String,
    ) -> Self {
        Self::Validation {
            message,
            field_errors,
            context: ErrorContext {
                operation_id: None,
                worker_id: None,
                provider: Some(provider),
                configuration: None,
                timestamp: chrono::Utc::now(),
            },
        }
    }

    /// Crea un error de timeout
    pub fn timeout(
        message: String,
        operation: String,
        timeout_duration: std::time::Duration,
        provider: String,
    ) -> Self {
        Self::Timeout {
            message,
            operation,
            timeout_duration,
            context: ErrorContext {
                operation_id: None,
                worker_id: None,
                provider: Some(provider),
                configuration: None,
                timestamp: chrono::Utc::now(),
            },
        }
    }

    /// Crea un error específico del provider
    pub fn provider_specific(
        message: String,
        provider: String,
        provider_code: Option<String>,
    ) -> Self {
        Self::ProviderSpecific {
            message,
            provider,
            provider_code,
            context: ErrorContext {
                operation_id: None,
                worker_id: None,
                provider: Some(provider),
                configuration: None,
                timestamp: chrono::Utc::now(),
            },
        }
    }

    /// Obtiene la categoría del error para manejo específico
    pub fn category(&self) -> ErrorCategory {
        match self {
            ProviderError::Infrastructure { .. } => ErrorCategory::PermanentInfrastructure,
            ProviderError::Authentication { .. } => ErrorCategory::Authorization,
            ProviderError::Authorization { .. } => ErrorCategory::Authorization,
            ProviderError::NotFound { .. } => ErrorCategory::ClientValidation,
            ProviderError::AlreadyExists { .. } => ErrorCategory::ClientValidation,
            ProviderError::Validation { .. } => ErrorCategory::ClientValidation,
            ProviderError::ResourceExhausted { .. } => ErrorCategory::ResourceLimits,
            ProviderError::Timeout { .. } => ErrorCategory::TransientInfrastructure,
            ProviderError::Cancelled { .. } => ErrorCategory::ClientValidation,
            ProviderError::Network { .. } => ErrorCategory::TransientInfrastructure,
            ProviderError::Configuration { .. } => ErrorCategory::Configuration,
            ProviderError::Compatibility { .. } => ErrorCategory::Configuration,
            ProviderError::Internal { .. } => ErrorCategory::Internal,
            ProviderError::Credentials { .. } => ErrorCategory::Authorization,
            ProviderError::Secrets { .. } => ErrorCategory::Authorization,
            ProviderError::Volume { .. } => ErrorCategory::Configuration,
            ProviderError::NetworkError { .. } => ErrorCategory::TransientInfrastructure,
            ProviderError::Security { .. } => ErrorCategory::Authorization,
            ProviderError::RateLimit { .. } => ErrorCategory::TransientInfrastructure,
            ProviderError::ProviderSpecific { .. } => ErrorCategory::Internal,
        }
    }

    /// Obtiene la severidad del error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            ProviderError::Infrastructure { .. } => ErrorSeverity::High,
            ProviderError::Authentication { .. } => ErrorSeverity::Medium,
            ProviderError::Authorization { .. } => ErrorSeverity::Medium,
            ProviderError::NotFound { .. } => ErrorSeverity::Low,
            ProviderError::AlreadyExists { .. } => ErrorSeverity::Low,
            ProviderError::Validation { .. } => ErrorSeverity::Low,
            ProviderError::ResourceExhausted { .. } => ErrorSeverity::High,
            ProviderError::Timeout { .. } => ErrorSeverity::Medium,
            ProviderError::Cancelled { .. } => ErrorSeverity::Low,
            ProviderError::Network { .. } => ErrorSeverity::Medium,
            ProviderError::Configuration { .. } => ErrorSeverity::High,
            ProviderError::Compatibility { .. } => ErrorSeverity::High,
            ProviderError::Internal { .. } => ErrorSeverity::Critical,
            ProviderError::Credentials { .. } => ErrorSeverity::High,
            ProviderError::Secrets { .. } => ErrorSeverity::High,
            ProviderError::Volume { .. } => ErrorSeverity::Medium,
            ProviderError::NetworkError { .. } => ErrorSeverity::Medium,
            ProviderError::Security { .. } => ErrorSeverity::High,
            ProviderError::RateLimit { .. } => ErrorSeverity::Low,
            ProviderError::ProviderSpecific { .. } => ErrorSeverity::Medium,
        }
    }

    /// Verifica si el error es transitorio y se puede reintentar
    pub fn is_transient(&self) -> bool {
        matches!(
            self.category(),
            ErrorCategory::TransientInfrastructure | ErrorCategory::ResourceLimits
        )
    }

    /// Verifica si el error es permanente y no se puede reintentar
    pub fn is_permanent(&self) -> bool {
        matches!(
            self.category(),
            ErrorCategory::ClientValidation 
            | ErrorCategory::Configuration 
            | ErrorCategory::Authorization
            | ErrorCategory::Internal
        )
    }

    /// Obtiene el worker ID asociado con el error (si existe)
    pub fn worker_id(&self) -> Option<&str> {
        match self {
            ProviderError::NotFound { resource_id, .. }
            | ProviderError::AlreadyExists { resource_id, .. } => Some(resource_id),
            _ => self.context().worker_id.as_deref(),
        }
    }

    /// Obtiene el contexto completo del error
    pub fn context(&self) -> &ErrorContext {
        match self {
            ProviderError::Infrastructure { context, .. } => context,
            ProviderError::Authentication { context, .. } => context,
            ProviderError::Authorization { context, .. } => context,
            ProviderError::NotFound { context, .. } => context,
            ProviderError::AlreadyExists { context, .. } => context,
            ProviderError::Validation { context, .. } => context,
            ProviderError::ResourceExhausted { context, .. } => context,
            ProviderError::Timeout { context, .. } => context,
            ProviderError::Cancelled { context, .. } => context,
            ProviderError::Network { context, .. } => context,
            ProviderError::Configuration { context, .. } => context,
            ProviderError::Compatibility { context, .. } => context,
            ProviderError::Internal { context, .. } => context,
            ProviderError::Credentials { context, .. } => context,
            ProviderError::Secrets { context, .. } => context,
            ProviderError::Volume { context, .. } => context,
            ProviderError::NetworkError { context, .. } => context,
            ProviderError::Security { context, .. } => context,
            ProviderError::RateLimit { context, .. } => context,
            ProviderError::ProviderSpecific { context, .. } => context,
        }
    }

    /// Añade información adicional al contexto del error
    pub fn with_context(mut self, operation_id: String) -> Self {
        match &mut self {
            ProviderError::Infrastructure { context, .. }
            | ProviderError::Authentication { context, .. }
            | ProviderError::Authorization { context, .. }
            | ProviderError::NotFound { context, .. }
            | ProviderError::AlreadyExists { context, .. }
            | ProviderError::Validation { context, .. }
            | ProviderError::ResourceExhausted { context, .. }
            | ProviderError::Timeout { context, .. }
            | ProviderError::Cancelled { context, .. }
            | ProviderError::Network { context, .. }
            | ProviderError::Configuration { context, .. }
            | ProviderError::Compatibility { context, .. }
            | ProviderError::Internal { context, .. }
            | ProviderError::Credentials { context, .. }
            | ProviderError::Secrets { context, .. }
            | ProviderError::Volume { context, .. }
            | ProviderError::NetworkError { context, .. }
            | ProviderError::Security { context, .. }
            | ProviderError::RateLimit { context, .. }
            | ProviderError::ProviderSpecific { context, .. } => {
                context.operation_id = Some(operation_id);
            }
        }
        self
    }
}

/// Errores específicos de gestión de credenciales
#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum CredentialError {
    #[error("Credential not found: {name}")]
    NotFound { name: String },

    #[error("Credential expired: {name}")]
    Expired { name: String },

    #[error("Credential rotation failed: {name}")]
    RotationFailed { name: String, source: ProviderError },

    #[error("Invalid credential format: {name}")]
    InvalidFormat { name: String },

    #[error("Permission denied for credential: {name}")]
    PermissionDenied { name: String },

    #[error("Credential limit exceeded: {name}")]
    LimitExceeded { name: String },

    #[error("Credential sync failed: {source}")]
    SyncFailed { source: ProviderError },

    #[error("Credential validation failed: {name}")]
    ValidationFailed { name: String, errors: Vec<String> },
}

/// Errores específicos de gestión de secretos
#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum SecretError {
    #[error("Secret not found: {name}")]
    NotFound { name: String },

    #[error("Secret access denied: {name}")]
    AccessDenied { name: String },

    #[error("Secret version not found: {name} version {version}")]
    VersionNotFound { name: String, version: String },

    #[error("Secret rotation failed: {name}")]
    RotationFailed { name: String },

    #[error("Invalid secret format: {name}")]
    InvalidFormat { name: String },

    #[error("Secret size limit exceeded: {name}")]
    SizeLimitExceeded { name: String },

    #[error("Secret encryption failed: {name}")]
    EncryptionFailed { name: String },

    #[error("Secret decryption failed: {name}")]
    DecryptionFailed { name: String },
}

/// Errores específicos de auto-scaling
#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum AutoScalingError {
    #[error("Scaling policy not found: {name}")]
    PolicyNotFound { name: String },

    #[error("Scaling metric invalid: {metric}")]
    InvalidMetric { metric: String },

    #[error("Scaling threshold exceeded: {metric} = {value} > {threshold}")]
    ThresholdExceeded { metric: String, value: f64, threshold: f64 },

    #[error("Scaling cooldown period active")]
    CooldownActive,

    #[error("Scaling min limit reached: {current} >= {minimum}")]
    MinLimitReached { current: u32, minimum: u32 },

    #[error("Scaling max limit reached: {current} <= {maximum}")]
    MaxLimitReached { current: u32, maximum: u32 },

    #[error("Scaling provider error: {source}")]
    ProviderError { source: ProviderError },
}

/// Errores específicos de health checks
#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum HealthCheckError {
    #[error("Health check failed: {worker_id}")]
    Failed { worker_id: String },

    #[error("Health check timeout: {worker_id}")]
    Timeout { worker_id: String },

    #[error("Health check endpoint not found: {worker_id}")]
    EndpointNotFound { worker_id: String },

    #[error("Health check configuration invalid")]
    InvalidConfiguration,

    #[error("Health check provider error: {source}")]
    ProviderError { source: ProviderError },
}

/// Implementación de From para errores comunes de proveedores
impl From<reqwest::Error> for ProviderError {
    fn from(error: reqwest::Error) -> Self {
        let message = format!("HTTP client error: {}", error);
        if error.is_timeout() {
            ProviderError::timeout(message, "http_client".to_string(), std::time::Duration::from_secs(30), "generic".to_string())
        } else if error.is_connect() {
            ProviderError::network(message, None, None, ErrorContext {
                operation_id: None,
                worker_id: None,
                provider: Some("generic".to_string()),
                configuration: None,
                timestamp: chrono::Utc::now(),
            })
        } else {
            ProviderError::infrastructure(message, "generic".to_string())
        }
    }
}

impl From<serde_json::Error> for ProviderError {
    fn from(error: serde_json::Error) -> Self {
        ProviderError::internal(
            format!("JSON serialization error: {}", error),
            Some(error.to_string()),
            ErrorContext {
                operation_id: None,
                worker_id: None,
                provider: Some("generic".to_string()),
                configuration: None,
                timestamp: chrono::Utc::now(),
            }
        )
    }
}

impl From<tokio::time::error::Elapsed> for ProviderError {
    fn from(error: tokio::time::error::Elapsed) -> Self {
        ProviderError::timeout(
            "Operation timed out".to_string(),
            "async_operation".to_string(),
            std::time::Duration::from_secs(30),
            "generic".to_string(),
        )
    }
}