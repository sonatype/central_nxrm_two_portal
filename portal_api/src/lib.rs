use reqwest::{Client, ClientBuilder};

pub struct PortalApiClient {
    client: Client,
}

impl PortalApiClient {
    pub fn default() -> eyre::Result<Self> {
        let client = ClientBuilder::default().build()?;
        Ok(Self { client })
    }

    pub async fn upload(&mut self) -> eyre::Result<()> {
        todo!()
    }
}
