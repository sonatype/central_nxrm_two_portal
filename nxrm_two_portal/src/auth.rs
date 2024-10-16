// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use eyre::{bail, OptionExt};
use repository::traits::Repository;
use tracing::instrument;
use user_auth::user_token::UserToken;

use crate::state::AppState;

#[instrument(skip(app_state, req, next))]
pub async fn auth<R: Repository>(
    State(app_state): State<AppState<R>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| {
            tracing::error!("Expected auth header");
            StatusCode::UNAUTHORIZED
        })?;

    let token = token_from_header(auth_header).map_err(|e| {
        tracing::error!("Failed to parse auth header: {e}");
        StatusCode::UNAUTHORIZED
    })?;
    let user_token = UserToken::from_token(&token).map_err(|e| {
        tracing::error!("Failed to decode user token: {e}");
        StatusCode::UNAUTHORIZED
    })?;
    let name_code = user_token.token_username.clone();
    tracing::trace!(name_code = ?name_code, "Parsed user token from header");

    let jwt = app_state
        .portal_api_client
        .request_jwt(&user_token)
        .await
        .map_err(|e| {
            tracing::error!(name_code = ?name_code, "Failed to request a JWT: {e}");
            StatusCode::UNAUTHORIZED
        })?;
    tracing::trace!(name_code= ?name_code, "Retrieved JWT");

    let user_auth_context = app_state.jwt_verifier.verify_jwt(jwt).map_err(|e| {
        tracing::error!(name_code = ?name_code, "Failed to verify the JWT: {e}");
        StatusCode::UNAUTHORIZED
    })?;
    tracing::trace!(name_code= ?name_code, user_id =?user_auth_context.user_id,
        "Verified JWT",
    );

    req.extensions_mut().insert(user_auth_context);
    Ok(next.run(req).await)
}

const BASIC_PREFIX: &str = "Basic ";
const BEARER_PREFIX: &str = "Bearer ";

fn token_from_header(auth_header: &str) -> eyre::Result<String> {
    if auth_header.starts_with(BASIC_PREFIX) {
        tracing::trace!("Basic authorization header provided");
        return auth_header
            .strip_prefix(BASIC_PREFIX)
            .map(String::from)
            .ok_or_eyre("Improperly formatted Basic auth header");
    } else if auth_header.starts_with(BEARER_PREFIX) {
        tracing::trace!("Bearer authorization header provided");
        return auth_header
            .strip_prefix(BEARER_PREFIX)
            .map(String::from)
            .ok_or_eyre("Improperly formatted Bearer auth header");
    }
    bail!("Auth header provided with some other prefix");
}
