// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use std::{collections::HashSet, path::Path};

use jwt_simple::{
    algorithms::{RS256PublicKey, RSAPublicKeyLike},
    common::VerificationOptions,
    token::Token,
};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tracing::{instrument, Level};

use crate::{
    errors::{JwtKeyLoadError, JwtVerificationError},
    AsBearerAuthHeader,
};

#[derive(Clone)]
pub struct UserAuthContext {
    pub user_id: String,
    pub token_username: String,
    pub namespaces: Vec<String>,
    jwt: String,
}

impl AsBearerAuthHeader for UserAuthContext {
    fn as_bearer_auth_header(&self) -> String {
        format!("Bearer {}", self.jwt)
    }
}

pub struct JwtVerifier {
    public_key: RS256PublicKey,
}

impl JwtVerifier {
    #[instrument]
    pub async fn from_key_file(path: &Path) -> Result<Self, JwtKeyLoadError> {
        let jwt_public_key_file = File::open(path).await?;
        let mut jwt_public_key_reader = BufReader::new(jwt_public_key_file);
        let mut jwt_public_key = String::new();
        jwt_public_key_reader
            .read_to_string(&mut jwt_public_key)
            .await?;

        let public_key = RS256PublicKey::from_pem(&jwt_public_key)?;
        tracing::debug!("Loaded the JWT verification key");

        Ok(Self { public_key })
    }

    #[instrument(skip(self, jwt))]
    pub fn verify_jwt(&self, jwt: String) -> Result<UserAuthContext, JwtVerificationError> {
        if tracing::enabled!(Level::TRACE) {
            match Token::decode_metadata(&jwt) {
                Ok(metadata) => {
                    tracing::trace!(metadata = ?metadata, "JWT decoded")
                }
                Err(e) => tracing::error!("Failed to decode metadata: {e:#}"),
            }
        }

        let options = VerificationOptions {
            allowed_issuers: Some(HashSet::from(["user-service".to_string()])),
            allowed_audiences: Some(HashSet::from(["ossrh-proxy".to_string()])),

            ..Default::default()
        };

        let claims = self
            .public_key
            .verify_token::<UserServiceClaims>(&jwt, Some(options))?;

        Ok(UserAuthContext {
            user_id: claims.custom.user_id,
            token_username: claims.custom.name_code,
            namespaces: claims.custom.namespaces,
            jwt,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserServiceClaims {
    user_id: String,
    name_code: String,
    namespaces: Vec<String>,
}
