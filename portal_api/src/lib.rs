use std::path::PathBuf;

use api_types::PublishingType;
use base64::engine::general_purpose::STANDARD;
use base64::engine::Engine;
use eyre::ContextCompat;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT},
    multipart::{Form, Part},
    Body, Client, ClientBuilder,
};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

pub mod api_types;

pub const CENTRAL_HOST: &str = "https://central.sonatype.com";

const API_ENDPOINT: &str = "/api/v1/publisher";
const UPLOAD_ENDPOINT: &str = "/upload";

pub struct Credentials {
    username: String,
    password: String,
}

impl Credentials {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    fn as_bearer_token(&self) -> String {
        let token = STANDARD.encode(format!("{}:{}", self.username, self.password));
        format!("UserToken {token}")
    }
}

/// The client for publishing via the Central Publisher Portal
pub struct PortalApiClient {
    client: Client,
    host: String,
}

impl PortalApiClient {
    /// Publish to Maven Central
    ///
    /// Provide [Credentials] to publish via a generated token.
    pub fn central_client(credentials: Credentials) -> eyre::Result<Self> {
        Self::client(CENTRAL_HOST.to_string(), credentials)
    }

    /// Publish to a compatible server
    ///
    /// Publish to an arbitrary server that implements the same API as Maven Central. Provide the host URL and generated
    /// [Credentials].
    pub fn client(host: String, credentials: Credentials) -> eyre::Result<Self> {
        let mut default_headers = HeaderMap::new();

        let user_agent_header =
            HeaderValue::from_str(&format!("portal_api client ({})", env!("CARGO_PKG_NAME")))?;
        default_headers.insert(USER_AGENT, user_agent_header);

        add_credentials_to_headers(&mut default_headers, &credentials)?;

        let client = ClientBuilder::default()
            .default_headers(default_headers)
            .build()?;

        Ok(Self { client, host })
    }

    #[tracing::instrument(skip(self))]
    pub async fn upload(
        &mut self,
        deployment_name: &str,
        publishing_type: PublishingType,
        upload_bundle_path: &PathBuf,
    ) -> eyre::Result<()> {
        let url = format!("{}{API_ENDPOINT}{UPLOAD_ENDPOINT}", self.host);
        tracing::trace!("Upload request to {url} - Started");

        let file = File::open(upload_bundle_path).await?;
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = Body::wrap_stream(stream);
        let file_name = upload_bundle_path
            .file_name()
            .wrap_err("Expected a valid filename")?
            .to_string_lossy()
            .to_string();
        let part = Part::stream(body)
            .file_name(file_name)
            .mime_str("application/octet-stream")?;
        let bundle = Form::new().part("bundle", part);

        let response = self
            .client
            .post(&url)
            .query(&[("name", deployment_name)])
            .query(&[("publishingType", publishing_type)])
            .multipart(bundle)
            .send()
            .await?;

        tracing::trace!("Got response: {:?}", response);
        if response.status().is_success() {
            tracing::info!("Upload request succeded");
            let deployment_id = response.text().await?;
            println!("Deployment ID: {deployment_id}");
        } else {
            tracing::debug!("Response body: {:?}", response.text().await?);
        }
        tracing::trace!("Upload request to {url} - Complete");

        Ok(())
    }
}

fn add_credentials_to_headers(
    headers: &mut HeaderMap,
    credentials: &Credentials,
) -> eyre::Result<()> {
    let token_header = HeaderValue::from_str(&credentials.as_bearer_token())?;
    headers.insert(AUTHORIZATION, token_header);

    Ok(())
}
