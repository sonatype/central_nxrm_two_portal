use async_trait::async_trait;
use axum::body::Body;
use axum::body::Bytes;
use axum::extract::FromRequest;
use axum::http::header;
use axum::http::header::CONTENT_TYPE;
use axum::http::HeaderMap;
use axum::http::Request;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;

use crate::errors::ApiError;

pub(crate) struct Xml<T>(pub(crate) T);

impl<T: ex_em_ell::ToXmlDocument> Xml<T> {
    fn into_response_or_api_error(self) -> Result<Response<String>, ApiError> {
        let response_xml = ex_em_ell::to_string_pretty(&self.0)?;

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

/// Borrowed from Axum's Json extractor
#[async_trait]
impl<T, S> FromRequest<S> for Xml<T>
where
    T: ex_em_ell::FromXmlDocument,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        if xml_content_type(req.headers()) {
            let bytes = Bytes::from_request(req, state).await?;
            let response: T = ex_em_ell::from_reader(bytes.as_ref())?;
            Ok(Xml(response))
        } else {
            Err(ApiError(eyre::eyre!(
                "Expected a header with application/xml"
            )))
        }
    }
}

/// Borrowed from Axum's Json extractor
fn xml_content_type(headers: &HeaderMap) -> bool {
    let content_type = if let Some(content_type) = headers.get(header::CONTENT_TYPE) {
        content_type
    } else {
        return false;
    };

    let content_type = if let Ok(content_type) = content_type.to_str() {
        content_type
    } else {
        return false;
    };

    let mime = match content_type.parse::<mime::Mime>() {
        Ok(mime) => mime,
        Err(_) => return false,
    };

    let is_xml_content_type = mime.type_() == "application"
        && (mime.subtype() == "xml" || mime.suffix().map_or(false, |name| name == "xml"));

    is_xml_content_type
}
