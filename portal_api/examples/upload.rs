use clap::Parser;
use std::path::PathBuf;

use portal_api::{
    api_types::PublishingType::Automatic, Credentials, PortalApiClient, CENTRAL_HOST,
};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// An example CLI to upload bundles to Central
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Central URL
    #[arg(short, long)]
    central_host: Option<String>,

    /// The username of the credentials (if not provided, it will be prompted)
    #[arg(short, long)]
    username: Option<String>,

    /// The deployment name (if not provided, defaults to "Upload")
    #[arg(short, long)]
    deployment_name: Option<String>,

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

    let credentials = Credentials::new(username, password);

    let mut api_client = PortalApiClient::client(host, credentials)?;

    let deployment_name = cli.deployment_name.unwrap_or("Upload".to_string());

    api_client
        .upload(&deployment_name, Automatic, &cli.upload_bundle)
        .await?;

    Ok(())
}
