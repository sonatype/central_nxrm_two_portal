use std::path::PathBuf;

use api_types::PublishingType;
use eyre::ContextCompat;
use reqwest::{
    header::{HeaderMap, HeaderValue, USER_AGENT},
    multipart::{Form, Part},
    Body, Client, ClientBuilder,
};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use url::Url;

pub mod api_types;
pub mod credentials;

pub use credentials::Credentials;

pub const CENTRAL_HOST: &str = "https://central.sonatype.com";

const API_ENDPOINT: &str = "/api/v1/publisher/";
const UPLOAD_ENDPOINT: &str = "upload"; // relative to API_ENDPOINT

const UPLOAD_MIME_STR: &str = "application/octet-stream";

/// The client for publishing via the Central Publisher Portal
pub struct PortalApiClient {
    client: Client,
    host: Url,
}

impl PortalApiClient {
    /// Publish to Maven Central
    ///
    /// Provide [Credentials] to publish via a generated token.
    pub fn central_client() -> eyre::Result<Self> {
        Self::client(CENTRAL_HOST)
    }

    /// Publish to a compatible server
    ///
    /// Publish to an arbitrary server that implements the same API as Maven Central.
    pub fn client(host: &str) -> eyre::Result<Self> {
        let mut default_headers = HeaderMap::new();

        let user_agent_header =
            HeaderValue::from_str(&format!("portal_api client ({})", env!("CARGO_PKG_NAME")))?;
        default_headers.insert(USER_AGENT, user_agent_header);

        let client = ClientBuilder::default()
            .default_headers(default_headers)
            .build()?;

        let host = Url::parse(host)?;

        Ok(Self { client, host })
    }

    #[tracing::instrument(skip(self, credentials, upload_bundle_contents))]
    pub async fn upload_from_memory(
        &self,
        credentials: &Credentials,
        deployment_name: &str,
        publishing_type: PublishingType,
        upload_bundle_contents: Vec<u8>,
    ) -> eyre::Result<String> {
        let part = Part::bytes(upload_bundle_contents)
            .file_name("bundle.zip")
            .mime_str(UPLOAD_MIME_STR)?;

        let deployment_id = self
            .upload_part(credentials, deployment_name, publishing_type, part)
            .await?;

        Ok(deployment_id)
    }

    #[tracing::instrument(skip(self, credentials))]
    pub async fn upload_from_file(
        &self,
        credentials: &Credentials,
        deployment_name: &str,
        publishing_type: PublishingType,
        upload_bundle_path: &PathBuf,
    ) -> eyre::Result<String> {
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
            .mime_str(UPLOAD_MIME_STR)?;

        let deployment_id = self
            .upload_part(credentials, deployment_name, publishing_type, part)
            .await?;

        Ok(deployment_id)
    }

    #[tracing::instrument(skip(self, credentials, part))]
    async fn upload_part(
        &self,
        credentials: &Credentials,
        deployment_name: &str,
        publishing_type: PublishingType,
        part: Part,
    ) -> eyre::Result<String> {
        let url = self.host.join(API_ENDPOINT)?.join(UPLOAD_ENDPOINT)?;
        let url_display = url.clone().to_string();
        tracing::trace!("Upload request to {url_display} - Started");

        let bundle = Form::new().part("bundle", part);

        let request = self
            .client
            .post(url)
            .query(&[("name", deployment_name)])
            .query(&[("publishingType", publishing_type)])
            .multipart(bundle);
        let request = credentials.add_credentials_to_request(request)?;

        let response = request.send().await?;

        tracing::trace!("Got response: {:?}", response);
        let deployment_id = if response.status().is_success() {
            tracing::info!("Upload request succeeded");
            response.text().await?
        } else {
            tracing::debug!("Response body: {:?}", response.text().await?);
            eyre::bail!("Upload request failed");
        };
        tracing::trace!("Upload request to {url_display} - Complete");

        Ok(deployment_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, header, method, path, query_param};
    use wiremock::{Mock, MockBuilder, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn successful_upload() -> eyre::Result<()> {
        let mock_server = MockServer::start().await;

        common_test_expectations()
            .respond_with(ResponseTemplate::new(200).set_body_string("test_deployment_id"))
            .mount(&mock_server)
            .await;

        let mut client = PortalApiClient::client(&mock_server.uri())?;

        let deployment_id = client
            .upload_from_file(
                &Credentials::new("test_username".to_string(), "test_password".to_string()),
                "test_deployment",
                PublishingType::Automatic,
                &PathBuf::from("Cargo.toml"), // Don't bother with client side validation of the bundle
            )
            .await?;

        assert_eq!(deployment_id, "test_deployment_id");

        Ok(())
    }

    #[tokio::test]
    async fn failed_upload() -> eyre::Result<()> {
        let mock_server = MockServer::start().await;

        common_test_expectations()
            .respond_with(
                ResponseTemplate::new(500).set_body_string(r#"{"error": "example_error"}"#),
            )
            .mount(&mock_server)
            .await;

        let mut client = PortalApiClient::client(&mock_server.uri())?;

        let error = client
            .upload_from_file(
                &Credentials::new("test_username".to_string(), "test_password".to_string()),
                "test_deployment",
                PublishingType::Automatic,
                &PathBuf::from("Cargo.toml"), // Don't bother with client side validation of the bundle
            )
            .await
            .expect_err("Succeeded, incorrectly");

        assert!(error.to_string().contains("Upload request failed"));

        Ok(())
    }

    fn common_test_expectations() -> MockBuilder {
        Mock::given(method("POST"))
            .and(path("/api/v1/publisher/upload"))
            .and(header(
                "Authorization",
                "UserToken dGVzdF91c2VybmFtZTp0ZXN0X3Bhc3N3b3Jk",
            ))
            .and(query_param("name", "test_deployment"))
            .and(query_param("publishingType", "AUTOMATIC"))
            // expect the contents of the Cargo.toml file (as a stand-in for the bundle)
            .and(body_string_contains("portal_api"))
            // expect a multipart form field named "bundle" with a filename
            .and(body_string_contains(
                r#"form-data; name="bundle"; filename="Cargo.toml""#,
            ))
    }
}
