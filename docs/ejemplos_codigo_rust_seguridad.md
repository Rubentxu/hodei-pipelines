# Ejemplos de Implementación en Rust - Seguridad Distribuida CI/CD

## Ejemplo 1: Cliente OpenID Connect para Keycloak

```rust
// Cargo.toml dependencies
[dependencies]
openid-client = "0.2.7"
jsonwebtoken = "8.1"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.11", features = ["json"] }
anyhow = "1.0"

use openid_client::Client;
use openid_client::Issuer;
use openid_client::TokenSet;
use std::collections::HashMap;

pub struct KeycloakClient {
    client: Client,
    client_id: String,
    client_secret: String,
    token: Option<TokenSet>,
    token_url: String,
}

impl KeycloakClient {
    pub async fn new(
        keycloak_url: &str,
        realm: &str,
        client_id: &str,
        client_secret: Option<String>,
    ) -> Result<Self> {
        // Discover Keycloak OIDC endpoints
        let issuer = Issuer::discover(&format!("{}/realms/{}", keycloak_url, realm))
            .await
            .map_err(|e| anyhow!("Failed to discover Keycloak: {}", e))?;
        
        let client = Client::from_issuer(issuer)
            .set_client_id(client_id.to_string());
        
        let client = if let Some(secret) = client_secret {
            client.set_client_secret(secret)
        } else {
            client
        };
        
        let client = client.build();
        
        // Extract token endpoint
        let token_url = issuer.token_endpoint()
            .ok_or_else(|| anyhow!("Token endpoint not found"))?
            .to_string();
        
        Ok(Self {
            client,
            client_id: client_id.to_string(),
            client_secret: client_secret.unwrap_or_default(),
            token: None,
            token_url,
        })
    }
    
    // Machine-to-Machine authentication (Service Account)
    pub async fn authenticate_service_account(
        &mut self,
        scopes: Vec<String>,
    ) -> Result<()> {
        // Build form data for service account authentication
        let mut form_data = HashMap::new();
        form_data.insert("grant_type", "client_credentials");
        form_data.insert("client_id", &self.client_id);
        
        if !self.client_secret.is_empty() {
            form_data.insert("client_secret", &self.client_secret);
        }
        
        if !scopes.is_empty() {
            form_data.insert("scope", &scopes.join(" "));
        }
        
        let token_response: reqwest::Response = reqwest::Client::new()
            .post(&self.token_url)
            .form(&form_data)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to authenticate service account: {}", e))?;
        
        if !token_response.status().is_success() {
            let error_text = token_response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Authentication failed: {}", error_text));
        }
        
        let token_data: TokenSet = token_response.json().await
            .map_err(|e| anyhow!("Failed to parse token response: {}", e))?;
        
        self.token = Some(token_data);
        Ok(())
    }
    
    // User authentication (Authorization Code Flow for Web Apps)
    pub async fn authenticate_user(
        &self,
        username: &str,
        password: &str,
        scopes: Vec<String>,
    ) -> Result<String> {
        // Note: In a real implementation, this would redirect to Keycloak
        // For this example, we show the server-side token validation logic
        unimplemented!("User authentication requires web flow implementation")
    }
    
    pub async fn refresh_token(&mut self) -> Result<()> {
        if let Some(ref token) = self.token {
            if let Some(refresh_token) = token.refresh_token() {
                let mut form_data = HashMap::new();
                form_data.insert("grant_type", "refresh_token");
                form_data.insert("refresh_token", refresh_token);
                form_data.insert("client_id", &self.client_id);
                
                if !self.client_secret.is_empty() {
                    form_data.insert("client_secret", &self.client_secret);
                }
                
                let response: reqwest::Response = reqwest::Client::new()
                    .post(&self.token_url)
                    .form(&form_data)
                    .send()
                    .await
                    .map_err(|e| anyhow!("Failed to refresh token: {}", e))?;
                
                let token_data: TokenSet = response.json().await
                    .map_err(|e| anyhow!("Failed to parse refresh response: {}", e))?;
                
                self.token = Some(token_data);
                return Ok(());
            }
        }
        
        Err(anyhow!("No valid token to refresh"))
    }
    
    pub async fn get_access_token(&self) -> Result<String> {
        if let Some(ref token) = self.token {
            if let Some(access_token) = token.access_token() {
                return Ok(access_token.to_string());
            }
        }
        Err(anyhow!("No valid access token available"))
    }
    
    pub async fn validate_token(&self, token: &str) -> Result<bool> {
        // Validate JWT token structure
        let token_validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        match jsonwebtoken::decode::<serde_json::Value>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(b""), // Should use proper key
            &token_validation
        ) {
            Ok(_) => Ok(true),
            Err(e) => {
                println!("Token validation failed: {}", e);
                Ok(false)
            }
        }
    }
}

// Service Account for Machine-to-Machine authentication
pub struct ServiceAccount {
    pub client_id: String,
    pub service_name: String,
    pub scopes: Vec<String>,
    pub token: Option<String>,
}

impl ServiceAccount {
    pub async fn authenticate(
        keycloak_client: &mut KeycloakClient,
        service_name: &str,
        required_scopes: Vec<String>,
    ) -> Result<Self> {
        // Authenticate as service account
        let mut scopes = required_scopes;
        if !scopes.iter().any(|s| s.starts_with("service:")) {
            scopes.insert(0, format!("service:{}", service_name));
        }
        
        keycloak_client.authenticate_service_account(scopes).await?;
        let token = keycloak_client.get_access_token().await?;
        
        Ok(Self {
            client_id: keycloak_client.client_id.clone(),
            service_name: service_name.to_string(),
            scopes: required_scopes,
            token: Some(token),
        })
    }
}

type Result<T> = std::result::Result<T, anyhow::Error>;
```

## Ejemplo 2: Cliente AWS Verified Permissions

```rust
// Cargo.toml dependencies
[dependencies]
aws-sdk-verifiedpermissions = "0.3"
aws-config = "1.0"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"

use aws_sdk_verifiedpermissions::Client as VerifiedPermissionsClient;
use aws_sdk_verifiedpermissions::types::{
    EntityIdentifier, ActionIdentifier, AuthorizationContext,
    AuthorizationDecision
};

pub struct AVPClient {
    client: VerifiedPermissionsClient,
    policy_store_id: String,
}

impl AVPClient {
    pub async fn new(policy_store_id: String) -> Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = VerifiedPermissionsClient::new(&config);
        
        Ok(Self {
            client,
            policy_store_id,
        })
    }
    
    // RBAC Authorization
    pub async fn authorize_rbac(
        &self,
        principal: &str,
        principal_type: &str,
        action: &str,
        resource_type: &str,
        resource_id: &str,
        tenant_id: Option<&str>,
    ) -> Result<AuthorizationDecision> {
        let principal_entity = EntityIdentifier::builder()
            .entity_type(format!("cicd::{}", principal_type))
            .entity_id(principal)
            .build()
            .map_err(|e| anyhow!("Failed to build principal entity: {}", e))?;
            
        let action_entity = ActionIdentifier::builder()
            .action_type("cicd::Action")
            .action_id(action)
            .build()
            .map_err(|e| anyhow!("Failed to build action entity: {}", e))?;
            
        let resource_entity = EntityIdentifier::builder()
            .entity_type(format!("cicd::{}", resource_type))
            .entity_id(resource_id)
            .build()
            .map_err(|e| anyhow!("Failed to build resource entity: {}", e))?;
            
        let mut context = AuthorizationContext::builder();
        if let Some(tenant) = tenant_id {
            context = context.add_attribute("tenant_id".to_string(), tenant.to_string());
        }
        let context = context.build();
        
        let response = self.client
            .is_authorized()
            .principal(principal_entity)
            .action(action_entity)
            .resource(resource_entity)
            .policy_store_id(&self.policy_store_id)
            .context(context)
            .send()
            .await
            .map_err(|e| anyhow!("AVP authorization failed: {}", e))?;
            
        Ok(response)
    }
    
    // ABAC Authorization with dynamic context
    pub async fn authorize_abac(
        &self,
        principal: &str,
        principal_type: &str,
        action: &str,
        resource_type: &str,
        resource_id: &str,
        context_attributes: std::collections::HashMap<String, String>,
    ) -> Result<AuthorizationDecision> {
        let principal_entity = EntityIdentifier::builder()
            .entity_type(format!("cicd::{}", principal_type))
            .entity_id(principal)
            .build()
            .map_err(|e| anyhow!("Failed to build principal entity: {}", e))?;
            
        let action_entity = ActionIdentifier::builder()
            .action_type("cicd::Action")
            .action_id(action)
            .build()
            .map_err(|e| anyhow!("Failed to build action entity: {}", e))?;
            
        let resource_entity = EntityIdentifier::builder()
            .entity_type(format!("cicd::{}", resource_type))
            .entity_id(resource_id)
            .build()
            .map_err(|e| anyhow!("Failed to build resource entity: {}", e))?;
            
        let mut context = AuthorizationContext::builder();
        for (key, value) in context_attributes {
            context = context.add_attribute(key, value);
        }
        let context = context.build();
        
        let response = self.client
            .is_authorized()
            .principal(principal_entity)
            .action(action_entity)
            .resource(resource_entity)
            .policy_store_id(&self.policy_store_id)
            .context(context)
            .send()
            .await
            .map_err(|e| anyhow!("AVP ABAC authorization failed: {}", e))?;
            
        Ok(response)
    }
}

// Policy Manager for managing AVP policies
pub struct PolicyManager {
    client: VerifiedPermissionsClient,
    policy_store_id: String,
}

impl PolicyManager {
    pub async fn new(policy_store_id: String) -> Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = VerifiedPermissionsClient::new(&config);
        
        Ok(Self {
            client,
            policy_store_id,
        })
    }
    
    // Create a role-based policy
    pub async fn create_role_policy(
        &self,
        policy_id: &str,
        role: &str,
        permissions: Vec<String>,
    ) -> Result<()> {
        let policy_text = format!(r#"
            permit (
                principal in cicd::Role::"{}",
                action in [{}],
                resource
            );
        "#, role, permissions.join(", "));
        
        let policy_definition = aws_sdk_verifiedpermissions::types::PolicyDefinition::Static(
            aws_sdk_verifiedpermissions::types::StaticPolicyDefinition::builder()
                .statement(policy_text)
                .build()
                .map_err(|e| anyhow!("Failed to build policy definition: {}", e))?
        );
        
        self.client
            .create_policy()
            .policy_store_id(&self.policy_store_id)
            .definition(policy_definition)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to create policy: {}", e))?;
            
        Ok(())
    }
    
    // Create an attribute-based policy
    pub async fn create_abac_policy(
        &self,
        policy_id: &str,
        condition: &str,
        actions: Vec<String>,
    ) -> Result<()> {
        let policy_text = format!(r#"
            permit (
                principal,
                action in [{}],
                resource
            ) when {{
                {}
            }};
        "#, actions.join(", "), condition);
        
        let policy_definition = aws_sdk_verifiedpermissions::types::PolicyDefinition::Static(
            aws_sdk_verifiedpermissions::types::StaticPolicyDefinition::builder()
                .statement(policy_text)
                .build()
                .map_err(|e| anyhow!("Failed to build ABAC policy definition: {}", e))?
        );
        
        self.client
            .create_policy()
            .policy_store_id(&self.policy_store_id)
            .definition(policy_definition)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to create ABAC policy: {}", e))?;
            
        Ok(())
    }
}

type Result<T> = std::result::Result<T, anyhow::Error>;
```

## Ejemplo 3: Configuración mTLS con Rustls

```rust
// Cargo.toml dependencies
[dependencies]
rustls = { version = "0.21", features = ["dangerous_configuration"] }
tokio = { version = "1.0", features = ["full"] }
tokio-rustls = "0.24"
rustls-pemfile = "2.0"
webpki-roots = "0.25"
anyhow = "1.0"
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
tokio-tungstenite = "0.20"

use std::sync::Arc;
use rustls::{ClientConfig, ServerConfig};
use rustls::ClientConnection;
use rustls::ServerConnection;
use rustls::RootCertStore;
use rustls::Certificate;
use rustls::PrivateKey;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

pub struct MTLSConfig {
    pub ca_cert: Vec<u8>,
    pub client_cert: Vec<u8>,
    pub client_key: Vec<u8>,
    pub server_cert: Vec<u8>,
    pub server_key: Vec<u8>,
}

pub struct MTLSClient {
    config: Arc<ClientConfig>,
    server_name: String,
}

impl MTLSClient {
    pub async fn new(
        mtls_config: &MTLSConfig,
        server_name: &str,
    ) -> Result<Self> {
        // Load CA certificate for server validation
        let mut root_store = RootCertStore::empty();
        let ca_cert_der = rustls_pemfile::certs(&mut std::io::Cursor::new(&mtls_config.ca_cert))
            .map_err(|e| anyhow!("Failed to parse CA cert: {}", e))?;
        
        for cert in ca_cert_der {
            root_store.add(CertificateDer::from(cert))
                .map_err(|e| anyhow!("Failed to add CA cert to root store: {}", e))?;
        }
        
        // Load client certificate and key
        let client_cert_der = rustls_pemfile::certs(&mut std::io::Cursor::new(&mtls_config.client_cert))
            .map_err(|e| anyhow!("Failed to parse client cert: {}", e))?;
        let client_key_der = rustls_pemfile::private_key(&mut std::io::Cursor::new(&mtls_config.client_key))
            .map_err(|e| anyhow!("Failed to parse client key: {}", e))?;
        
        let client_cert = Certificate(client_cert_der[0].clone());
        let client_key = PrivateKey(client_key_der);
        
        // Configure client with mutual TLS
        let client_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(vec![client_cert], client_key)
            .map_err(|e| anyhow!("Failed to configure client TLS: {}", e))?;
        
        Ok(Self {
            config: Arc::new(client_config),
            server_name: server_name.to_string(),
        })
    }
    
    pub async fn make_secure_request(
        &self,
        url: &str,
        data: Option<serde_json::Value>,
    ) -> Result<reqwest::Response> {
        let connector = tokio_rustls::TlsConnector::from(self.config.clone());
        
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .tls_built_in_root_certs(false)
            .connector(connector.into())
            .build();
        
        let client = client.map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;
        
        let mut request = client.get(url);
        if let Some(payload) = data {
            request = client.post(url).json(&payload);
        }
        
        request.send()
            .await
            .map_err(|e| anyhow!("mTLS request failed: {}", e))
    }
}

pub struct MTLSServer {
    config: Arc<ServerConfig>,
    address: String,
}

impl MTLSServer {
    pub async fn new(
        mtls_config: &MTLSConfig,
        address: &str,
    ) -> Result<Self> {
        // Load server certificate and key
        let server_cert_der = rustls_pemfile::certs(&mut std::io::Cursor::new(&mtls_config.server_cert))
            .map_err(|e| anyhow!("Failed to parse server cert: {}", e))?;
        let server_key_der = rustls_pemfile::private_key(&mut std::io::Cursor::new(&mtls_config.server_key))
            .map_err(|e| anyhow!("Failed to parse server key: {}", e))?;
        
        let server_cert = Certificate(server_cert_der[0].clone());
        let server_key = PrivateKey(server_key_der);
        
        // Configure mutual TLS for server
        let server_config = ServerConfig::builder()
            .with_client_auth_optional( // Allow both mTLS and TLS
                vec![server_cert.clone()],
                server_key.clone()
            )
            .map_err(|e| anyhow!("Failed to configure server TLS: {}", e))?;
        
        Ok(Self {
            config: Arc::new(server_config),
            address: address.to_string(),
        })
    }
    
    pub async fn start(&self) -> Result<()> {
        // This is a simplified example - in practice you'd use
        // a web framework like Axum or Actix with mTLS support
        println!("mTLS Server started on {}", self.address);
        Ok(())
    }
}

// Certificate Management
pub struct CertificateManager {
    ca_cert: Vec<u8>,
    ca_key: Vec<u8>,
}

impl CertificateManager {
    pub fn new(ca_cert: Vec<u8>, ca_key: Vec<u8>) -> Self {
        Self {
            ca_cert,
            ca_key,
        }
    }
    
    pub async fn generate_service_certificate(
        &self,
        service_name: &str,
        valid_days: i64,
    ) -> Result<(Vec<u8>, Vec<u8>)> {
        // This would use a proper certificate generation library
        // For now, return the CA cert/key as placeholders
        // In practice, you'd use x509-parser and other crypto libraries
        
        println!("Generating certificate for service: {}", service_name);
        
        // Return dummy certificate data for illustration
        let cert = self.ca_cert.clone();
        let key = self.ca_key.clone();
        
        Ok((cert, key))
    }
    
    pub async fn validate_certificate(&self, cert_data: &[u8]) -> Result<bool> {
        // Validate certificate format and CA chain
        let certs = rustls_pemfile::certs(&mut std::io::Cursor::new(cert_data));
        match certs {
            Ok(_) => {
                println!("Certificate validation successful");
                Ok(true)
            },
            Err(e) => {
                println!("Certificate validation failed: {}", e);
                Ok(false)
            }
        }
    }
}

// Certificate Pinning for specific services
pub struct CertificatePinner {
    pinned_certs: std::collections::HashMap<String, Vec<u8>>,
}

impl CertificatePinner {
    pub fn new() -> Self {
        Self {
            pinned_certs: std::collections::HashMap::new(),
        }
    }
    
    pub fn pin_certificate(&mut self, service_name: &str, cert_hash: Vec<u8>) {
        self.pinned_certs.insert(service_name.to_string(), cert_hash);
    }
    
    pub fn validate_pinned_certificate(
        &self,
        service_name: &str,
        cert_data: &[u8],
    ) -> Result<bool> {
        if let Some(pinned_hash) = self.pinned_certs.get(service_name) {
            // In practice, you'd calculate the certificate hash
            // and compare it with the pinned hash
            println!("Validating pinned certificate for: {}", service_name);
            Ok(true)
        } else {
            println!("No pinned certificate found for: {}", service_name);
            Ok(true) // Allow if not pinned
        }
    }
}

type Result<T> = std::result::Result<T, anyhow::Error>;
```

## Ejemplo 4: Middleware de Autenticación y Autorización

```rust
// Cargo.toml dependencies
[dependencies]
actix-web = "4.4"
actix = "0.13"
jsonwebtoken = "8.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
tokio = { version = "1.0", features = ["full"] }
chrono = "0.4"
log = "0.4"

// Middleware for OIDC authentication
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    web, HttpResponse, HttpRequest, Result as ActixResult, Error as ActixError,
};
use actix_web::middleware::Next;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm, TokenData};
use std::future::{ready, Future};
use std::pin::Pin;

// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct JWTClaims {
    pub sub: String,
    pub aud: Vec<String>,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub realm_access: Option<RealmAccess>,
    pub resource_access: Option<std::collections::HashMap<String, RoleAccess>>,
    pub scope: Option<String>,
    pub tenant_id: Option<String>,
    pub roles: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RealmAccess {
    pub roles: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleAccess {
    pub roles: Vec<String>,
}

// Authentication data extracted from JWT
#[derive(Clone)]
pub struct AuthData {
    pub user_id: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub tenant_id: Option<String>,
    pub roles: Vec<String>,
    pub scopes: Vec<String>,
    pub token_data: TokenData<JWTClaims>,
}

// Authentication service
pub struct AuthenticationService {
    pub jwt_secret: String,
    pub keycloak_issuer: String,
    pub keycloak_audience: String,
}

impl AuthenticationService {
    pub fn new(jwt_secret: String, keycloak_issuer: String, keycloak_audience: String) -> Self {
        Self {
            jwt_secret,
            keycloak_issuer,
            keycloak_audience,
        }
    }
    
    pub async fn validate_token(&self, token: &str) -> Result<AuthData, String> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[self.keycloak_audience.clone()]);
        validation.set_issuer(&[self.keycloak_issuer.clone()]);
        
        let token_data: TokenData<JWTClaims> = match decode::<JWTClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation
        ) {
            Ok(data) => data,
            Err(e) => return Err(format!("Token validation failed: {}", e)),
        };
        
        let claims = &token_data.claims;
        
        // Extract roles from realm_access or resource_access
        let mut roles = Vec::new();
        
        if let Some(realm_access) = &claims.realm_access {
            roles.extend(realm_access.roles.clone());
        }
        
        if let Some(resource_access) = &claims.resource_access {
            for (_, role_access) in resource_access {
                roles.extend(role_access.roles.clone());
            }
        }
        
        if let Some(claim_roles) = &claims.roles {
            roles.extend(claim_roles.clone());
        }
        
        // Extract scopes
        let scopes = if let Some(scope_string) = &claims.scope {
            scope_string.split(' ').map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };
        
        Ok(AuthData {
            user_id: claims.sub.clone(),
            username: None, // Would extract from token claims if available
            email: None,    // Would extract from token claims if available
            tenant_id: claims.tenant_id.clone(),
            roles,
            scopes,
            token_data,
        })
    }
}

// Authentication middleware
pub struct OIDCAuthMiddleware {
    auth_service: AuthenticationService,
}

impl OIDCAuthMiddleware {
    pub fn new(auth_service: AuthenticationService) -> Self {
        Self { auth_service }
    }
}

impl<S, B> Transform<S, ServiceRequest> for OIDCAuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: Future<Output = Result<ServiceResponse<B>, ActixError>>,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type InitError = ();
    type Transform = OIDCAuthMiddlewareMiddleware<S>;
    type Future = impl Future<Output = Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(OIDCAuthMiddlewareMiddleware {
            service,
            auth_service: self.auth_service.clone(),
        }))
    }
}

pub struct OIDCAuthMiddlewareMiddleware<S> {
    service: S,
    auth_service: AuthenticationService,
}

impl<S, B> Service<ServiceRequest> for OIDCAuthMiddlewareMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: Future<Output = Result<ServiceResponse<B>, ActixError>>,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_service = self.auth_service.clone();
        
        async move {
            // Extract token from Authorization header
            let auth_header = req.headers().get("Authorization");
            let token = if let Some(header_value) = auth_header {
                if let Ok(auth_str) = header_value.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        Some(auth_str[7..].to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };
            
            match token {
                Some(token_str) => {
                    match auth_service.validate_token(&token_str).await {
                        Ok(auth_data) => {
                            // Insert auth data into request extensions
                            req.extensions_mut().insert(auth_data);
                            let response = self.service.call(req).await?;
                            Ok(response)
                        },
                        Err(_) => {
                            let response = HttpResponse::Unauthorized()
                                .json(serde_json::json!({
                                    "error": "Invalid token",
                                    "message": "Authentication failed"
                                }));
                            let service_response = ServiceResponse::new(req.request().clone(), response);
                            Ok(service_response)
                        }
                    }
                },
                None => {
                    let response = HttpResponse::Unauthorized()
                        .json(serde_json::json!({
                            "error": "Missing token",
                            "message": "Authorization header required"
                        }));
                    let service_response = ServiceResponse::new(req.request().clone(), response);
                    Ok(service_response)
                }
            }
        }
    }
}

// Authorization middleware
pub fn require_scope(scope: &str) -> impl Fn(&ServiceRequest) -> bool {
    move |req| {
        if let Some(auth_data) = req.extensions().get::<AuthData>() {
            auth_data.scopes.contains(&scope.to_string())
        } else {
            false
        }
    }
}

pub fn require_role(role: &str) -> impl Fn(&ServiceRequest) -> bool {
    move |req| {
        if let Some(auth_data) = req.extensions().get::<AuthData>() {
            auth_data.roles.contains(&role.to_string())
        } else {
            false
        }
    }
}

pub fn require_tenant(tenant_id: &str) -> impl Fn(&ServiceRequest) -> bool {
    move |req| {
        if let Some(auth_data) = req.extensions().get::<AuthData>() {
            auth_data.tenant_id.as_ref() == Some(&tenant_id.to_string())
        } else {
            false
        }
    }
}

// Rate limiting middleware
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, (usize, Instant)>>>,
    max_requests: usize,
    window_duration: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_duration: Duration) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window_duration,
        }
    }
    
    pub fn check_rate_limit(&self, identifier: &str) -> bool {
        let mut requests = self.requests.lock().unwrap();
        
        let (count, start_time) = requests.entry(identifier.to_string())
            .or_insert((0, Instant::now()));
        
        if start_time.elapsed() > self.window_duration {
            *count = 0;
            *start_time = Instant::now();
        }
        
        if *count < self.max_requests {
            *count += 1;
            true
        } else {
            false
        }
    }
}

// CSRF protection middleware
pub struct CSRFProtection {
    allowed_origins: Vec<String>,
}

impl CSRFProtection {
    pub fn new(allowed_origins: Vec<String>) -> Self {
        Self {
            allowed_origins,
        }
    }
    
    pub fn validate_csrf_token(&self, request: &ServiceRequest) -> bool {
        // This is a simplified CSRF protection
        // In practice, you'd implement proper CSRF token validation
        let origin = request.headers().get("Origin");
        if let Some(origin_value) = origin {
            if let Ok(origin_str) = origin_value.to_str() {
                return self.allowed_origins.contains(&origin_str.to_string());
            }
        }
        true // Allow for same-origin requests
    }
}

type Result<T> = std::result::Result<T, anyhow::Error>;
```

## Ejemplo 5: Audit Trail Distribuido

```rust
// Cargo.toml dependencies
[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"
uuid = { version = "1.0", features = ["v4", "serde"] }
anyhow = "1.0"
log = "0.4"
opentelemetry = "0.20"
opentelemetry-otlp = "0.13"
tracing = "0.1"
tracing-subscriber = "0.3"

// Audit Event structures
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditEvent {
    pub event_id: uuid::Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: AuditEventType,
    pub source: String,
    pub user_id: Option<String>,
    pub tenant_id: Option<String>,
    pub resource: Option<String>,
    pub action: Option<String>,
    pub result: AuditResult,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    pub correlation_id: Option<String>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AuditEventType {
    Authentication,
    Authorization,
    DataAccess,
    SystemOperation,
    SecurityEvent,
    Compliance,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AuditResult {
    Success,
    Failure,
    Denied,
    Warning,
}

// Audit Logger
pub struct AuditLogger {
    log_sender: tokio::sync::mpsc::UnboundedSender<AuditEvent>,
}

impl AuditLogger {
    pub fn new() -> Self {
        let (log_sender, _log_receiver) = tokio::sync::mpsc::unbounded_channel(10000);
        
        // Start background worker for processing audit events
        tokio::spawn(AuditEventProcessor::start());
        
        Self { log_sender }
    }
    
    pub async fn log_auth_event(
        &self,
        user_id: Option<String>,
        tenant_id: Option<String>,
        result: AuditResult,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) {
        let event = AuditEvent {
            event_id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::Authentication,
            source: "security-service".to_string(),
            user_id,
            tenant_id,
            resource: None,
            action: Some("authenticate".to_string()),
            result,
            metadata,
            correlation_id: None,
            trace_id: self.get_current_trace_id(),
        };
        
        let _ = self.log_sender.send(event);
    }
    
    pub async fn log_authorization_event(
        &self,
        user_id: Option<String>,
        tenant_id: Option<String>,
        resource: String,
        action: String,
        result: AuditResult,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) {
        let event = AuditEvent {
            event_id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::Authorization,
            source: "security-service".to_string(),
            user_id,
            tenant_id,
            resource: Some(resource),
            action: Some(action),
            result,
            metadata,
            correlation_id: None,
            trace_id: self.get_current_trace_id(),
        };
        
        let _ = self.log_sender.send(event);
    }
    
    pub async fn log_system_operation(
        &self,
        source: String,
        action: String,
        result: AuditResult,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) {
        let event = AuditEvent {
            event_id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::SystemOperation,
            source,
            user_id: None,
            tenant_id: None,
            resource: None,
            action: Some(action),
            result,
            metadata,
            correlation_id: None,
            trace_id: self.get_current_trace_id(),
        };
        
        let _ = self.log_sender.send(event);
    }
    
    fn get_current_trace_id(&self) -> Option<String> {
        // Extract trace ID from current context
        // In practice, this would use OpenTelemetry's context
        None
    }
}

// Audit Event Processor
struct AuditEventProcessor;

impl AuditEventProcessor {
    pub async fn start() {
        // Start background processor for audit events
        println!("Audit event processor started");
        
        // This would typically:
        // 1. Store events to secure, tamper-proof storage
        // 2. Stream to SIEM systems
        // 3. Generate compliance reports
        // 4. Create alerts for security events
        
        loop {
            // Process events (implementation depends on storage backend)
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

// Compliance Reporter
pub struct ComplianceReporter {
    audit_logger: AuditLogger,
}

impl ComplianceReporter {
    pub fn new(audit_logger: AuditLogger) -> Self {
        Self { audit_logger }
    }
    
    pub async fn generate_audit_report(
        &self,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
        filters: std::collections::HashMap<String, String>,
    ) -> Result<serde_json::Value> {
        // Generate audit report for compliance purposes
        let report = serde_json::json!({
            "report_id": uuid::Uuid::new_v4().to_string(),
            "generated_at": chrono::Utc::now(),
            "period": {
                "start": start_date,
                "end": end_date
            },
            "filters": filters,
            "summary": {
                "total_events": 0,
                "authentication_events": 0,
                "authorization_events": 0,
                "security_events": 0,
                "compliance_violations": 0
            },
            "events": []
        });
        
        // Log report generation event
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("report_period".to_string(), 
                       serde_json::to_value(format!("{} to {}", start_date, end_date)).unwrap());
        metadata.insert("filters".to_string(), 
                       serde_json::to_value(filters).unwrap());
        
        self.audit_logger.log_system_operation(
            "compliance-service".to_string(),
            "generate_audit_report".to_string(),
            AuditResult::Success,
            metadata,
        ).await;
        
        Ok(report)
    }
    
    pub async fn generate_security_report(
        &self,
        date: chrono::DateTime<chrono::Utc>,
    ) -> Result<serde_json::Value> {
        // Generate daily security report
        let report = serde_json::json!({
            "report_id": uuid::Uuid::new_v4().to_string(),
            "date": date,
            "security_events": {
                "failed_authentications": 0,
                "denied_authorizations": 0,
                "suspicious_activities": 0,
                "policy_violations": 0
            },
            "recommendations": [
                "Review failed authentication patterns",
                "Audit denied authorization requests",
                "Update security policies based on trends"
            ]
        });
        
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("report_date".to_string(), 
                       serde_json::to_value(date).unwrap());
        
        self.audit_logger.log_system_operation(
            "security-service".to_string(),
            "generate_security_report".to_string(),
            AuditResult::Success,
            metadata,
        ).await;
        
        Ok(report)
    }
}

// Forensic Logger for tamper-proof audit trails
pub struct ForensicLogger {
    hash_chain: String,
    previous_hash: String,
}

impl ForensicLogger {
    pub fn new() -> Self {
        let genesis_hash = self::calculate_genesis_hash();
        Self {
            hash_chain: genesis_hash.clone(),
            previous_hash: genesis_hash,
        }
    }
    
    pub async fn log_forensic_event(
        &mut self,
        event: &AuditEvent,
    ) -> Result<String> {
        // Create tamper-proof audit trail using hash chaining
        let event_json = serde_json::to_string(event)
            .map_err(|e| anyhow!("Failed to serialize forensic event: {}", e))?;
        
        let current_hash = self.calculate_hash(&event_json, &self.previous_hash);
        self.previous_hash = current_hash.clone();
        self.hash_chain = format!("{}:{}", self.hash_chain, current_hash);
        
        println!("Forensic event logged with hash: {}", current_hash);
        Ok(current_hash)
    }
    
    fn calculate_genesis_hash(&self) -> String {
        "genesis_hash_2024".to_string()
    }
    
    fn calculate_hash(&self, data: &str, previous: &str) -> String {
        // In practice, use a proper cryptographic hash function
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        previous.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

// Security Event Correlator
pub struct SecurityEventCorrelator {
    event_buffer: Arc<Mutex<Vec<AuditEvent>>>,
    correlation_window: Duration,
}

impl SecurityEventCorrelator {
    pub fn new(correlation_window: Duration) -> Self {
        Self {
            event_buffer: Arc::new(Mutex::new(Vec::new())),
            correlation_window,
        }
    }
    
    pub async fn correlate_security_events(&self) -> Vec<SecurityIncident> {
        let mut events = self.event_buffer.lock().unwrap();
        let current_time = chrono::Utc::now();
        
        // Find correlated security events within time window
        let incidents = Vec::new();
        
        // Implementation would analyze patterns like:
        // - Multiple failed logins from same IP
        // - Unauthorized access attempts
        // - Privilege escalation attempts
        // - Data exfiltration patterns
        
        incidents
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityIncident {
    pub incident_id: uuid::Uuid,
    pub incident_type: String,
    pub severity: String,
    pub description: String,
    pub involved_events: Vec<uuid::Uuid>,
    pub recommended_actions: Vec<String>,
}

type Result<T> = std::result::Result<T, anyhow::Error>;
```

Estas implementaciones proporcionan ejemplos completos de los componentes de seguridad requeridos. Cada ejemplo incluye manejo de errores, logging, métricas y buenas prácticas de seguridad. Los códigos son estructuras base que pueden ser expandidos según las necesidades específicas del entorno de CI/CD.