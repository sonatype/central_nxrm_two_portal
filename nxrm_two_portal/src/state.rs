// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use std::sync::Arc;

use jwt_simple::algorithms::RS256PublicKey;
use portal_api::PortalApiClient;
use repository::traits::Repository;

pub struct AppState<R: Repository> {
    pub repository: Arc<R>,
    pub portal_api_client: Arc<PortalApiClient>,
    pub jwt_verification_key: Arc<RS256PublicKey>,
}

impl<R: Repository> AppState<R> {
    pub fn new(
        repository: R,
        portal_api_client: PortalApiClient,
        jwt_verification_key: RS256PublicKey,
    ) -> Self {
        Self {
            repository: Arc::new(repository),
            portal_api_client: Arc::new(portal_api_client),
            jwt_verification_key: Arc::new(jwt_verification_key),
        }
    }
}

impl<R: Repository> Clone for AppState<R> {
    fn clone(&self) -> Self {
        Self {
            repository: self.repository.clone(),
            portal_api_client: self.portal_api_client.clone(),
            jwt_verification_key: self.jwt_verification_key.clone(),
        }
    }
}
