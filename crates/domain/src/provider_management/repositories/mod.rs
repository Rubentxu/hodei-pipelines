//! Provider Repository Port

use super::entities::Provider;
use crate::shared_kernel::DomainResult;

#[async_trait::async_trait]
pub trait ProviderRepository: Send + Sync {
    async fn save(&self, provider: &Provider) -> DomainResult<()>;
    async fn find_by_id(
        &self,
        id: &crate::shared_kernel::ProviderId,
    ) -> DomainResult<Option<Provider>>;
    async fn list(&self) -> DomainResult<Vec<Provider>>;
    async fn delete(&self, id: &crate::shared_kernel::ProviderId) -> DomainResult<()>;
}
