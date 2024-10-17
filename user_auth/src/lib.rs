// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

pub mod errors;
pub mod jwt;
pub mod user_token;

pub trait AsBearerAuthHeader {
    fn as_bearer_auth_header(&self) -> String;
}
