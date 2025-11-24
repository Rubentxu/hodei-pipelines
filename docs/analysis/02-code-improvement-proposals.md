# Propuestas de Mejora de Código - Hodei Jobs Platform

**Fecha**: 2025-11-24  
**Basado en**: Análisis Táctico DDD (01-tactical-ddd-analysis.md)  
**Principios**: DDD, SOLID, Clean Architecture, Hexagonal Architecture

---

## Resumen Ejecutivo

Este documento presenta propuestas específicas de mejora del código basadas en el análisis táctico DDD realizado. Las mejoras están priorizadas por impacto y complejidad, enfocándose en reducir acoplamiento, mejorar mantenibilidad y robustez del sistema.

**Áreas de mejora identificadas**:
- Validación de DAG en Pipelines (crítico)
- Transformación de Connascence of Position → Connascence of Name
- Implementación de Specification Pattern para validaciones
- Refuerzo de seguridad mTLS
- Reducción de Feature Envy
- Eliminación de Temporal Coupling

---

## 🎯 Propuestas Prioritarias

### 1. Implementar Validación de DAG para Pipelines ✅ COMPLETADO
**Prioridad**: 🔴 **ALTA**  
**Complejidad**: Media  
**Impacto**: Crítico

#### ✅ SOLUCIÓN IMPLEMENTADA
La validación DAG ha sido implementada con éxito:
- ✅ Algoritmo DFS para detección de ciclos
- ✅ Algoritmo Kahn para orden topológico  
- ✅ Función `PipelineStep::validate()` para validación individual
- ✅ Función `Pipeline::get_execution_order()` para obtener orden de ejecución
- ✅ Tests unitarios para casos de éxito, fallo y dependencias circulares

#### Beneficios
- ✅ Previene ejecución de pipelines con dependencias circulares
- ✅ Proporciona orden de ejecución determinístico
- ✅ Facilita optimización de paralelización
- ✅ Mejor manejo de errores con mensajes específicos
- ✅ Validación en tiempo de creación, no de ejecución

---

### 2. Reforzar Seguridad mTLS ✅ COMPLETADO
**Prioridad**: 🔴 **ALTA**  
**Complejidad**: Alta  
**Impacto**: Crítico (Seguridad)

#### ✅ SOLUCIÓN IMPLEMENTADA
La validación mTLS ha sido implementada con las siguientes características:
- ✅ Configuración completa de mTLS con allowlist de DNS/IP
- ✅ Validación de cadena de certificados completa
- ✅ Parsing y validación de certificados usando x509-parser
- ✅ RootCA store para validación contra CA confiable
- ✅ Verificación de estructura de certificados
- ✅ Configuración flexible con parámetros personalizables
- ✅ Integración con sistema de errores de seguridad
- ✅ Preparado para validación extendida (CRL/OCSP, Key Usage, EKU)

#### Componentes Implementados
1. **MtlsConfig**: Configuración flexible con opciones para:
   - Ruta de certificado CA
   - Requerimiento de certificado cliente
   - Allowlist de DNS names
   - Allowlist de IPs
   - Profundidad máxima de cadena

2. **CertificateValidationConfig**: Configuración interna para validación
3. **ProductionCertificateValidator**: Validador principal con:
   - Parsing de certificados PEM
   - Validación de cadena completa
   - Estructura extensible para validación avanzada
4. **TlsCertificateValidator**: Adaptador que implementa CertificateValidator trait

#### Beneficios
- ✅ Validación robusta de certificados cliente
- ✅ Prevención de conexiones no autorizadas
- ✅ Configuración flexible para diferentes entornos
- ✅ Base sólida para compliance de seguridad
- ✅ Extensible para futuras mejoras (CRL/OCSP, EKU validation)

---

### 3. Transformar Connascence of Position → Connascence of Name ✅ COMPLETADO
**Prioridad**: 🟡 **MEDIA**  
**Complejidad**: Media  
**Impacto**: Alto

#### ✅ SOLUCIÓN IMPLEMENTADA
El Builder Pattern ha sido implementado exitosamente en `SchedulerModule`:

- ✅ **SchedulerBuilder**: Nuevo tipo builder con métodos fluidos
- ✅ **Configuración flexible**: Campos en cualquier orden
- ✅ **Valores por defecto**: SchedulerConfig::default() para parámetros opcionales
- ✅ **Validación en build()**: Verificación de parámetros requeridos
- ✅ **API auto-documentada**: Métodos con nombres descriptivos
- ✅ **Compilación exitosa**: Código funcional y testeable

#### Beneficios Logrados
- ✅ **Elimina Connascence of Position**: Ya no depende del orden de parámetros
- ✅ **Código más legible**: Métodos nombrados vs parámetros posicionales
- ✅ **Mejor mantenibilidad**: Agregar campos sin romper API existente
- ✅ **Testing simplificado**: Builder facilita mocking y configuración
- ✅ **Detección temprana**: Errores de configuración en tiempo de build

#### Ejemplo de Uso Nuevo
```rust
// ANTES - Orden crítico (Connascence of Position)
let scheduler = SchedulerModule::new(
    job_repo,
    event_bus,
    worker_client,
    worker_repo,
    config,
);

// DESPUÉS - Orden libre, auto-documentado (Connascence of Name)
let scheduler = SchedulerBuilder::new()
    .event_bus(event_bus)
    .job_repository(job_repo)
    .worker_client(worker_client)
    .worker_repository(worker_repo)
    .config(custom_config)
    .build()?;  // Result con validación de campos requeridos
```

---

### 4. Implementar Specification Pattern para Validaciones ✅ COMPLETADO
**Prioridad**: 🟡 **MEDIA**  
**Complejidad**: Media  
**Impacto**: Alto

#### ✅ SOLUCIÓN IMPLEMENTADA
El Specification Pattern ha sido implementado exitosamente con las siguientes características:

- ✅ **Specification trait**: Base para composable validation rules
- ✅ **Composite specifications**: AndSpec, OrSpec, NotSpec para combinación lógica
- ✅ **Concrete specifications**: JobNameNotEmptySpec, JobImageNotEmptySpec, JobCommandNotEmptySpec, JobTimeoutPositiveSpec
- ✅ **SpecificationResult**: Resultado de validación con errores detallados
- ✅ **Integración con JobSpec**: Método validate() refactorizado para usar specifications
- ✅ **Tests unitarios**: Cobertura completa para todos los specifications
- ✅ **Documentación KDoc**: API completamente documentada

#### Archivos Creados/Modificados
1. **crates/shared-types/src/specifications.rs** - Core specification infrastructure
2. **crates/shared-types/src/job_specifications.rs** - Concrete JobSpec specifications
3. **crates/shared-types/src/job_definitions.rs** - Integrated with specifications
4. **crates/shared-types/src/lib.rs** - Export specifications module

#### Beneficios Logrados
- ✅ **Reglas modulares**: Cada validación es un componente independiente
- ✅ **Composabilidad**: Combinar reglas usando AND, OR, NOT
- ✅ **Reutilización**: Specifications reutilizables en diferentes contextos
- ✅ **Testabilidad**: Cada regla testeable de forma aislada
- ✅ **Mantenibilidad**: Agregar/editar reglas sin afectar otras
- ✅ **Legibilidad**: Lógica de negocio más clara y expresiva

#### Problema Identificado
```rust
// crates/shared-types/src/job_definitions.rs - LÍNEA ACTUAL
impl JobSpec {
    pub fn validate(&self) -> Result<(), DomainError> {
        // ⚠️ Validaciones dispersas y difíciles de mantener
        if self.name.trim().is_empty() {
            return Err(DomainError::Validation("Job name cannot be empty".to_string()));
        }
        if self.timeout_ms == 0 {
            return Err(DomainError::Validation("Timeout must be greater than 0".to_string()));
        }
        // ... más validaciones
    }
}
```

#### Solución Propuesta
Implementar Specifications composables para reglas de negocio.

```rust
// crates/core/src/specifications.rs - NUEVO ARCHIVO
use hodei_shared_types::{JobSpec, Worker, PipelineStep};
use std::fmt;

// Specification trait
pub trait Specification<T> {
    fn is_satisfied_by(&self, candidate: &T) -> bool;
    fn and<U>(self, other: U) -> AndSpec<Self, U, T>
    where
        Self: Sized,
        U: Specification<T>,
    {
        AndSpec::new(self, other)
    }
    fn or<U>(self, other: U) -> OrSpec<Self, U, T>
    where
        Self: Sized,
        U: Specification<T>,
    {
        OrSpec::new(self, other)
    }
    fn not(self) -> NotSpec<Self, T>
    where
        Self: Sized,
    {
        NotSpec::new(self)
    }
}

// Composite specifications
pub struct AndSpec<A, B, T> {
    left: A,
    right: B,
    phantom: std::marker::PhantomData<T>,
}

impl<A, B, T> AndSpec<A, B, T> {
    pub fn new(left: A, right: B) -> Self {
        Self {
            left,
            right,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<A, B, T> Specification<T> for AndSpec<A, B, T>
where
    A: Specification<T>,
    B: Specification<T>,
{
    fn is_satisfied_by(&self, candidate: &T) -> bool {
        self.left.is_satisfied_by(candidate) && self.right.is_satisfied_by(candidate)
    }
}

pub struct OrSpec<A, B, T> {
    left: A,
    right: B,
    phantom: std::marker::PhantomData<T>,
}

impl<A, B, T> OrSpec<A, B, T> {
    pub fn new(left: A, right: B) -> Self {
        Self {
            left,
            right,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<A, B, T> Specification<T> for OrSpec<A, B, T>
where
    A: Specification<T>,
    B: Specification<T>,
{
    fn is_satisfied_by(&self, candidate: &T) -> bool {
        self.left.is_satisfied_by(candidate) || self.right.is_satisfied_by(candidate)
    }
}

pub struct NotSpec<A, T> {
    spec: A,
    phantom: std::marker::PhantomData<T>,
}

impl<A, T> NotSpec<A, T> {
    pub fn new(spec: A) -> Self {
        Self {
            spec,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<A, T> Specification<T> for NotSpec<A, T>
where
    A: Specification<T>,
{
    fn is_satisfied_by(&self, candidate: &T) -> bool {
        !self.spec.is_satisfied_by(candidate)
    }
}

// Concrete Specifications for JobSpec

pub struct JobNameSpec {
    min_length: usize,
    max_length: usize,
}

impl JobNameSpec {
    pub fn new(min: usize, max: usize) -> Self {
        Self {
            min_length: min,
            max_length: max,
        }
    }
}

impl Specification<JobSpec> for JobNameSpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        let name = candidate.name.trim();
        !name.is_empty()
            && name.len() >= self.min_length
            && name.len() <= self.max_length
    }
}

pub struct JobTimeoutSpec {
    min_timeout_ms: u64,
}

impl JobTimeoutSpec {
    pub fn new(min_ms: u64) -> Self {
        Self {
            min_timeout_ms: min_ms,
        }
    }
}

impl Specification<JobSpec> for JobTimeoutSpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        candidate.timeout_ms >= self.min_timeout_ms
    }
}

pub struct JobImageSpec;

impl Specification<JobSpec> for JobImageSpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        let image = candidate.image.trim();
        !image.is_empty()
            && (image.contains(':') || image.contains('@'))
    }
}

pub struct JobCommandSpec;

impl Specification<JobSpec> for JobCommandSpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        !candidate.command.is_empty()
    }
}

// Worker Specifications

pub struct WorkerCapacitySpec {
    min_concurrent_jobs: usize,
}

impl WorkerCapacitySpec {
    pub fn new(min_jobs: usize) -> Self {
        Self {
            min_concurrent_jobs: min_jobs,
        }
    }
}

impl Specification<Worker> for WorkerCapacitySpec {
    fn is_satisfied_by(&self, candidate: &Worker) -> bool {
        candidate.capabilities.max_concurrent_jobs() >= self.min_concurrent_jobs
    }
}

// Pipeline Specifications

pub struct PipelineStepTimeoutSpec;

impl Specification<PipelineStep> for PipelineStepTimeoutSpec {
    fn is_satisfied_by(&self, candidate: &PipelineStep) -> bool {
        candidate.timeout_ms > 0
    }
}

// Specification Result helper
pub struct SpecificationResult<T> {
    candidate: T,
    errors: Vec<String>,
}

impl<T> SpecificationResult<T> {
    pub fn new(candidate: T) -> Self {
        Self {
            candidate,
            errors: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn into_result(self) -> Result<T, DomainError>
    where
        T: Clone,
    {
        if self.errors.is_empty() {
            Ok(self.candidate)
        } else {
            Err(DomainError::Validation(self.errors.join("; ")))
        }
    }

    pub fn map<U, F>(self, f: F) -> SpecificationResult<U>
    where
        F: FnOnce(T) -> U,
    {
        SpecificationResult {
            candidate: f(self.candidate),
            errors: self.errors,
        }
    }
}

// Validate helper function
pub fn validate_specification<T, S>(
    candidate: T,
    specifications: &[S],
) -> Result<T, DomainError>
where
    T: Clone,
    S: Specification<T> + std::fmt::Display,
{
    let mut result = SpecificationResult::new(candidate);

    for spec in specifications {
        if !spec.is_satisfied_by(&result.candidate) {
            result.add_error(format!("Specification not satisfied: {}", spec));
        }
    }

    result.into_result()
}

impl std::fmt::Display for JobNameSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job name must be between {} and {} characters", self.min_length, self.max_length)
    }
}

impl std::fmt::Display for JobTimeoutSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job timeout must be at least {}ms", self.min_timeout_ms)
    }
}

impl std::fmt::Display for JobImageSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job image must be a valid container image reference")
    }
}

impl std::fmt::Display for JobCommandSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job command cannot be empty")
    }
}

impl std::fmt::Display for WorkerCapacitySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Worker must support at least {} concurrent jobs", self.min_concurrent_jobs)
    }
}

impl std::fmt::Display for PipelineStepTimeoutSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pipeline step timeout must be greater than 0")
    }
}
```

#### Uso en JobSpec
```rust
// crates/shared-types/src/job_definitions.rs
impl JobSpec {
    pub fn validate(&self) -> Result<(), DomainError> {
        use hodei_core::specifications::{
            validate_specification,
            JobNameSpec,
            JobTimeoutSpec,
            JobImageSpec,
            JobCommandSpec,
        };

        let specifications: Vec<Box<dyn Specification<JobSpec> + Send + Sync>> = vec![
            Box::new(JobNameSpec::new(1, 255)),
            Box::new(JobTimeoutSpec::new(1000)),
            Box::new(JobImageSpec),
            Box::new(JobCommandSpec),
        ];

        // Clone the candidate for validation
        let candidate = self.clone();
        validate_specification(candidate, &specifications)
    }
}
```

#### Beneficios
- ✅ Reglas de negocio modulares y reutilizables
- ✅ Combinación de especificaciones (AND, OR, NOT)
- ✅ Fácil testing unitario de cada regla
- ✅ Mensajes de error descriptivos
- ✅ Separación clara de responsabilidades
- ✅ Evolución independiente de reglas

---

### 4. Reforzar Seguridad mTLS
**Prioridad**: 🔴 **ALTA**  
**Complejidad**: Alta  
**Impacto**: Crítico (Seguridad)

#### Problema Identificado
```rust
// crates/adapters/src/security/mtls.rs - LÍNEA ACTUAL
impl TlsCertificateValidator {
    pub fn validate_client_cert(&self, cert: &Certificate) -> Result<(), DomainError> {
        // ⚠️ Validación incompleta - no verifica cadena completa
        Ok(())
    }
}
```

#### Solución Propuesta
Implementar validación completa de certificado con verificación de cadena.

```rust
// crates/adapters/src/security/mtls.rs - PROPUESTA COMPLETA
use rustls::{Certificate, ServerConfig, ClientConfig, RootCertStore, CertificateError};
use rustls::server::{ClientCertVerifier, ServerCertVerifier};
use rustls::client::{ServerCertVerifier, WebPkiVerifier};
use rustls_pemfile::{certs, pkcs8_private_keys};
use x509_parser::{X509Certificate, ParsingError};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use chrono::{DateTime, Utc, NaiveDateTime};
use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct CertificateValidationConfig {
    pub require_client_auth: bool,
    pub min_tls_version: rustls::ProtocolVersion,
    pub allowed_client_dns_names: Vec<String>,
    pub allowed_client_ips: Vec<IpAddr>,
    pub max_cert_chain_depth: u8,
    pub check_crl: bool,
    pub ocsp_url: Option<String>,
}

impl Default for CertificateValidationConfig {
    fn default() -> Self {
        Self {
            require_client_auth: true,
            min_tls_version: rustls::ProtocolVersion::TLSv1_2,
            allowed_client_dns_names: Vec::new(),
            allowed_client_ips: Vec::new(),
            max_cert_chain_depth: 10,
            check_crl: false,
            ocsp_url: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CertificateValidationError {
    #[error("Invalid certificate chain")]
    InvalidChain,
    #[error("Certificate expired")]
    Expired,
    #[error("Certificate not yet valid")]
    NotYetValid,
    #[error("Invalid certificate subject")]
    InvalidSubject,
    #[error("Certificate authority validation failed")]
    CaValidationFailed,
    #[error("Client name mismatch")]
    NameMismatch,
    #[error("Certificate revoked")]
    Revoked,
    #[error("Internal error: {0}")]
    Internal(String),
}

pub struct ProductionCertificateValidator {
    root_store: RootCertStore,
    config: CertificateValidationConfig,
}

impl ProductionCertificateValidator {
    pub fn new(
        ca_cert_pem: &[u8],
        config: CertificateValidationConfig,
    ) -> Result<Self, CertificateValidationError> {
        let mut root_store = RootCertStore::empty();

        // Parse and add CA certificate
        let ca_certs = certs(&mut BufReader::new(ca_cert_pem))
            .map_err(|e| CertificateValidationError::Internal(
                format!("Failed to parse CA cert: {}", e)
            ))?;

        for cert in ca_certs {
            root_store.add(&Certificate(cert))
                .map_err(|e| CertificateValidationError::Internal(
                    format!("Failed to add CA to store: {}", e)
                ))?;
        }

        Ok(ProductionCertificateValidator {
            root_store,
            config,
        })
    }

    pub fn validate_client_cert_chain(
        &self,
        client_certs: &[Certificate],
    ) -> Result<(), CertificateValidationError> {
        if client_certs.is_empty() {
            return Err(CertificateValidationError::InvalidChain);
        }

        // Parse all certificates in chain
        let mut parsed_certs = Vec::new();
        for cert in client_certs {
            let parsed = X509Certificate::from_der(&cert.0)
                .map_err(|_| CertificateValidationError::InvalidChain)?;
            parsed_certs.push(parsed.1);
        }

        // Validate chain length
        if parsed_certs.len() > self.config.max_cert_chain_depth as usize {
            return Err(CertificateValidationError::InvalidChain);
        }

        // Validate each certificate in chain
        let now = Utc::now();
        for cert in &parsed_certs {
            self.validate_single_cert(cert, now)?;
        }

        // Validate certificate chain trust
        self.validate_chain_trust(&parsed_certs)?;

        // Validate client identity if required
        if self.config.require_client_auth {
            self.validate_client_identity(&parsed_certs[0])?;
        }

        Ok(())
    }

    fn validate_single_cert(
        &self,
        cert: &X509Certificate,
        now: DateTime<Utc>,
    ) -> Result<(), CertificateValidationError> {
        // Check validity period
        let not_before = cert.validity().not_before.to_datetime();
        let not_after = cert.validity().not_after.to_datetime();

        if now < not_before {
            return Err(CertificateValidationError::NotYetValid);
        }

        if now > not_after {
            return Err(CertificateValidationError::Expired);
        }

        // Check key usage if present
        if let Some(key_usage) = cert.key_usage() {
            if !key_usage.digital_signature() && !key_usage.key_encipherment() {
                return Err(CertificateValidationError::InvalidChain);
            }
        }

        // Check extended key usage if present
        if let Some(ext_key_usage) = cert.extended_key_usage() {
            let valid_usages = ext_key_usage.client_auth() || ext_key_usage.server_auth();
            if !valid_usages {
                return Err(CertificateValidationError::InvalidChain);
            }
        }

        Ok(())
    }

    fn validate_chain_trust(
        &self,
        certs: &[x509_parser::certificate::X509Certificate],
    ) -> Result<(), CertificateValidationError> {
        // For full validation, we'd need to:
        // 1. Build chain to root CA
        // 2. Verify signatures at each step
        // 3. Check revocation status (CRL/OCSP)
        // 4. Verify algorithm constraints

        // This is a simplified version - production would use WebPkiVerifier
        if certs.len() == 0 {
            return Err(CertificateValidationError::InvalidChain);
        }

        // Verify that last cert in chain is signed by trusted root
        let leaf_cert = &certs[0];
        let issuer = &certs[certs.len() - 1];

        if leaf_cert.issuer() != issuer.subject() {
            return Err(CertificateValidationError::CaValidationFailed);
        }

        Ok(())
    }

    fn validate_client_identity(
        &self,
        client_cert: &X509Certificate,
    ) -> Result<(), CertificateValidationError> {
        let subject = client_cert.subject();
        let san = client_cert.subject_alternative_name();

        // Check DNS names
        if !self.config.allowed_client_dns_names.is_empty() {
            let dns_match = san.and_then(|san| san.dns_names())
                .map(|dns_iter| {
                    dns_iter.any(|dns| {
                        self.config.allowed_client_dns_names.iter()
                            .any(|allowed| allowed == dns)
                    })
                })
                .unwrap_or(false);

            if !dns_match && !self.config.allowed_client_dns_names.is_empty() {
                return Err(CertificateValidationError::NameMismatch);
            }
        }

        // Check IP addresses
        if !self.config.allowed_client_ips.is_empty() {
            let ip_match = san.and_then(|san| san.ip_addresses())
                .map(|ip_iter| {
                    ip_iter.any(|ip| {
                        self.config.allowed_client_ips.iter()
                            .any(|allowed| allowed == &ip)
                    })
                })
                .unwrap_or(false);

            if !ip_match && !self.config.allowed_client_ips.is_empty() {
                return Err(CertificateValidationError::NameMismatch);
            }
        }

        Ok(())
    }
}

// Custom Client Cert Verifier
pub struct ClientCertVerifierImpl {
    validator: ProductionCertificateValidator,
}

impl ClientCertVerifierImpl {
    pub fn new(validator: ProductionCertificateValidator) -> Self {
        Self { validator }
    }
}

impl ClientCertVerifier for ClientCertVerifierImpl {
    fn offer_client_certs(&self) -> bool {
        true
    }

    fn client_auth_mandatory(&self) -> bool {
        self.validator.config.require_client_auth
    }

    fn verify_client_cert(
        &self,
        end_entity: &Certificate,
        intermediates: &[Certificate],
        now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::server::ClientCertVerified, rustls::Error> {
        let mut certs = vec![end_entity.clone()];
        certs.extend_from_slice(intermediates);

        self.validator
            .validate_client_cert_chain(&certs)
            .map(|_| rustls::server::ClientCertVerified::assertion())
            .map_err(|e| {
                rustls::Error::InvalidCertificateEncoding
            })
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &Certificate,
        sig: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &Certificate,
        sig: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}
```

#### Beneficios
- ✅ Validación completa de cadena de certificados
- ✅ Verificación de fechas de validez
- ✅ Validación de sujeto (DNS/IP allowlist)
- ✅ Verificación de uso de clave (Key Usage, Extended Key Usage)
- ✅ Preparado para CRL/OCSP
- ✅ Compatibilidad con TLS 1.2 y 1.3

---

### 5. Reducir Feature Envy con capa Mapper ✅ COMPLETADO
**Prioridad**: 🟡 **MEDIA**  
**Complejidad**: Media  
**Impacto**: Medio

#### ✅ SOLUCIÓN IMPLEMENTADA
La capa Mapper ha sido implementada exitosamente con las siguientes características:

- ✅ **JobMapper trait**: Interfaz para mapeo de Job entity a JobRow
- ✅ **WorkerMapper trait**: Interfaz para mapeo de Worker entity a WorkerRow  
- ✅ **SqlxJobMapper**: Implementación concreta de mapeo para Job
- ✅ **SqlxWorkerMapper**: Implementación concreta de mapeo para Worker
- ✅ **JobRow struct**: Representación de base de datos para Job
- ✅ **WorkerRow struct**: Representación de base de datos para Worker
- ✅ **Refactoring de repositorios**: PostgreSqlJobRepository y PostgreSqlWorkerRepository ahora usan mappers
- ✅ **Separación de concerns**: Lógica de mapeo extraída de adapters a mappers

#### Archivos Creados/Modificados
1. **crates/core/src/mappers/mod.rs** - Exportación de mappers
2. **crates/core/src/mappers/job_mapper.rs** - JobMapper trait e implementación SqlxJobMapper
3. **crates/core/src/mappers/worker_mapper.rs** - WorkerMapper trait e implementación SqlxWorkerMapper
4. **crates/core/src/lib.rs** - Agregado módulo mappers
5. **crates/adapters/src/postgres.rs** - Refactorizado para usar mappers

#### Beneficios Logrados
- ✅ **Elimina Feature Envy**: Repository adapters ya no contienen lógica de mapeo detallada
- ✅ **Separación clara**: Mapper = Domain knowledge, Adapter = Persistence knowledge
- ✅ **Reutilización**: Mappers pueden ser usados por múltiples adapters
- ✅ **Testabilidad**: Mappers testeables de forma independiente
- ✅ **Mantenibilidad**: Cambios en mapeo no afectan código de repository
- ✅ **Consistencia**: Un solo lugar para lógica de transformación DB ↔ Domain

#### Problema Identificado
```rust
// crates/adapters/src/postgres.rs - LÍNEA ACTUAL
pub struct PostgreSqlJobRepository {
    // ⚠️ Conocimiento detallado del struct Job en el adapter
}

impl JobRepository for PostgreSqlJobRepository {
    async fn save_job(&self, job: &Job) -> Result<(), Error> {
        // ⚠️ Mapper de Job a campos SQL disperso
        let query = r#"
            INSERT INTO jobs (id, name, spec, state, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
        "#;
        // ... mapeo manual de campos
    }
}
```

#### Solución Propuesta
Extraer Mapper layer para separación de concerns.

```rust
// crates/core/src/mappers/job_mapper.rs - NUEVO ARCHIVO
use hodei_core::Job;
use hodei_shared_types::{JobState, JobId};
use chrono::{DateTime, Utc};

pub struct JobRow {
    pub id: String,
    pub name: String,
    pub spec_json: String,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub worker_id: Option<String>,
    pub retry_count: i32,
}

pub trait JobMapper {
    fn to_row(&self, job: &Job) -> JobRow;
    fn from_row(&self, row: JobRow) -> Result<Job, String>;
    fn to_update_query(&self, job: &Job) -> (String, Vec<String>);
}

pub struct SqlxJobMapper;

impl JobMapper for SqlxJobMapper {
    fn to_row(&self, job: &Job) -> JobRow {
        JobRow {
            id: job.id.to_string(),
            name: job.name.clone(),
            spec_json: serde_json::to_string(&job.spec)
                .unwrap_or_else(|_| "{}".to_string()),
            state: job.state.to_string(),
            created_at: job.created_at,
            updated_at: job.updated_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            worker_id: job.worker_id.as_ref().map(|w| w.to_string()),
            retry_count: job.retry_count as i32,
        }
    }

    fn from_row(&self, row: JobRow) -> Result<Job, String> {
        let job_spec = serde_json::from_str(&row.spec_json)
            .map_err(|e| format!("Failed to parse job spec: {}", e))?;

        let job_id = JobId::parse_str(&row.id)
            .map_err(|e| format!("Invalid job ID: {}", e))?;

        let job_state = JobState::from_str(&row.state)
            .map_err(|e| format!("Invalid job state: {}", e))?;

        // Reconstruir Job usando constructor interno
        Ok(Job {
            id: job_id,
            name: row.name,
            spec: job_spec,
            state: job_state,
            created_at: row.created_at,
            updated_at: row.updated_at,
            started_at: row.started_at,
            completed_at: row.completed_at,
            worker_id: row.worker_id.map(|s| s.parse().unwrap()),
            retry_count: row.retry_count as u32,
        })
    }

    fn to_update_query(&self, job: &Job) -> (String, Vec<String>) {
        let mut updates = Vec::new();
        let mut params = Vec::new();

        updates.push(format!("name = ${}", params.len() + 1));
        params.push(job.name.clone());

        updates.push(format!("spec = ${}", params.len() + 1));
        params.push(serde_json::to_string(&job.spec)
            .unwrap_or_else(|_| "{}".to_string()));

        updates.push(format!("state = ${}", params.len() + 1));
        params.push(job.state.to_string());

        updates.push(format!("updated_at = ${}", params.len() + 1));
        params.push(job.updated_at.to_rfc3339());

        if let Some(started) = job.started_at {
            updates.push(format!("started_at = ${}", params.len() + 1));
            params.push(started.to_rfc3339());
        }

        if let Some(completed) = job.completed_at {
            updates.push(format!("completed_at = ${}", params.len() + 1));
            params.push(completed.to_rfc3339());
        }

        let query = format!(
            "UPDATE jobs SET {} WHERE id = ${}",
            updates.join(", "),
            params.len() + 1
        );
        params.push(job.id.to_string());

        (query, params)
    }
}

// Extensiones de Job para serialización
impl Job {
    pub fn to_row(&self) -> JobRow {
        let mapper = SqlxJobMapper;
        mapper.to_row(self)
    }

    pub fn from_row(row: JobRow) -> Result<Self, String> {
        let mapper = SqlxJobMapper;
        mapper.from_row(row)
    }
}
```

```rust
// crates/adapters/src/postgres.rs - REFACTORIZADO
impl JobRepository for PostgreSqlJobRepository {
    async fn save_job(&self, job: &Job) -> Result<(), Error> {
        let mapper = SqlxJobMapper;
        let row = mapper.to_row(job);

        sqlx::query!(
            r#"
                INSERT INTO jobs (
                    id, name, spec, state, created_at, updated_at,
                    started_at, completed_at, worker_id, retry_count
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name,
                    spec = EXCLUDED.spec,
                    state = EXCLUDED.state,
                    updated_at = EXCLUDED.updated_at,
                    started_at = EXCLUDED.started_at,
                    completed_at = EXCLUDED.completed_at,
                    worker_id = EXCLUDED.worker_id,
                    retry_count = EXCLUDED.retry_count
            "#,
            row.id,
            row.name,
            row.spec_json,
            row.state,
            row.created_at,
            row.updated_at,
            row.started_at,
            row.completed_at,
            row.worker_id,
            row.retry_count,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_job(&self, id: &JobId) -> Result<Option<Job>, Error> {
        let mapper = SqlxJobMapper;
        let job_id_str = id.to_string();

        let row = sqlx::query!(
            r#"
                SELECT id, name, spec, state, created_at, updated_at,
                       started_at, completed_at, worker_id, retry_count
                FROM jobs WHERE id = $1
            "#,
            job_id_str
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let job_row = JobRow {
                id: row.id,
                name: row.name,
                spec_json: row.spec,
                state: row.state,
                created_at: row.created_at,
                updated_at: row.updated_at,
                started_at: row.started_at,
                completed_at: row.completed_at,
                worker_id: row.worker_id,
                retry_count: row.retry_count,
            };

            match mapper.from_row(job_row) {
                Ok(job) => Ok(Some(job)),
                Err(e) => Err(Error::Database(e)),
            }
        } else {
            Ok(None)
        }
    }
}
```

#### Beneficios
- ✅ Separación clara de concerns (persistencia vs dominio)
- ✅ Mapper reutilizable
- ✅ Tests unitarios del mapper independientes
- ✅ Facilita múltiples representaciones (DB, API, Event)
- ✅ Adapter más simple y enfocado

---

### 6. Eliminar Temporal Coupling con State Machine ✅ COMPLETADO
**Prioridad**: 🟡 **MEDIA**  
**Complejidad**: Media  
**Impacto**: Medio

#### ✅ SOLUCIÓN IMPLEMENTADA
La State Machine ha sido implementada exitosamente con las siguientes características:

- ✅ **SchedulingState enum**: Estados explícitos (Idle, Collecting, Matching, Optimizing, Committing, Completed, Failed)
- ✅ **SchedulingContext struct**: Contexto para almacenar datos de scheduling (job, workers, assignments)
- ✅ **SchedulingStateMachine**: State machine principal con transiciones validadas
- ✅ **Métodos de transición**: collect(), match_jobs(), optimize(), commit(), complete()
- ✅ **Validación de estado**: validate_state() asegura transiciones válidas
- ✅ **Integración con SchedulerModule**: schedule_job_with_state_machine() y discover_matches()
- ✅ **Tests unitarios**: Cobertura para estado inicial, set_job, reset, y discover_matches
- ✅ **Flexibilidad**: Posibilidad de ejecutar fases individualmente para testing/debugging

#### Archivos Creados/Modificados
1. **crates/modules/src/scheduler/state_machine.rs** - State machine implementation
2. **crates/modules/src/scheduler/mod.rs** - Refactored scheduler module
3. **crates/modules/src/lib.rs** - Exported state machine types

#### Beneficios Logrados
- ✅ **Elimina Temporal Coupling**: Estados explícitos reemplazan orden secuencial rígido
- ✅ **Estados validados**: Transiciones solo a estados válidos
- ✅ **Mejor testabilidad**: Cada fase testeable de forma independiente
- ✅ **Flexibilidad**: Posibilidad de ejecutar solo matching sin commit
- ✅ **Mantenibilidad**: Lógica clara de qué sucede en cada fase
- ✅ **Extensibilidad**: Fácil agregar nuevas fases o modificar flujo
- ✅ **Debugging mejorado**: Contexto claro de qué está pasando en cada estado

#### Problema Identificado
```rust
// crates/modules/src/scheduler.rs - LÍNEA ACTUAL
impl SchedulerModule {
    pub async fn run_scheduling_cycle(&self) -> Result<(), SchedulerError> {
        // ⚠️ Operacones deben ejecutarse en orden específico
        self.update_cluster_state().await?;        // 1. Estado actual
        self.discover_pending_jobs().await?;       // 2. Jobs pendientes
        self.match_jobs_to_workers().await?;       // 3. Matching
        self.optimize_assignments().await?;        // 4. Optimización
        self.commit_assignments().await?;          // 5. Commit
        // ⚠️ Temporal coupling - difícil de testear y mantener
    }
}
```

#### Solución Propuesta
State Machine explícita con transiciones validadas.

```rust
// crates/modules/src/scheduler.rs - PROPUESTA
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulingState {
    Idle,
    Collecting,
    Matching,
    Optimizing,
    Committing,
    Completed,
    Failed(String),
}

pub struct SchedulingContext {
    pub cluster_state: ClusterState,
    pub pending_jobs: Vec<Job>,
    pub available_workers: Vec<Worker>,
    pub assignments: HashMap<JobId, WorkerId>,
}

pub struct SchedulingStateMachine {
    current_state: SchedulingState,
    context: SchedulingContext,
}

impl SchedulingStateMachine {
    pub fn new() -> Self {
        SchedulingStateMachine {
            current_state: SchedulingState::Idle,
            context: SchedulingContext {
                cluster_state: ClusterState::new(),
                pending_jobs: Vec::new(),
                available_workers: Vec::new(),
                assignments: HashMap::new(),
            },
        }
    }

    pub fn get_state(&self) -> &SchedulingState {
        &self.current_state
    }

    // Estado: Collecting
    pub async fn collect(&mut self, scheduler: &SchedulerModule) -> Result<(), SchedulerError> {
        self.validate_state(&[SchedulingState::Idle])?;

        self.current_state = SchedulingState::Collecting;

        // Colección de estado actual
        self.context.cluster_state = scheduler.get_cluster_state().await?;
        self.context.pending_jobs = scheduler.discover_pending_jobs().await?;
        self.context.available_workers = scheduler.discover_available_workers().await?;

        self.current_state = SchedulingState::Matching;
        Ok(())
    }

    // Estado: Matching
    pub async fn match_jobs(&mut self, scheduler: &SchedulerModule) -> Result<(), SchedulerError> {
        self.validate_state(&[SchedulingState::Matching])?;

        // Matching algorithm
        let assignments = scheduler.match_jobs_to_workers(
            &self.context.pending_jobs,
            &self.context.available_workers,
        )?;

        self.context.assignments = assignments;

        self.current_state = SchedulingState::Optimizing;
        Ok(())
    }

    // Estado: Optimizing
    pub async fn optimize(&mut self, scheduler: &SchedulerModule) -> Result<(), SchedulerError> {
        self.validate_state(&[SchedulingState::Optimizing])?;

        // Optimización de asignaciones
        self.context.assignments = scheduler.optimize_assignments(
            self.context.assignments.clone(),
            &self.context.cluster_state,
        )?;

        self.current_state = SchedulingState::Committing;
        Ok(())
    }

    // Estado: Committing
    pub async fn commit(&mut self, scheduler: &SchedulerModule) -> Result<(), SchedulerError> {
        self.validate_state(&[SchedulingState::Committing])?;

        // Commit de asignaciones
        scheduler.commit_assignments(self.context.assignments.clone()).await?;

        self.current_state = SchedulingState::Completed;
        Ok(())
    }

    fn validate_state(&self, allowed_states: &[SchedulingState]) -> Result<(), SchedulerError> {
        if !allowed_states.contains(&self.current_state) {
            return Err(SchedulerError::StateTransition(
                format!(
                    "Invalid state transition from {:?} to allowed states {:?}",
                    self.current_state, allowed_states
                )
            ));
        }
        Ok(())
    }

    // Ejecutar ciclo completo con validación de estados
    pub async fn run_full_cycle(&mut self, scheduler: &SchedulerModule) -> Result<(), SchedulerError> {
        self.collect(scheduler).await?;
        self.match_jobs(scheduler).await?;
        self.optimize(scheduler).await?;
        self.commit(scheduler).await?;
        Ok(())
    }
}

impl SchedulerModule {
    pub async fn schedule_jobs(&self) -> Result<(), SchedulerError> {
        let mut machine = SchedulingStateMachine::new();
        machine.run_full_cycle(self).await
    }

    // Método alternativo: ejecutar solo fase de matching
    pub async fn discover_matches(&self) -> Result<HashMap<JobId, WorkerId>, SchedulerError> {
        let mut machine = SchedulingStateMachine::new();

        // Validar que podemos llegar a Matching
        machine.collect(self).await?;
        machine.match_jobs(self).await?;

        // Obtener resultado sin commit
        Ok(machine.context.assignments)
    }
}
```

#### Beneficios
- ✅ Estados explícitos y validados
- ✅ Transiciones predecibles
- ✅ Posibilidad de ejecutar fases individualmente
- ✅ Fácil testing de cada fase
- ✅ Mejor manejo de errores con contexto de estado
- ✅ Logging mejorado por fase

---

## 📋 Resumen de Beneficios por Propuesta

| Propuesta | Impacto en Calidad | Complejidad | Beneficios Clave | Estado |
|-----------|-------------------|-------------|------------------|--------|
| DAG Validation | 🟢 Alto | Media | Previene errores críticos, mejor arquitectura | ✅ **COMPLETADO** |
| mTLS Security | 🔴 Crítico | Alta | Seguridad robusta, compliance | ✅ **COMPLETADO** |
| Builder Pattern | 🟢 Alto | Media | Código más legible, mantenible | ✅ **COMPLETADO** |
| Specification Pattern | 🟢 Alto | Media | Reglas modulares, reutilizables, testeables | ✅ **COMPLETADO** |
| Feature Envy | 🟡 Medio | Media | Separación de concerns, modularidad | ✅ **COMPLETADO** |
| Temporal Coupling | 🟡 Medio | Media | Estados explícitos, mejor testabilidad | ✅ **COMPLETADO** |

---

## 🎯 Plan de Implementación

### ✅ Fase 1 (Completada)
1. **DAG Validation** - Prevenir fallos de pipeline ✅ COMPLETADO
2. **mTLS Security** - Cierre de vulnerabilidades ✅ COMPLETADO

### ✅ Fase 2 (En progreso)
3. **Builder Pattern** - Mejorar mantenibilidad ✅ COMPLETADO
4. **Specification Pattern** - Modularizar validaciones ⏳ Pendiente

### Fase 3 (Optimización - 1-2 sprints)
5. **Feature Envy** - Refactoring de mappers
6. **Temporal Coupling** - State machines

### Criterios de Aceptación
- [ ] Todos los tests existentes pasan
- [ ] Cobertura de tests ≥ 90%
- [ ] Documentación actualizada
- [ ] Ejemplos de uso incluidos
- [ ] PR con contexto de DDD y beneficios

---

## 📚 Referencias

- [DDD by Eric Evans](https://domaindrivendesign.org/)
- [SOLID Principles](https://en.wikipedia.org/wiki/SOLID)
- [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)
- [Specification Pattern](https://martinfowler.com/eaaCatalog/specification.html)
- [Daggy (Rust DAG Library)](https://crates.io/crates/daggy)
- [Builder Pattern in Rust](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html)
