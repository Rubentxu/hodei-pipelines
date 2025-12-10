//! Application Service for Provider Management

use domain::provider_management::{Provider, ProviderRepository};

pub struct ProviderService {
    provider_repo: Box<dyn ProviderRepository>,
}

impl ProviderService {
    pub fn new(provider_repo: Box<dyn ProviderRepository>) -> Self {
        Self { provider_repo }
    }
}
