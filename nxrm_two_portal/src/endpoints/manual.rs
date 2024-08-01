// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use std::net::SocketAddr;
use std::ops::Deref;

use axum::extract::{ConnectInfo, Host, Query, State};
use axum::http::StatusCode;
use axum::Extension;
use axum_extra::headers::UserAgent;
use axum_extra::TypedHeader;
use portal_api::api_types::PublishingType;
use repository::traits::Repository;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::auth::UserToken;
use crate::errors::ApiError;
use crate::publish::publish;
use crate::state::AppState;

#[instrument(skip(app_state, user_token))]
pub(crate) async fn manual_upload_default_repository<R: Repository>(
    Host(host): Host,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    State(app_state): State<AppState<R>>,
    Extension(user_token): Extension<UserToken>,
    Query(params): Query<ManualUploadQueryParams>,
) -> Result<StatusCode, ApiError> {
    tracing::debug!("Request to manually uplaod the bundle to Portal");

    let repository_key = app_state
        .repository
        .open_no_profile_repository(&user_token.token_username, &addr.ip())
        .await?;

    let credentials = user_token.as_credentials();

    publish(
        &app_state.portal_api_client,
        app_state.repository.deref(),
        &credentials,
        &repository_key,
        params.get_publishing_type(),
    )
    .await?;

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ManualUploadQueryParams {
    publishing_type: Option<String>,
}

impl ManualUploadQueryParams {
    fn get_publishing_type(&self) -> PublishingType {
        self.publishing_type
            .to_owned()
            .map(|pt| {
                if pt.to_lowercase().eq("automatic") {
                    PublishingType::Automatic
                } else {
                    PublishingType::UserManaged
                }
            })
            .unwrap_or(PublishingType::UserManaged)
    }
}
