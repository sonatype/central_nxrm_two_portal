use axum::extract::{Host, Path, Query};
use axum_extra::headers::UserAgent;
use axum_extra::TypedHeader;
use serde::Deserialize;
use tracing::instrument;

use crate::errors::ApiError;
use crate::extract::Xml;

#[instrument]
pub(crate) async fn staging_profile_evaluate_endpoint(
    Host(host): Host,
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    Query(query): Query<StagingProfileEvaluateQueryParams>,
) -> Result<Xml<StagingProfilesEvaluateResponse>, ApiError> {
    tracing::debug!("Request to match staging profiles");
    let staging_profile_evaluate = StagingProfilesEvaluateResponse::new(host, query.group);

    Ok(Xml(staging_profile_evaluate))
}

#[derive(Debug, Deserialize)]
pub(crate) struct StagingProfileEvaluateQueryParams {
    #[serde(rename = "a")]
    _artifact: String,
    #[serde(rename = "t")]
    _repository_type: String,
    #[serde(rename = "v")]
    _version: String,
    #[serde(rename = "g")]
    group: String,
}

#[instrument]
pub(crate) async fn staging_profiles_endpoint(
    Host(host): Host,
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    Path(profile_id): Path<String>,
) -> Result<Xml<StagingProfilesResponse>, ApiError> {
    tracing::debug!("Request to get staging profile");
    let staging_profiles = StagingProfilesResponse::new(host, profile_id);

    Ok(Xml(staging_profiles))
}

#[instrument(skip(staging_profiles_start_request))]
pub(crate) async fn staging_profiles_start_endpoint(
    Host(host): Host,
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    Path(profile_id): Path<String>,
    Xml(staging_profiles_start_request): Xml<StagingProfilesStartRequest>,
) -> Result<Xml<StagingProfilesStartResponse>, ApiError> {
    tracing::debug!("Request to start staging profile");
    let staging_profiles_start_response = StagingProfilesStartResponse::new(
        format!("{profile_id}-1"),
        staging_profiles_start_request.data.description,
    );

    Ok(Xml(staging_profiles_start_response))
}

#[derive(Debug, PartialEq, ex_em_ell::FromXmlDocument)]
#[ex_em_ell(rename = "promoteRequest")]
pub(crate) struct StagingProfilesStartRequest {
    data: StagingProfilesStartRequestData,
}

#[derive(Debug, PartialEq, ex_em_ell::FromXmlElement)]
pub(crate) struct StagingProfilesStartRequestData {
    description: String,
}

#[derive(Debug, ex_em_ell::ToXmlDocument)]
#[ex_em_ell(rename = "stagingProfiles")]
pub(crate) struct StagingProfilesEvaluateResponse {
    data: Vec<StagingProfile>,
}

impl StagingProfilesEvaluateResponse {
    fn new(base_url: String, namespace: String) -> Self {
        Self {
            data: vec![StagingProfile::new(
                &base_url,
                &namespace,
                format!("{base_url}/service/local/staging/profile_evaluate/{namespace}"),
            )],
        }
    }
}

#[derive(Debug, ex_em_ell::ToXmlDocument)]
#[ex_em_ell(rename = "profileResponse")]
pub(crate) struct StagingProfilesResponse {
    data: StagingProfile,
}

impl StagingProfilesResponse {
    fn new(base_url: String, profile_id: String) -> Self {
        Self {
            data: StagingProfile::new(
                &base_url,
                &profile_id,
                format!("{base_url}/service/local/staging/profiles/{profile_id}/{profile_id}"),
            ),
        }
    }
}

#[derive(Debug, ex_em_ell::ToXmlDocument)]
#[ex_em_ell(rename = "promoteResponse")]
pub(crate) struct StagingProfilesStartResponse {
    data: StagingProfilesStartResponseData,
}

impl StagingProfilesStartResponse {
    fn new(staged_repository_id: String, description: String) -> Self {
        Self {
            data: StagingProfilesStartResponseData {
                staged_repository_id,
                description,
            },
        }
    }
}

#[derive(Debug, ex_em_ell::ToXmlElement, ex_em_ell::NamedXmlElement)]
struct StagingProfile {
    #[ex_em_ell(rename = "resourceURI")]
    resource_uri: String,
    id: String,
    name: String,
    repository_type: String,
    repository_template_id: String,
    repository_target_id: String,
    in_progress: bool,
    order: String,
    #[ex_em_ell(rename = "deployURI")]
    deploy_uri: String,
    target_groups: Vec<WrappedString>,
    finish_notify_roles: Vec<WrappedString>,
    promotion_notify_roles: Vec<WrappedString>,
    drop_notify_roles: Vec<WrappedString>,
    close_rule_sets: Vec<WrappedString>,
    promote_rule_sets: Vec<WrappedString>,
    promotion_target_repository: String,
    mode: String,
    finish_notify_creator: bool,
    promotion_notify_creator: bool,
    drop_notify_creator: bool,
    auto_staging_disabled: bool,
    repositories_searchable: bool,
    properties: Properties,
}

impl StagingProfile {
    fn new(base_url: &str, namespace: &str, resource_uri: String) -> Self {
        Self {
            resource_uri,
            id: namespace.to_string(), // TODO: does this need to be numeric? The XSD says String
            name: namespace.to_string(),
            repository_type: "maven2".to_string(),
            repository_template_id: "default_hosted_release".to_string(),
            repository_target_id: "repository_target_id".to_string(),
            in_progress: false,
            order: "12345".to_string(),
            deploy_uri: format!("{base_url}/service/local/staging/deploy/maven2"),
            target_groups: vec![WrappedString("staging".to_string())],
            finish_notify_roles: vec![WrappedString(format!("{namespace}-deployer"))],
            promotion_notify_roles: Vec::new(),
            drop_notify_roles: Vec::new(),
            close_rule_sets: vec![WrappedString("close_rule_set".to_string())],
            promote_rule_sets: Vec::new(),
            promotion_target_repository: "releases".to_string(),
            mode: "BOTH".to_string(),
            finish_notify_creator: true,
            promotion_notify_creator: true,
            drop_notify_creator: true,
            auto_staging_disabled: false,
            repositories_searchable: false,
            properties: Properties(),
        }
    }
}

#[derive(Debug, ex_em_ell::NamedXmlElement)]
#[ex_em_ell(name = "string")]
struct WrappedString(String);

impl ex_em_ell::ToXmlElement for WrappedString {
    fn to_xml_element<W: std::io::Write>(
        self: &Self,
        writer: &mut ex_em_ell::xml::EventWriter<W>,
        tag: &str,
    ) -> Result<(), ex_em_ell::errors::XmlWriteError> {
        ex_em_ell::xml_utils::write_simple_tag(writer, tag, &self.0)
    }
}

#[derive(Debug)]
struct Properties();

impl ex_em_ell::ToXmlElement for Properties {
    fn to_xml_element<W: std::io::Write>(
        self: &Self,
        writer: &mut ex_em_ell::xml::EventWriter<W>,
        tag: &str,
    ) -> Result<(), ex_em_ell::errors::XmlWriteError> {
        writer
            .write(
                ex_em_ell::xml::writer::XmlEvent::start_element(tag)
                    .attr("class", "linked-hash-map"),
            )
            .map_err(ex_em_ell::xml_utils::to_xml_write_error(tag))?;

        writer
            .write(ex_em_ell::xml::writer::XmlEvent::end_element())
            .map_err(ex_em_ell::xml_utils::to_xml_write_error(tag))?;
        Ok(())
    }
}

#[derive(Debug, ex_em_ell::ToXmlElement)]
struct StagingProfilesStartResponseData {
    staged_repository_id: String,
    description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_serialization_staging_profiles_evaluate_response() -> eyre::Result<()> {
        let staging_profiles_evaluate_response = StagingProfilesEvaluateResponse::new(
            "https://s01.oss.sonatype.org".to_string(),
            "com.example".to_string(),
        );
        let actual_xml = ex_em_ell::to_string_pretty(&staging_profiles_evaluate_response)?;
        let expected_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<stagingProfiles>
  <data>
    <stagingProfile>
      <resourceURI>https://s01.oss.sonatype.org/service/local/staging/profile_evaluate/com.example</resourceURI>
      <id>com.example</id>
      <name>com.example</name>
      <repositoryType>maven2</repositoryType>
      <repositoryTemplateId>default_hosted_release</repositoryTemplateId>
      <repositoryTargetId>repository_target_id</repositoryTargetId>
      <inProgress>false</inProgress>
      <order>12345</order>
      <deployURI>https://s01.oss.sonatype.org/service/local/staging/deploy/maven2</deployURI>
      <targetGroups>
        <string>staging</string>
      </targetGroups>
      <finishNotifyRoles>
        <string>com.example-deployer</string>
      </finishNotifyRoles>
      <promotionNotifyRoles />
      <dropNotifyRoles />
      <closeRuleSets>
        <string>close_rule_set</string>
      </closeRuleSets>
      <promoteRuleSets />
      <promotionTargetRepository>releases</promotionTargetRepository>
      <mode>BOTH</mode>
      <finishNotifyCreator>true</finishNotifyCreator>
      <promotionNotifyCreator>true</promotionNotifyCreator>
      <dropNotifyCreator>true</dropNotifyCreator>
      <autoStagingDisabled>false</autoStagingDisabled>
      <repositoriesSearchable>false</repositoriesSearchable>
      <properties class="linked-hash-map" />
    </stagingProfile>
  </data>
</stagingProfiles>"#;

        assert_eq!(actual_xml, expected_xml);

        Ok(())
    }

    #[test]
    fn test_xml_serialization_staging_profiles_response() -> eyre::Result<()> {
        let staging_profiles_evaluate_response = StagingProfilesResponse::new(
            "https://s01.oss.sonatype.org".to_string(),
            "com.example".to_string(),
        );
        let actual_xml = ex_em_ell::to_string_pretty(&staging_profiles_evaluate_response)?;
        let expected_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<profileResponse>
  <data>
    <resourceURI>https://s01.oss.sonatype.org/service/local/staging/profiles/com.example/com.example</resourceURI>
    <id>com.example</id>
    <name>com.example</name>
    <repositoryType>maven2</repositoryType>
    <repositoryTemplateId>default_hosted_release</repositoryTemplateId>
    <repositoryTargetId>repository_target_id</repositoryTargetId>
    <inProgress>false</inProgress>
    <order>12345</order>
    <deployURI>https://s01.oss.sonatype.org/service/local/staging/deploy/maven2</deployURI>
    <targetGroups>
      <string>staging</string>
    </targetGroups>
    <finishNotifyRoles>
      <string>com.example-deployer</string>
    </finishNotifyRoles>
    <promotionNotifyRoles />
    <dropNotifyRoles />
    <closeRuleSets>
      <string>close_rule_set</string>
    </closeRuleSets>
    <promoteRuleSets />
    <promotionTargetRepository>releases</promotionTargetRepository>
    <mode>BOTH</mode>
    <finishNotifyCreator>true</finishNotifyCreator>
    <promotionNotifyCreator>true</promotionNotifyCreator>
    <dropNotifyCreator>true</dropNotifyCreator>
    <autoStagingDisabled>false</autoStagingDisabled>
    <repositoriesSearchable>false</repositoriesSearchable>
    <properties class="linked-hash-map" />
  </data>
</profileResponse>"#;

        assert_eq!(actual_xml, expected_xml);

        Ok(())
    }

    #[test]
    fn test_xml_deserialization_staging_profiles_start_request() -> eyre::Result<()> {
        let actual_xml = "<promoteRequest><data><description>com.example:example:0.1.0</description></data></promoteRequest>";
        let actual_staging_profile_request: StagingProfilesStartRequest =
            ex_em_ell::from_reader(actual_xml.as_bytes())?;
        let expected_staging_profile_request = StagingProfilesStartRequest {
            data: StagingProfilesStartRequestData {
                description: "com.example:example:0.1.0".to_string(),
            },
        };

        assert_eq!(
            actual_staging_profile_request,
            expected_staging_profile_request
        );

        Ok(())
    }

    #[test]
    fn test_xml_serialization_staging_profiles_start_response() -> eyre::Result<()> {
        let staging_profiles_start_response = StagingProfilesStartResponse::new(
            "comexample-1".to_string(),
            "com.example:example:0.1.0".to_string(),
        );
        let actual_xml = ex_em_ell::to_string_pretty(&staging_profiles_start_response)?;
        let expected_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<promoteResponse>
  <data>
    <stagedRepositoryId>comexample-1</stagedRepositoryId>
    <description>com.example:example:0.1.0</description>
  </data>
</promoteResponse>"#;

        assert_eq!(actual_xml, expected_xml);

        Ok(())
    }
}
