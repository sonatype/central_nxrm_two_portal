use async_trait::async_trait;
use bytes::Bytes;
use eyre::WrapErr;
use futures::Stream;
use std::{
    fmt::{Debug, Display},
    io::{Cursor, Write},
    path::Path,
};
use tokio::{fs::File, io::AsyncReadExt};
use zip::{write::FileOptions, ZipWriter};

/// A trait to define the actions of NXRM2 staging repositories
///
/// Defined abstractly in order to allow multiple implementations that can be swapped out.
#[async_trait]
pub trait Repository {
    /// Open a new repository for the provided user and profile
    ///
    /// The profile_id value should be the namespace that was requested for publishing.
    async fn start(&self, user_id: &str, profile_id: &str) -> eyre::Result<RepositoryKey>;

    async fn add_file<P, S>(
        &self,
        repository_key: &RepositoryKey,
        file_path: P,
        file_contents: S,
    ) -> eyre::Result<()>
    where
        P: AsRef<Path> + Debug + Send,
        S: Stream<Item = eyre::Result<Bytes>> + Send;

    async fn finish(&self, repository_key: &RepositoryKey) -> eyre::Result<ZipFile>;
}

///
#[derive(Debug, PartialEq)]
pub struct RepositoryKey {
    pub user_id: String,
    pub profile_id: String,
    pub repository_index: u32,
}

impl RepositoryKey {
    pub fn new(user_id: &str, profile_id: &str, repository_index: u32) -> Self {
        Self {
            user_id: user_id.to_string(),
            profile_id: profile_id.to_string(),
            repository_index,
        }
    }

    /// Convenience function to translate the `repository_index` into its component parts
    ///
    /// API calls operate on the user's repository once it has been opened, rather than the profile
    pub fn from_user_id_and_repository_id(
        user_id: &str,
        repository_id: &str,
    ) -> eyre::Result<Self> {
        if let Some((profile_id, repository_index)) = repository_id.rsplit_once('-') {
            let repository_index: u32 = repository_index.parse()?;
            Ok(Self::new(user_id, profile_id, repository_index))
        } else {
            Err(eyre::eyre!("Invalid repository_id: {repository_id}"))
        }
    }
}

impl Display for RepositoryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}-{}",
            self.user_id, self.profile_id, self.repository_index
        )
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
            .start_file(&relative_path, FileOptions::default())?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .await
            .wrap_err("Failed to read file: {relative_path}")?;
        self.writer
            .write_all(&contents)
            .wrap_err("Failed to add file contents to .zip")?;

        Ok(())
    }

    pub fn as_buffer(mut self) -> eyre::Result<Vec<u8>> {
        let cursor = self.writer.finish().wrap_err("Failed to write zip file")?;

        Ok(cursor.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_repository_key_from_valid_repository_id() -> eyre::Result<()> {
        let actual_repository_key =
            RepositoryKey::from_user_id_and_repository_id("user", "profile-1")?;
        let expected_repository_key = RepositoryKey::new("user", "profile", 1);

        assert_eq!(actual_repository_key, expected_repository_key);
        Ok(())
    }
}
