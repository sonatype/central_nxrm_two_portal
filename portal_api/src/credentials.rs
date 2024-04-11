use base64::engine::general_purpose::STANDARD;
use base64::engine::Engine;
use reqwest::{
    header::{HeaderValue, AUTHORIZATION},
    RequestBuilder,
};

pub struct Credentials {
    username: String,
    password: String,
}

impl Credentials {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    pub fn add_credentials_to_request(
        &self,
        request: RequestBuilder,
    ) -> eyre::Result<RequestBuilder> {
        let token_header = HeaderValue::from_str(&self.as_bearer_token())?;
        let request = request.header(AUTHORIZATION, token_header);
        tracing::trace!("Added {AUTHORIZATION} header");
        Ok(request)
    }

    fn as_bearer_token(&self) -> String {
        let token = STANDARD.encode(format!("{}:{}", self.username, self.password));
        format!("UserToken {token}")
    }
}
