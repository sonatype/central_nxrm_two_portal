// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use async_trait::async_trait;
use async_walkdir::{Filtering, WalkDir};
use bytes::Bytes;
use eyre::WrapErr;
use futures::{Stream, TryStreamExt};
use path_absolutize::Absolutize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use temp_dir::TempDir;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use tokio::{fs::File, io::BufWriter};
use tokio_util::io::StreamReader;
use tracing::instrument;

use crate::traits::{Repository, RepositoryKey, RepositoryState, ZipFile, NO_PROFILE};

const REPOSITORY_FOLDER: &str = "repository_contents";
const REPOSITORY_STATE_FILE: &str = "repository_state";

pub struct LocalRepository {
    root: TempDir,
    repository_indexes: RwLock<HashMap<String, u32>>,
}

impl LocalRepository {
    pub fn new() -> eyre::Result<Self> {
        let root = TempDir::with_prefix("local-repository")?;

        let repository_indexes = RwLock::new(HashMap::new());

        tracing::debug!("Created new local repository: {:?}", root.path());

        Ok(Self {
            root,
            repository_indexes,
        })
    }

    async fn retrieve_new_index(
        &self,
        user_id: &str,
        ip_addr: &IpAddr,
        profile_id: &str,
    ) -> eyre::Result<u32> {
        let repository_index_key = create_repository_index_key(user_id, ip_addr, profile_id);
        let mut repository_indexes = self.repository_indexes.write().await;
        let repository_index = repository_indexes
            .entry(repository_index_key.to_string())
            .and_modify(|i| *i += 1)
            .or_insert(0);

        Ok(repository_index.to_owned())
    }

    async fn retrieve_current_no_profile_index(
        &self,
        user_id: &str,
        ip_addr: &IpAddr,
    ) -> eyre::Result<u32> {
        let repository_index_key = create_repository_index_key(user_id, ip_addr, NO_PROFILE);
        let mut repository_indexes = self.repository_indexes.write().await;
        let repository_index = repository_indexes
            .entry(repository_index_key.to_string())
            .or_insert(0);

        Ok(repository_index.to_owned())
    }

    async fn validate_repository(&self, repository_key: &RepositoryKey) -> eyre::Result<()> {
        let repository_indexes = self.repository_indexes.read().await;
        let repository_index_key = create_repository_index_key(
            &repository_key.user_id,
            &repository_key.ip_addr,
            &repository_key.get_profile_id(),
        );
        let max_index = repository_indexes.get(&repository_index_key);
        match max_index {
            None => eyre::bail!("Repository {repository_key} does not exist"),
            Some(max_index) if max_index < &repository_key.repository_index => {
                eyre::bail!("Repository {repository_key} larger than {max_index}")
            }
            Some(max_index) => tracing::trace!("Repository {repository_key} is <= {max_index}"),
        }

        Ok(())
    }

    fn absolute_path_for_repository(
        &self,
        repository_key: &RepositoryKey,
    ) -> eyre::Result<PathBuf> {
        let repository_file_path = repository_key_to_file_path(repository_key);
        let absolute_path = self
            .root
            .path()
            .join(repository_file_path)
            .join(REPOSITORY_FOLDER);
        let absolute_path = absolute_path
            .absolutize()
            .wrap_err_with(|| format!("Failed to canonicalize {absolute_path:?}"))?;

        if absolute_path.starts_with(self.root.path()) {
            Ok(absolute_path.into_owned())
        } else {
            Err(eyre::eyre!("Invalid repository: {repository_key}"))
        }
    }

    fn absolute_path_for_repository_state(
        &self,
        repository_key: &RepositoryKey,
    ) -> eyre::Result<PathBuf> {
        let repository_file_path = repository_key_to_file_path(repository_key);
        let absolute_path = self
            .root
            .path()
            .join(repository_file_path)
            .join(REPOSITORY_STATE_FILE);
        let absolute_path = absolute_path
            .absolutize()
            .wrap_err_with(|| format!("Failed to canonicalize {absolute_path:?}"))?;

        if absolute_path.starts_with(self.root.path()) {
            Ok(absolute_path.into_owned())
        } else {
            Err(eyre::eyre!("Invalid repository: {repository_key}"))
        }
    }

    fn validated_path_in_repository(
        &self,
        repository_key: &RepositoryKey,
        file_path: impl AsRef<Path>,
    ) -> eyre::Result<PathBuf> {
        let repository_root = self.absolute_path_for_repository(repository_key)?;
        let absolute_file_path = repository_root.join(&file_path);
        let absolute_file_path = absolute_file_path
            .absolutize()
            .wrap_err_with(|| format!("Failed to canonicalize {absolute_file_path:?}"))?;

        if absolute_file_path.starts_with(repository_root) {
            Ok(absolute_file_path.into_owned())
        } else {
            Err(eyre::eyre!(
                "Invalid path to upload: {}",
                file_path.as_ref().display()
            ))
        }
    }

    async fn write_repository_state(
        &self,
        repository_key: &RepositoryKey,
        repository_state: RepositoryState,
    ) -> eyre::Result<()> {
        let state_file_path = self.absolute_path_for_repository_state(repository_key)?;
        let mut state_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(state_file_path)
            .await?;

        state_file
            .write_all(repository_state.to_string().as_bytes())
            .await?;

        Ok(())
    }

    async fn read_repository_state(
        &self,
        repository_key: &RepositoryKey,
    ) -> eyre::Result<RepositoryState> {
        let state_file_path = self.absolute_path_for_repository_state(repository_key)?;
        let mut state_file = File::open(state_file_path).await?;

        let mut state_string = String::new();
        state_file.read_to_string(&mut state_string).await?;

        let state: RepositoryState = state_string
            .trim()
            .try_into()
            .map_err(|e: String| eyre::eyre!(e))?;
        Ok(state)
    }
}

#[async_trait]
impl Repository for LocalRepository {
    #[instrument]
    async fn start(
        &self,
        user_id: &str,
        ip_addr: &IpAddr,
        profile_id: &str,
    ) -> eyre::Result<RepositoryKey> {
        let repository_index = self
            .retrieve_new_index(user_id, ip_addr, profile_id)
            .await?;
        let repository_key = RepositoryKey::new(
            user_id,
            ip_addr,
            Some(profile_id.to_string()),
            repository_index,
        );
        tracing::debug!("Starting repository: {}", repository_key);

        let path = self.absolute_path_for_repository(&repository_key)?;
        tokio::fs::create_dir_all(&path).await?;
        tracing::trace!("Created repository folders: {path:?}");

        self.write_repository_state(&repository_key, RepositoryState::Open)
            .await?;
        tracing::debug!("Opened the repository");

        Ok(repository_key)
    }

    #[instrument]
    async fn open_no_profile_repository(
        &self,
        user_id: &str,
        ip_addr: &IpAddr,
    ) -> eyre::Result<RepositoryKey> {
        let repository_index = self
            .retrieve_current_no_profile_index(user_id, ip_addr)
            .await?;
        let repository_key = RepositoryKey::new(user_id, ip_addr, None, repository_index);
        tracing::debug!("Opening repository: {}", repository_key);

        let path = self.absolute_path_for_repository(&repository_key)?;
        tokio::fs::create_dir_all(&path).await?;
        tracing::trace!("Created repository folders: {path:?}");

        self.write_repository_state(&repository_key, RepositoryState::Open)
            .await?;
        tracing::debug!("Opened the repository");

        Ok(repository_key)
    }

    /// Correctness: Assumes a well-behaved plugin that is not attempting to upload the same file concurrently
    #[instrument(skip(file_contents))]
    async fn add_file<P, S>(
        &self,
        repository_key: &RepositoryKey,
        file_path: P,
        file_contents: S,
    ) -> eyre::Result<()>
    where
        P: AsRef<Path> + Debug + Send,
        S: Stream<Item = eyre::Result<Bytes>> + Send,
    {
        tracing::debug!("Adding file to repository: {repository_key}");
        self.validate_repository(repository_key).await?;
        let file_path = self.validated_path_in_repository(repository_key, file_path)?;
        let parent = file_path
            .parent()
            .ok_or_else(|| eyre::eyre!("No parent folder found for {file_path:?}"))?;

        tokio::fs::create_dir_all(parent).await?;
        tracing::trace!("Created repository folders: {file_path:?}");

        // Adapted from the Tokio examples
        async {
            let body_with_io_error =
                file_contents.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
            let body_reader = StreamReader::new(body_with_io_error);
            futures::pin_mut!(body_reader);

            let mut file = BufWriter::new(File::create(&file_path).await?);

            tokio::io::copy(&mut body_reader, &mut file).await?;

            Ok::<_, io::Error>(())
        }
        .await?;

        tracing::trace!("File written to: {file_path:?}");
        Ok(())
    }

    #[instrument]
    async fn finish(&self, repository_key: &RepositoryKey) -> eyre::Result<ZipFile> {
        tracing::debug!("Finishing repository");
        self.validate_repository(repository_key).await?;
        let path = self.absolute_path_for_repository(repository_key)?;
        // create the zip file from all of the existing files
        let mut zip_file = ZipFile::in_memory();

        let mut entries = WalkDir::new(&path).filter(|entry| async move {
            if let Ok(file_type) = entry.file_type().await {
                if !file_type.is_dir() {
                    return Filtering::Continue;
                }
            } else {
                tracing::error!("Encountered error reading file entry: {:?}", entry.path());
            }
            Filtering::Ignore
        });

        while let Some(entry) = entries.try_next().await? {
            let entry_path = entry.path();
            tracing::trace!("Adding file to .zip: {entry_path:?}");
            let relative_path = entry_path.strip_prefix(&path)?;
            let file_to_add = File::open(&entry_path).await?;
            zip_file.add_file(relative_path, file_to_add).await?;
        }

        tracing::debug!("Created .zip file for repository");

        // delete the repository folder
        tokio::fs::remove_dir_all(&path).await?;
        tracing::debug!("Cleaned up the repository: {path:?}");

        self.write_repository_state(repository_key, RepositoryState::Closed)
            .await?;
        tracing::debug!("Closed the repository");

        Ok(zip_file)
    }

    #[instrument]
    async fn release(&self, repository_key: &RepositoryKey) -> eyre::Result<()> {
        tracing::debug!("Releasing repository");
        self.validate_repository(repository_key).await?;

        self.write_repository_state(repository_key, RepositoryState::Released)
            .await?;
        tracing::debug!("Released the repository");

        Ok(())
    }

    #[instrument]
    async fn get_state(&self, repository_key: &RepositoryKey) -> eyre::Result<RepositoryState> {
        tracing::debug!("Getting the state of repository");
        let valid = self.validate_repository(repository_key).await;
        if let Err(_error) = valid {
            return Ok(RepositoryState::NotFound);
        }

        let state = self.read_repository_state(repository_key).await?;

        Ok(state)
    }
}

impl std::fmt::Debug for LocalRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalRepository")
            .field("root", &self.root.path())
            .field("repository_versions", &"opaque")
            .finish()
    }
}

/// Convenience function to ensure consistent construction of file paths
fn repository_key_to_file_path(repository_key: &RepositoryKey) -> PathBuf {
    PathBuf::from(format!(
        "{}/{}/{}-{}/",
        repository_key.user_id,
        repository_key.ip_addr,
        repository_key.get_profile_id(),
        repository_key.repository_index
    ))
}

/// Convenience function to ensure consistent construction of repository index keys
fn create_repository_index_key(user_id: &str, ip_addr: &IpAddr, profile_id: &str) -> String {
    format!("{user_id}/{ip_addr}/{profile_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Cursor, Read},
        net::Ipv4Addr,
    };
    use zip::read::ZipArchive;

    #[tokio::test]
    async fn simple_end_to_end_test() -> eyre::Result<()> {
        let test_file_path = "com/example/file.txt";
        let test_file_contents = "test_file_content";
        let local_repository = LocalRepository::new()?;

        // start the repository
        let repository_key = local_repository
            .start(
                "test_user",
                &IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                "test_profile",
            )
            .await?;

        let file_contents = futures::stream::once(async { Ok(Bytes::from(test_file_contents)) });

        // add a file
        local_repository
            .add_file(&repository_key, test_file_path, file_contents)
            .await?;

        // finish the repository
        let zip_file = local_repository.finish(&repository_key).await?;
        let zip_contents = zip_file.as_buffer()?;

        // verifiy the zip
        let mut zip_reader = ZipArchive::new(Cursor::new(zip_contents))?;
        assert_eq!(
            zip_reader.file_names().collect::<Vec<&str>>(),
            vec![test_file_path]
        );
        let mut actual_file = zip_reader.by_name(test_file_path)?;
        let mut actual_content = String::new();
        actual_file.read_to_string(&mut actual_content)?;

        assert_eq!(&actual_content, test_file_contents);

        Ok(())
    }

    #[tokio::test]
    async fn reject_directory_traversal() -> eyre::Result<()> {
        let local_repository = LocalRepository::new()?;

        // start the repository
        let repository_key = local_repository
            .start(
                "test_user",
                &IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                "test_profile",
            )
            .await?;

        let file_contents = futures::stream::once(async { Ok(Bytes::from("test_file_content")) });

        // add a file
        if let Err(e) = local_repository
            .add_file(
                &repository_key,
                "../../other_test_user/other_test_repository/com/example/file.txt",
                file_contents,
            )
            .await
        {
            assert!(e.to_string().contains("Invalid path to upload"));
        } else {
            eyre::bail!("Failed to prevent directory traversal");
        }

        Ok(())
    }
}
