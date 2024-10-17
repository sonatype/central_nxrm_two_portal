// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum JwtKeyLoadError {
    #[error("Failed to read file")]
    FileReadError(#[from] std::io::Error),

    #[error("Failed to process file as a key")]
    ReadKeyError(#[from] jwt_simple::Error),
}

#[derive(Debug, Error)]
pub enum JwtVerificationError {
    #[error("Failed to verify the JWT: {0:#}")]
    VerificationFailed(#[from] jwt_simple::Error),
}

#[derive(Debug, Error)]
pub enum UserTokenError {
    #[error("Failed to extract user token from header")]
    InvalidHeader,

    #[error("Token was not Base64 encoded")]
    Base64Error(#[from] base64::DecodeError),

    #[error("Encoded value was not UTF-8")]
    UTF8Error(#[from] std::string::FromUtf8Error),
}
