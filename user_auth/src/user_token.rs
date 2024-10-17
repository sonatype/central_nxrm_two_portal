// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use base64::engine::general_purpose::STANDARD;
use base64::prelude::{Engine, BASE64_STANDARD};

use crate::errors::UserTokenError;
use crate::AsBearerAuthHeader;

pub struct UserToken {
    pub token_username: String,
    token_password: String,
}

impl UserToken {
    pub fn new(token_username: String, token_password: String) -> Self {
        Self {
            token_username,
            token_password,
        }
    }

    pub fn from_token(token: &str) -> Result<Self, UserTokenError> {
        let token = BASE64_STANDARD.decode(token)?;
        let token = String::from_utf8(token)?;
        let (token_username, token_password) =
            token.split_once(':').ok_or(UserTokenError::InvalidHeader)?;

        Ok(Self {
            token_username: token_username.to_string(),
            token_password: token_password.to_string(),
        })
    }
}

impl AsBearerAuthHeader for UserToken {
    fn as_bearer_auth_header(&self) -> String {
        let encoded = STANDARD.encode(format!("{}:{}", self.token_username, self.token_password));
        format!("Bearer {encoded}")
    }
}
