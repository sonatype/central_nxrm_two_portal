// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use base64::engine::general_purpose::STANDARD;
use base64::engine::Engine;
use reqwest::{
    header::{HeaderValue, AUTHORIZATION},
    RequestBuilder,
};

pub enum Credentials {
    UserToken { username: String, password: String },
    Jwt { token: String },
}

impl Credentials {
    pub fn from_usertoken(username: String, password: String) -> Self {
        Self::UserToken { username, password }
    }

    pub fn from_jwt(token: String) -> Self {
        Self::Jwt { token }
    }

    pub fn add_credentials_to_request(
        &self,
        request: RequestBuilder,
    ) -> eyre::Result<RequestBuilder> {
        let token_header = HeaderValue::from_str(&self.as_bearer_token())?;
        let request = request.header(AUTHORIZATION, token_header);
        tracing::trace!("Added {AUTHORIZATION} header");
        Ok(request)
    }

    fn as_bearer_token(&self) -> String {
        let token = match self {
            Self::UserToken { username, password } => {
                STANDARD.encode(format!("{username}:{password}"))
            }
            Self::Jwt { token } => token.to_owned(),
        };
        format!("Bearer {token}")
    }
}
