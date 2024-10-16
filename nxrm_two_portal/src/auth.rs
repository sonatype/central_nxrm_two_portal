// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use std::collections::HashSet;

use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::prelude::{Engine, BASE64_STANDARD};
use eyre::{bail, OptionExt};
use jwt_simple::{
    algorithms::{RS256PublicKey, RSAPublicKeyLike},
    common::VerificationOptions,
    token::Token,
};
use portal_api::Credentials;
use repository::traits::Repository;
use serde::{Deserialize, Serialize};
use tracing::{instrument, Level};

use crate::state::AppState;

struct UserToken {
    token_username: String,
    token_password: String,
}

impl UserToken {
    pub fn from_token(token: &str) -> eyre::Result<Self> {
        let token = BASE64_STANDARD.decode(token)?;
        let token = String::from_utf8(token)?;
        let (token_username, token_password) = token
            .split_once(':')
            .ok_or_eyre("Failed to extract a valid user token")?;

        Ok(Self {
            token_username: token_username.to_string(),
            token_password: token_password.to_string(),
        })
    }

    pub fn as_credentials(self) -> Credentials {
        Credentials::from_usertoken(self.token_username, self.token_password)
    }
}

#[derive(Clone)]
pub struct UserAuthContext {
    pub user_id: String,
    pub token_username: String,
    pub namespaces: Vec<String>,
    jwt: String,
}

impl UserAuthContext {
    pub fn as_credentials(&self) -> Credentials {
        Credentials::from_jwt(self.jwt.clone())
    }
}

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
        .request_jwt(&user_token.as_credentials())
        .await
        .map_err(|e| {
            tracing::error!(name_code = ?name_code, "Failed to request a JWT: {e}");
            StatusCode::UNAUTHORIZED
        })?;
    tracing::trace!(name_code= ?name_code, "Retrieved JWT");

    if tracing::enabled!(Level::TRACE) {
        let metadata = Token::decode_metadata(&jwt).map_err(|e| {
            tracing::error!(name_code = ?name_code, "Failed to decode metadata: {e}");
            StatusCode::UNAUTHORIZED
        })?;
        tracing::trace!(name_code = ?name_code, metadata = ?metadata, "JWT decoded");
    }

    let user_auth_context = verify_jwt(&app_state.jwt_verification_key, jwt).map_err(|e| {
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserServiceClaims {
    user_id: String,
    name_code: String,
    namespaces: Vec<String>,
}

fn verify_jwt(jwt_verification_key: &RS256PublicKey, jwt: String) -> eyre::Result<UserAuthContext> {
    let options = VerificationOptions {
        allowed_issuers: Some(HashSet::from(["user-service".to_string()])),
        allowed_audiences: Some(HashSet::from(["ossrh-proxy".to_string()])),

        ..Default::default()
    };

    let claims = jwt_verification_key
        .verify_token::<UserServiceClaims>(&jwt, Some(options))
        .map_err(|e| eyre::eyre!("JWT Verification error:\n{e:#}"))?;

    Ok(UserAuthContext {
        user_id: claims.custom.user_id,
        token_username: claims.custom.name_code,
        namespaces: claims.custom.namespaces,
        jwt,
    })
}
