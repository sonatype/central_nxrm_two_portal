use async_trait::async_trait;
use axum::body::Body;
use axum::body::Bytes;
use axum::extract::FromRequest;
use axum::http::header;
use axum::http::header::CONTENT_TYPE;
use axum::http::HeaderMap;
use axum::http::HeaderName;
use axum::http::Request;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use eyre::Context;
use mime::Mime;
use tracing::instrument;

use crate::errors::ApiError;

pub(crate) struct Xml<T>(pub(crate) T);

impl<T: ex_em_ell::ToXmlDocument> Xml<T> {
    #[instrument(skip(self))]
    fn into_response_or_api_error(self) -> Result<Response<String>, ApiError> {
        let response_xml = ex_em_ell::to_string_pretty(&self.0)?;

        tracing::trace!("Sending response: {response_xml}");

        let response = Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "application/xml")
            .body(response_xml)?;
        Ok(response)
    }
}

impl<T: ex_em_ell::ToXmlDocument> IntoResponse for Xml<T> {
    fn into_response(self) -> axum::response::Response {
        self.into_response_or_api_error().into_response()
    }
}

pub fn respond_to_accepts_header<T>(headers: &HeaderMap, response: T) -> Response
where
    T: ex_em_ell::ToXmlDocument + serde::Serialize,
{
    let content_type = accept_content_type(headers);
    match content_type {
        Ok(ContentType::Xml) => Xml(response).into_response(),
        Ok(ContentType::Json) => Json(response).into_response(),
        Ok(ContentType::Unknown) => {
            ApiError(eyre::eyre!("Could not determine response content type")).into_response()
        }
        Err(e) => ApiError(e).into_response(),
    }
}

pub enum ContentType {
    Xml,
    Json,
    Unknown,
}

impl From<Mime> for ContentType {
    fn from(mime: Mime) -> Self {
        if mime.type_() == "application" {
            if is_mime_type("xml", &mime) {
                return ContentType::Xml;
            } else if is_mime_type("json", &mime) {
                return ContentType::Json;
            }
        }

        return ContentType::Unknown;
    }
}

pub(crate) struct XmlOrJson<T>(pub(crate) T);

/// Borrowed from Axum's Json extractor
#[async_trait]
impl<T, S> FromRequest<S> for XmlOrJson<T>
where
    T: ex_em_ell::FromXmlDocument + serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    #[instrument(skip(req, state))]
    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = content_type(req.headers())?;
        match content_type {
            ContentType::Xml => {
                let bytes = Bytes::from_request(req, state).await?;

                if tracing::enabled!(tracing::Level::TRACE) {
                    match std::str::from_utf8(&bytes) {
                        Ok(request) => tracing::trace!("Got request: {request}"),
                        Err(e) => tracing::trace!("Could not parse request as UTF-8: {e}"),
                    }
                }

                let response: T = ex_em_ell::from_reader(bytes.as_ref())?;
                Ok(XmlOrJson(response))
            }
            ContentType::Json => {
                let bytes = Bytes::from_request(req, state).await?;

                if tracing::enabled!(tracing::Level::TRACE) {
                    match std::str::from_utf8(&bytes) {
                        Ok(request) => tracing::trace!("Got request: {request}"),
                        Err(e) => tracing::trace!("Could not parse request as UTF-8: {e}"),
                    }
                }

                let response: T = serde_json::from_reader(bytes.as_ref())?;
                Ok(XmlOrJson(response))
            }
            ContentType::Unknown => Err(ApiError(eyre::eyre!(
                "Expected a header with application/xml"
            ))),
        }
    }
}

fn accept_content_type(headers: &HeaderMap) -> eyre::Result<ContentType> {
    let accept = mime_type_from_header(header::ACCEPT, headers).map(ContentType::from);

    match accept {
        Ok(accept @ (ContentType::Xml | ContentType::Json)) => return Ok(accept),
        _ => content_type(headers),
    }
}

/// Borrowed from Axum's Json extractor
fn content_type(headers: &HeaderMap) -> eyre::Result<ContentType> {
    let content_type = mime_type_from_header(header::CONTENT_TYPE, headers)?.into();
    Ok(content_type)
}

fn mime_type_from_header(header: HeaderName, headers: &HeaderMap) -> eyre::Result<Mime> {
    let content_type = if let Some(content_type) = headers.get(&header) {
        content_type
    } else {
        eyre::bail!("No {header} header provided");
    };

    let content_type = if let Ok(content_type) = content_type.to_str() {
        content_type
    } else {
        eyre::bail!("Could not parse {header} as a string: {content_type:?}");
    };

    content_type
        .parse()
        .wrap_err_with(|| "Could not parse content type as a Mime type: {content_type}")
}

fn is_mime_type(expected: &str, mime: &Mime) -> bool {
    mime.subtype() == expected || mime.suffix().map_or(false, |name| name == expected)
}
