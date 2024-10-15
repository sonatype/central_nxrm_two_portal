// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::prelude::{Engine, BASE64_STANDARD};
use eyre::{bail, OptionExt};
use portal_api::Credentials;
use tracing::instrument;

#[derive(Clone)]
pub struct UserToken {
    pub token_username: String,
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

#[instrument(skip(req, next))]
pub async fn auth(mut req: Request, next: Next) -> Result<Response, StatusCode> {
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
    req.extensions_mut().insert(user_token);
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
