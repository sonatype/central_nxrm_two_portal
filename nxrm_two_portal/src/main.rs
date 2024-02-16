use axum::{extract::Request, http::StatusCode, routing::get, Router};
use tokio::net::TcpListener;
use tracing::instrument;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let app = Router::new()
        .route("/service/local/status", get(status))
        .fallback(fallback);

    let listener = TcpListener::bind("0.0.0.0:2727").await?;

    axum::serve(listener, app).await?;

    Ok(())
}

#[instrument(skip(request))]
async fn fallback(request: Request) -> (StatusCode, String) {
    tracing::debug!("Request to {}: {}", request.method(), request.uri());
    tracing::trace!("Headers: {:?}", request.headers());
    let bytes = axum::body::to_bytes(request.into_body(), usize::MAX).await;
    match bytes {
        Ok(bytes) => {
            tracing::trace!("Body: {:?}", bytes);
        }
        Err(e) => {
            tracing::error!("Failed to retrieve the body: {e:?}");
        }
    }

    (StatusCode::NOT_FOUND, "New method identified".to_string())
}

#[instrument]
async fn status() -> (StatusCode, String) {
    (StatusCode::OK, "Maybe it doesn't read the XML".to_string())
}
