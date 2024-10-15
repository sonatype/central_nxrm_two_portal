// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use clap::Parser;
use std::{path::PathBuf, sync::Arc};

use portal_api::{api_types::PublishingType, Credentials, PortalApiClient, CENTRAL_HOST};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// An example CLI to upload bundles to Central
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Central URL
    #[arg(long)]
    central_host: Option<String>,

    /// The username of the credentials (if not provided, it will be prompted)
    #[arg(short, long)]
    username: Option<String>,

    /// The deployment name (if not provided, defaults to "Stress Test")
    #[arg(short, long)]
    deployment_name: Option<String>,

    /// The number of copies of the bundle to upload
    #[arg(short, long, default_value_t = 10)]
    count: u32,

    /// The path to a .zip/.tgz/etc. to upload
    upload_bundle: PathBuf,
}

#[tokio::main]
pub async fn main() -> eyre::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let host = cli.central_host.unwrap_or(CENTRAL_HOST.to_string());

    let username = match cli.username {
        Some(username) => username.to_string(),
        None => promptly::prompt("Publisher token username")?,
    };

    let password = rpassword::prompt_password("Publisher token password: ")?;

    let credentials = Arc::new(Credentials::from_usertoken(username, password));

    let api_client = Arc::new(PortalApiClient::client(&host)?);

    let deployment_name = cli.deployment_name.unwrap_or("Upload".to_string());

    let mut handles = Vec::with_capacity(cli.count as usize);

    println!(
        "Started a stress test of {host} with an upload of {} copies of {}",
        cli.count,
        cli.upload_bundle.to_string_lossy()
    );

    for i in 0..cli.count {
        tracing::debug!("Starting request {i}");

        let api_client = api_client.clone();
        let credentials = credentials.clone();
        let deployment_name = format!("{deployment_name} ({i})");
        let upload_bundle = cli.upload_bundle.clone();

        handles.push(tokio::spawn(async move {
            api_client
                .upload_from_file(
                    &credentials,
                    &deployment_name,
                    PublishingType::UserManaged,
                    &upload_bundle,
                )
                .await
        }));

        tracing::debug!("Completed request {i}");
    }

    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await;

        match result {
            Ok(Ok(deployment_id)) => {
                tracing::debug!("Request {i} returned deployment ID: {deployment_id}")
            }
            Ok(Err(e)) => tracing::error!("Request task {i} failed with error: {e:?}"),
            Err(e) => tracing::error!("Request {i} failed with error: {e:?}"),
        }
    }

    println!(
        "Completed a stress test of {host} with an upload of {} copies of {}",
        cli.count,
        cli.upload_bundle.to_string_lossy()
    );

    Ok(())
}
