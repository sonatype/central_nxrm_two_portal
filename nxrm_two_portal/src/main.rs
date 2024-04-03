use auth::auth;
use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use tokio::net::TcpListener;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use repository::local_repository::LocalRepository;

mod auth;
mod endpoints;
mod errors;
mod extract;
mod state;

use endpoints::{
    fallback::fallback,
    staging::{
        staging_bulk_promote, staging_deploy_by_repository_id, staging_deploy_by_repository_id_get,
        staging_profile_evaluate_endpoint, staging_profiles_endpoint,
        staging_profiles_finish_endpoint, staging_profiles_start_endpoint, staging_repository,
    },
    status::status_endpoint,
};
use state::AppState;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let app_state = AppState::new(LocalRepository::new()?);

    let staging_endpoints = Router::new()
        .route("/profile_evaluate", get(staging_profile_evaluate_endpoint))
        .route("/profiles/:profile_id", get(staging_profiles_endpoint))
        .route(
            "/profiles/:profile_id/start",
            post(staging_profiles_start_endpoint),
        )
        .route(
            "/deployByRepositoryId/:staging_repository_id/*file_path",
            put(staging_deploy_by_repository_id).get(staging_deploy_by_repository_id_get),
        )
        .route(
            "/profiles/:profile_id/finish",
            post(staging_profiles_finish_endpoint),
        )
        .route("/repository/:repository_id", get(staging_repository))
        .route("/bulk/promote", post(staging_bulk_promote))
        .route_layer(middleware::from_fn(auth));

    let app = Router::new()
        .route("/service/local/status", get(status_endpoint))
        .nest("/service/local/staging", staging_endpoints)
        .with_state(app_state)
        .fallback(fallback);

    let listener = TcpListener::bind("0.0.0.0:2727").await?;

    axum::serve(listener, app).await?;

    Ok(())
}
