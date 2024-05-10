use portal_api::{api_types::PublishingType, PortalApiClient};
use repository::traits::{Repository, RepositoryKey};
use tracing::instrument;

use crate::auth::UserToken;

#[instrument(skip(portal_api_client, repository, user_token))]
pub async fn publish<R: Repository>(
    portal_api_client: &PortalApiClient,
    repository: &R,
    user_token: UserToken,
    repository_key: &RepositoryKey,
    publishing_type: PublishingType,
) -> eyre::Result<()> {
    let zip_data = repository.finish(&repository_key).await?;
    let zip_data = zip_data.as_buffer()?;

    let credentials = user_token.as_credentials();

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
