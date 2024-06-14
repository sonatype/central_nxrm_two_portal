use portal_api::{api_types::PublishingType, Credentials, PortalApiClient};
use repository::traits::{Repository, RepositoryKey};
use tracing::instrument;

#[instrument(skip(portal_api_client, repository, credentials))]
pub async fn publish<R: Repository>(
    portal_api_client: &PortalApiClient,
    repository: &R,
    credentials: &Credentials,
    repository_key: &RepositoryKey,
    publishing_type: PublishingType,
) -> eyre::Result<()> {
    let zip_data = repository.finish(&repository_key).await?;
    let zip_data = zip_data.as_buffer()?;

    portal_api_client
        .upload_from_memory(
            &credentials,
            &format!(
                "{} (via OSSRH API Proxy)",
                repository_key.get_repository_id()
            ),
            publishing_type,
            zip_data,
        )
        .await?;
    Ok(())
}
