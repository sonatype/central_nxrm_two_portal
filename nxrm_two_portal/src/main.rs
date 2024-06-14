use std::net::SocketAddr;

use auth::auth;
use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use portal_api::PortalApiClient;
use tokio::net::TcpListener;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use repository::local_repository::LocalRepository;

mod auth;
mod config;
mod endpoints;
mod errors;
mod extract;
mod publish;
mod state;

use config::AppConfig;
use endpoints::{
    fallback::fallback,
    manual::manual_upload_default_repository,
    staging::{
        staging_bulk_close, staging_bulk_promote, staging_deploy_by_repository_id,
        staging_deploy_by_repository_id_get, staging_deploy_maven2, staging_deploy_maven2_get,
        staging_profile_evaluate_endpoint, staging_profiles_endpoint,
        staging_profiles_finish_endpoint, staging_profiles_list_endpoint,
        staging_profiles_start_endpoint, staging_repository,
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

    let app_config = AppConfig::load()?;
    tracing::debug!("Loaded configuration: {app_config:?}");

    let local_repository = LocalRepository::new()?;
    tracing::debug!("Initialized a local repository");

    let portal_api_client = PortalApiClient::client(&app_config.central_url)?;
    tracing::debug!("Initialized a Portal API client");

    let app_state = AppState::new(local_repository, portal_api_client);

    let staging_endpoints = Router::new()
        .route("/profile_evaluate", get(staging_profile_evaluate_endpoint))
        .route("/profiles", get(staging_profiles_list_endpoint))
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
        .route("/bulk/close", post(staging_bulk_close))
        .route("/bulk/promote", post(staging_bulk_promote))
        // required for Gradle maven-publish plugin
        .route(
            "/deploy/maven2/*file_path",
            put(staging_deploy_maven2).get(staging_deploy_maven2_get),
        )
        .route_layer(middleware::from_fn(auth));

    let manual_endpoints = Router::new()
        .route("/upload", post(manual_upload_default_repository))
        .route_layer(middleware::from_fn(auth));

    let app = Router::new()
        .route("/service/local/status", get(status_endpoint))
        .nest("/service/local/staging", staging_endpoints)
        .nest("/manual", manual_endpoints)
        .with_state(app_state)
        .fallback(fallback);

    tracing::info!("Listening on port: {}", app_config.app_port);
    let listener = TcpListener::bind(format!("0.0.0.0:{}", app_config.app_port)).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
