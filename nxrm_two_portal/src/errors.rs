use axum::{http::StatusCode, response::IntoResponse};

pub(crate) struct ApiError(pub(crate) eyre::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        tracing::debug!("Returning error to client: {}", self.0);
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to process request: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for ApiError
where
    E: Into<eyre::Error>,
{
    fn from(value: E) -> Self {
        ApiError(value.into())
    }
}
