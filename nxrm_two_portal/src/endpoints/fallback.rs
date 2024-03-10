use axum::{extract::Request, http::StatusCode};
use tracing::instrument;

#[instrument(skip(request))]
pub(crate) async fn fallback(request: Request) -> (StatusCode, String) {
    tracing::error!("Request to {}: {}", request.method(), request.uri());
    tracing::trace!("Headers: {:#?}", request.headers());
    tracing::trace!("Authority: {:#?}", request.uri().authority());
    let bytes = axum::body::to_bytes(request.into_body(), usize::MAX).await;
    match bytes {
        Ok(bytes) => {
            tracing::trace!("Body: {:?}", bytes);
        }
        Err(e) => {
            tracing::error!("Failed to retrieve the body: {e:?}");
        }
    }

    (
        StatusCode::UNAUTHORIZED,
        "New method identified".to_string(),
    )
}
