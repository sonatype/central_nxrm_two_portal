use std::sync::Arc;

use portal_api::PortalApiClient;
use repository::traits::Repository;

pub struct AppState<R: Repository> {
    pub repository: Arc<R>,
    pub portal_api_client: Arc<PortalApiClient>,
}

impl<R: Repository> AppState<R> {
    pub fn new(repository: R, portal_api_client: PortalApiClient) -> Self {
        Self {
            repository: Arc::new(repository),
            portal_api_client: Arc::new(portal_api_client),
        }
    }
}

impl<R: Repository> Clone for AppState<R> {
    fn clone(&self) -> Self {
        Self {
            repository: self.repository.clone(),
            portal_api_client: self.portal_api_client.clone(),
        }
    }
}
