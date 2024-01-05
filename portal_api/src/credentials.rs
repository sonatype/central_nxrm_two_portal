use base64::engine::general_purpose::STANDARD;
use base64::engine::Engine;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

pub struct Credentials {
    username: String,
    password: String,
}

impl Credentials {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    pub fn add_credentials_to_headers(&self, headers: &mut HeaderMap) -> eyre::Result<()> {
        let token_header = HeaderValue::from_str(&self.as_bearer_token())?;
        headers.insert(AUTHORIZATION, token_header);
        tracing::trace!("Added {AUTHORIZATION} header");
        Ok(())
    }

    fn as_bearer_token(&self) -> String {
        let token = STANDARD.encode(format!("{}:{}", self.username, self.password));
        format!("UserToken {token}")
    }
}
