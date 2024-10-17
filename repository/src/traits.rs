// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use async_trait::async_trait;
use bytes::Bytes;
use eyre::WrapErr;
use futures::Stream;
use std::{
    fmt::{Debug, Display},
    io::{Cursor, Write},
    net::IpAddr,
    path::Path,
};
use tokio::{fs::File, io::AsyncReadExt};
use zip::{write::SimpleFileOptions, ZipWriter};

/// A constant for deployments that do not provide a profile
pub const NO_PROFILE: &str = "no-profile";

/// A trait to define the actions of NXRM2 staging repositories
///
/// Defined abstractly in order to allow multiple implementations that can be swapped out.
#[async_trait]
pub trait Repository {
    /// Open a new repository for the provided user and profile
    ///
    /// The profile_id value should be the namespace that was requested for publishing.
    async fn start(
        &self,
        user_id: &str,
        ip_addr: &IpAddr,
        profile_id: &str,
    ) -> eyre::Result<RepositoryKey>;

    /// Opens a new repository without a provided profile
    ///
    /// This mimics the behavior of the general file upload of NXRM2.
    /// Repositories are differentiated soley based on user ID and IP address.
    async fn open_no_profile_repository(
        &self,
        user_id: &str,
        ip_addr: &IpAddr,
    ) -> eyre::Result<RepositoryKey>;

    async fn add_file<P, S>(
        &self,
        authorized_namespaces: &[String],
        repository_key: &RepositoryKey,
        file_path: P,
        file_contents: S,
    ) -> eyre::Result<()>
    where
        P: AsRef<Path> + Debug + Send,
        S: Stream<Item = eyre::Result<Bytes>> + Send;

    async fn finish(&self, repository_key: &RepositoryKey) -> eyre::Result<ZipFile>;

    async fn release(&self, repository_key: &RepositoryKey) -> eyre::Result<()>;

    async fn get_state(&self, repository_key: &RepositoryKey) -> eyre::Result<RepositoryState>;
}

#[derive(Debug, PartialEq)]
pub struct RepositoryKey {
    pub user_id: String,
    pub ip_addr: IpAddr,
    profile_id: Option<String>,
    pub repository_index: u32,
}

impl RepositoryKey {
    pub fn new(
        user_id: &str,
        ip_addr: &IpAddr,
        profile_id: Option<String>,
        repository_index: u32,
    ) -> Self {
        Self {
            user_id: user_id.to_string(),
            ip_addr: ip_addr.to_owned(),
            profile_id,
            repository_index,
        }
    }

    pub fn get_repository_id(&self) -> String {
        format!("{}-{}", self.get_profile_id(), self.repository_index)
    }

    /// Convenience function to translate the `repository_index` into its component parts
    ///
    /// API calls operate on the user's repository once it has been opened, rather than the profile
    pub fn from_user_context_and_repository_id(
        user_id: &str,
        ip_addr: &IpAddr,
        repository_id: &str,
    ) -> eyre::Result<Self> {
        if let Some((profile_id, repository_index)) = repository_id.rsplit_once('-') {
            let repository_index: u32 = repository_index.parse()?;
            Ok(Self::new(
                user_id,
                ip_addr,
                Some(profile_id.to_string()),
                repository_index,
            ))
        } else {
            Err(eyre::eyre!("Invalid repository_id: {repository_id}"))
        }
    }

    pub fn get_profile_id(&self) -> String {
        self.profile_id
            .clone()
            .unwrap_or_else(|| NO_PROFILE.to_string())
    }
}

impl Display for RepositoryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{}-{}",
            self.user_id,
            self.ip_addr,
            self.get_profile_id(),
            self.repository_index
        )
    }
}

pub enum RepositoryState {
    Open,
    Closed,
    Released,
    NotFound,
}

impl Display for RepositoryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state_display = match self {
            RepositoryState::Open => "open",
            RepositoryState::Closed => "closed",
            RepositoryState::Released => "released",
            RepositoryState::NotFound => "not_found",
        };
        write!(f, "{state_display}")
    }
}

impl TryFrom<&str> for RepositoryState {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "open" => Ok(RepositoryState::Open),
            "closed" => Ok(RepositoryState::Closed),
            "released" => Ok(RepositoryState::Released),
            "not_found" => Ok(RepositoryState::NotFound),
            other => Err(format!("Could not convert {other} into a RepositoryState")),
        }
    }
}

/// Convenience wrapper for the API
pub struct ZipFile {
    writer: ZipWriter<Cursor<Vec<u8>>>,
}

impl ZipFile {
    pub fn in_memory() -> Self {
        let writer = ZipWriter::new(Cursor::new(Vec::new()));
        Self { writer }
    }

    pub async fn add_file(
        &mut self,
        relative_path: impl AsRef<Path>,
        mut file: File,
    ) -> eyre::Result<()> {
        let relative_path = relative_path.as_ref().display().to_string();
        tracing::trace!("Adding file to .zip: {relative_path}");
        self.writer
            .start_file(relative_path, SimpleFileOptions::default())?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .await
            .wrap_err("Failed to read file: {relative_path}")?;
        self.writer
            .write_all(&contents)
            .wrap_err("Failed to add file contents to .zip")?;

        Ok(())
    }

    pub fn as_buffer(self) -> eyre::Result<Vec<u8>> {
        let cursor = self.writer.finish().wrap_err("Failed to write zip file")?;

        Ok(cursor.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    #[test]
    fn valid_repository_key_from_valid_repository_id() -> eyre::Result<()> {
        let actual_repository_key = RepositoryKey::from_user_context_and_repository_id(
            "user",
            &IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            "profile-1",
        )?;
        let expected_repository_key = RepositoryKey::new(
            "user",
            &IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            Some("profile".to_string()),
            1,
        );

        assert_eq!(actual_repository_key, expected_repository_key);
        Ok(())
    }
}
