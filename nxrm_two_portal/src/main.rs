use axum::{
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod endpoints;
mod errors;
mod extract;

use endpoints::{
    fallback::fallback,
    staging::{
        staging_profile_evaluate_endpoint, staging_profiles_endpoint,
        staging_profiles_start_endpoint,
    },
    status::status_endpoint,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let staging_endpoints = Router::new()
        .route("/profile_evaluate", get(staging_profile_evaluate_endpoint))
        .route("/profiles/:profile_id", get(staging_profiles_endpoint))
        .route(
            "/profiles/:profile_id/start",
            post(staging_profiles_start_endpoint),
        );

    let app = Router::new()
        .route("/service/local/status", get(status_endpoint))
        .nest("/service/local/staging", staging_endpoints)
        .fallback(fallback);

    let listener = TcpListener::bind("0.0.0.0:2727").await?;

    axum::serve(listener, app).await?;

    Ok(())
}
