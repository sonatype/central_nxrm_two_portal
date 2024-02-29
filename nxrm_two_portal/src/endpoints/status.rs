use axum::extract::Host;
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::response::Response;
use axum_extra::headers::UserAgent;
use axum_extra::TypedHeader;
use tracing::instrument;

use crate::errors::ApiError;

#[instrument]
pub(crate) async fn status_endpoint(
    Host(host): Host,
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
) -> Result<Response<String>, ApiError> {
    tracing::debug!("Request to get status");
    let status = StatusResult::new(host);
    let status_response = ex_em_ell::to_string_pretty(&status)?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/xml")
        .body(status_response)?;
    Ok(response)
}

#[derive(Debug, ex_em_ell::ToXmlDocument)]
#[ex_em_ell(rename = "status")]
struct StatusResult {
    data: Data,
}

#[derive(Debug, ex_em_ell::ToXmlElement)]
struct Data {
    app_name: String,
    formatted_app_name: String,
    version: String,
    api_version: String,
    edition_long: String,
    edition_short: String,
    #[ex_em_ell(rename = "attributionsURL")]
    attributions_url: String,
    #[ex_em_ell(rename = "purchaseURL")]
    purchase_url: String,
    #[ex_em_ell(rename = "userLicenseURL")]
    user_license_url: String,
    state: String,
    initialized_at: String,
    started_at: String,
    last_config_change: String,
    first_start: bool,
    instance_upgraded: bool,
    configuration_upgraded: bool,
    base_url: String,
    license_installed: bool,
    license_expired: bool,
    trial_license: bool,
}

impl StatusResult {
    fn new(base_url: String) -> Self {
        Self {
            data: Data {
                app_name: "Nexus Repository Manager".to_string(),
                formatted_app_name: "Nexus Repository Manager".to_string(),
                version: "2.15.1-02".to_string(),
                api_version: "2.15.1-02".to_string(),
                edition_long: "Professional".to_string(),
                edition_short: "PRO".to_string(),
                attributions_url: "http://links.sonatype.com/products/nexus/pro/attributions"
                    .to_string(),
                purchase_url: "http://links.sonatype.com/products/nexus/pro/store".to_string(),
                user_license_url: "http://links.sonatype.com/products/nexus/pro/eula".to_string(),
                state: "STARTED".to_string(),
                initialized_at: "1970-01-01 00:00:00.000 UTC".to_string(),
                started_at: "1970-01-01 00:00:00.000 UTC".to_string(),
                last_config_change: "1970-01-01 00:00:00.000 UTC".to_string(),
                first_start: false,
                instance_upgraded: false,
                configuration_upgraded: false,
                base_url,
                license_installed: true,
                license_expired: false,
                trial_license: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_serialization() -> eyre::Result<()> {
        let status_result = StatusResult::new("https://s01.oss.sonatype.org".to_string());
        let actual_state_xml = ex_em_ell::to_string_pretty(&status_result)?;
        let expected_state_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<status>
  <data>
    <appName>Nexus Repository Manager</appName>
    <formattedAppName>Nexus Repository Manager</formattedAppName>
    <version>2.15.1-02</version>
    <apiVersion>2.15.1-02</apiVersion>
    <editionLong>Professional</editionLong>
    <editionShort>PRO</editionShort>
    <attributionsURL>http://links.sonatype.com/products/nexus/pro/attributions</attributionsURL>
    <purchaseURL>http://links.sonatype.com/products/nexus/pro/store</purchaseURL>
    <userLicenseURL>http://links.sonatype.com/products/nexus/pro/eula</userLicenseURL>
    <state>STARTED</state>
    <initializedAt>1970-01-01 00:00:00.000 UTC</initializedAt>
    <startedAt>1970-01-01 00:00:00.000 UTC</startedAt>
    <lastConfigChange>1970-01-01 00:00:00.000 UTC</lastConfigChange>
    <firstStart>false</firstStart>
    <instanceUpgraded>false</instanceUpgraded>
    <configurationUpgraded>false</configurationUpgraded>
    <baseUrl>https://s01.oss.sonatype.org</baseUrl>
    <licenseInstalled>true</licenseInstalled>
    <licenseExpired>false</licenseExpired>
    <trialLicense>false</trialLicense>
  </data>
</status>"#;

        assert_eq!(actual_state_xml, expected_state_xml);

        Ok(())
    }
}
