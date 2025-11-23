use hodei_adapters::security::JwtTokenService;
use hodei_ports::security::TokenService;
use std::sync::Arc;
use tonic::{Request, Status, service::Interceptor};

#[derive(Clone)]
pub struct AuthInterceptor {
    token_service: Arc<JwtTokenService>,
}

impl AuthInterceptor {
    pub fn new(token_service: Arc<JwtTokenService>) -> Self {
        Self { token_service }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let token = match request.metadata().get("authorization") {
            Some(t) => t
                .to_str()
                .map_err(|_| Status::unauthenticated("Invalid token format"))?,
            None => return Err(Status::unauthenticated("Missing authorization token")),
        };

        let bearer = token
            .strip_prefix("Bearer ")
            .ok_or_else(|| Status::unauthenticated("Invalid token format"))?;

        match self.token_service.verify_token(bearer) {
            Ok(_) => Ok(request),
            Err(e) => Err(Status::unauthenticated(format!("Invalid token: {}", e))),
        }
    }
}
