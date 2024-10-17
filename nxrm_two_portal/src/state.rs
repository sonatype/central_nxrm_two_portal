// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use std::sync::Arc;

use portal_api::PortalApiClient;
use repository::traits::Repository;
use user_auth::jwt::JwtVerifier;

pub struct AppState<R: Repository> {
    pub repository: Arc<R>,
    pub portal_api_client: Arc<PortalApiClient>,
    pub jwt_verifier: Arc<JwtVerifier>,
}

impl<R: Repository> AppState<R> {
    pub fn new(
        repository: R,
        portal_api_client: PortalApiClient,
        jwt_verifier: JwtVerifier,
    ) -> Self {
        Self {
            repository: Arc::new(repository),
            portal_api_client: Arc::new(portal_api_client),
            jwt_verifier: Arc::new(jwt_verifier),
        }
    }
}

impl<R: Repository> Clone for AppState<R> {
    fn clone(&self) -> Self {
        Self {
            repository: self.repository.clone(),
            portal_api_client: self.portal_api_client.clone(),
            jwt_verifier: self.jwt_verifier.clone(),
        }
    }
}
