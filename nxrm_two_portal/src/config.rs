// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use config::{Config, Environment};
use portal_api::CENTRAL_HOST;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct AppConfig {
    pub central_url: String,
    pub app_port: u16,
}

impl AppConfig {
    pub fn load() -> eyre::Result<Self> {
        let env_source = Environment::with_prefix("nxrm_two_portal");
        let app_config = Config::builder()
            .set_default("central_url", CENTRAL_HOST)?
            .set_default("app_port", 2727_u16)?
            .add_source(env_source)
            .build()?
            .try_deserialize()?;
        Ok(app_config)
    }
}
