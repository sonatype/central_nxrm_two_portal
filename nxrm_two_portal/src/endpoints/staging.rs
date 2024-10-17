// Copyright (c) 2024-present Sonatype, Inc. All rights reserved.
// "Sonatype" is a trademark of Sonatype, Inc.

use std::net::SocketAddr;
use std::ops::Deref;

use axum::extract::{ConnectInfo, Host, Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use axum_extra::headers::UserAgent;
use axum_extra::TypedHeader;
use futures::stream::TryStreamExt;
use itertools::Itertools;
use portal_api::api_types::PublishingType;
use repository::traits::{Repository, RepositoryKey, RepositoryState};
use serde::{ser::SerializeMap, Deserialize, Serialize};
use tracing::instrument;
use user_auth::jwt::UserAuthContext;

use crate::errors::ApiError;
use crate::extract::{respond_to_accepts_header, XmlOrJson};
use crate::publish::publish;
use crate::state::AppState;

#[instrument(skip(headers))]
pub(crate) async fn staging_profile_evaluate_endpoint(
    Host(host): Host,
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    headers: HeaderMap,
    Query(query): Query<StagingProfileEvaluateQueryParams>,
) -> Result<Response, ApiError> {
    tracing::debug!("Request to match staging profiles");
    let staging_profile_evaluate = StagingProfilesEvaluateResponse::new(host, &vec![query.group]);

    Ok(respond_to_accepts_header(
        &headers,
        staging_profile_evaluate,
    ))
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

#[instrument(skip(headers, user_auth_context))]
pub(crate) async fn staging_profiles_list_endpoint(
    Host(host): Host,
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    headers: HeaderMap,
    Extension(user_auth_context): Extension<UserAuthContext>,
) -> Result<Response, ApiError> {
    tracing::debug!("Request to get staging profile");
    let staging_profiles =
        StagingProfilesEvaluateResponse::new(host, &user_auth_context.namespaces);

    Ok(respond_to_accepts_header(&headers, staging_profiles))
}

#[instrument(skip(headers))]
pub(crate) async fn staging_profiles_endpoint(
    Host(host): Host,
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    headers: HeaderMap,
    Path(profile_id): Path<String>,
) -> Result<Response, ApiError> {
    tracing::debug!("Request to get staging profile");
    let staging_profiles = StagingProfilesResponse::new(host, profile_id);

    Ok(respond_to_accepts_header(&headers, staging_profiles))
}

#[instrument(skip(headers, app_state, user_auth_context, staging_profiles_start_request))]
pub(crate) async fn staging_profiles_start_endpoint<R: Repository>(
    Host(host): Host,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    headers: HeaderMap,
    Path(profile_id): Path<String>,
    State(app_state): State<AppState<R>>,
    Extension(user_auth_context): Extension<UserAuthContext>,
    XmlOrJson(staging_profiles_start_request): XmlOrJson<StagingProfilesStartRequest>,
) -> Result<Response, ApiError> {
    tracing::debug!("Request to start staging profile");

    let repository = app_state
        .repository
        .start(&user_auth_context.token_username, &addr.ip(), &profile_id)
        .await?;

    let staging_profiles_start_response = StagingProfilesPromoteResponse::new(
        repository.get_repository_id(),
        staging_profiles_start_request.data.description,
    );

    Ok(respond_to_accepts_header(
        &headers,
        staging_profiles_start_response,
    ))
}

#[derive(Debug, PartialEq, Deserialize, ex_em_ell::FromXmlDocument)]
#[serde(rename_all = "camelCase")]
#[ex_em_ell(rename = "promoteRequest")]
pub(crate) struct StagingProfilesStartRequest {
    data: StagingProfilesStartRequestData,
}

#[derive(Debug, PartialEq, Deserialize, ex_em_ell::FromXmlElement)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StagingProfilesStartRequestData {
    description: String,
}

#[instrument(skip(app_state, user_auth_context, request))]
pub(crate) async fn staging_deploy_by_repository_id<R: Repository>(
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path((repository_id, file_path)): Path<(String, String)>,
    State(app_state): State<AppState<R>>,
    Extension(user_auth_context): Extension<UserAuthContext>,
    request: Request,
) -> Result<impl IntoResponse, ApiError> {
    tracing::debug!("Request to upload file to staging repository");

    if file_path.contains("maven-metadata.xml") {
        tracing::debug!("Skipping adding of a metadata file to the repository");
        return Ok(StatusCode::CREATED);
    }

    let repository_key = RepositoryKey::from_user_context_and_repository_id(
        &user_auth_context.token_username,
        &addr.ip(),
        &repository_id,
    )?;

    app_state
        .repository
        .add_file(
            &user_auth_context.namespaces,
            &repository_key,
            file_path,
            request
                .into_body()
                .into_data_stream()
                .map_err(|e| eyre::eyre!("Issue with the request body: {e}")),
        )
        .await?;

    Ok(StatusCode::CREATED)
}

#[instrument]
pub(crate) async fn staging_deploy_by_repository_id_get(
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    Path((repository_id, file_path)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::debug!("Request to get file from a staging repository");

    Ok(StatusCode::NOT_FOUND)
}

#[instrument(skip(app_state, user_auth_context, staging_profiles_finish_request))]
pub(crate) async fn staging_profiles_finish_endpoint<R: Repository>(
    Host(host): Host,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    Path(profile_id): Path<String>,
    State(app_state): State<AppState<R>>,
    Extension(user_auth_context): Extension<UserAuthContext>,
    XmlOrJson(staging_profiles_finish_request): XmlOrJson<StagingProfilesFinishRequest>,
) -> Result<StatusCode, ApiError> {
    tracing::debug!("Request to finish profile");

    let repository_key = RepositoryKey::from_user_context_and_repository_id(
        &user_auth_context.token_username,
        &addr.ip(),
        &staging_profiles_finish_request.data.staged_repository_id,
    )?;

    publish(
        &app_state.portal_api_client,
        app_state.repository.deref(),
        &user_auth_context,
        &repository_key,
        PublishingType::Automatic,
    )
    .await?;

    Ok(StatusCode::OK)
}

#[derive(Debug, PartialEq, Deserialize, ex_em_ell::FromXmlDocument)]
#[serde(rename_all = "camelCase")]
#[ex_em_ell(rename = "promoteRequest")]
pub(crate) struct StagingProfilesFinishRequest {
    data: StagingProfilesFinishRequestData,
}

#[derive(Debug, PartialEq, Deserialize, ex_em_ell::FromXmlElement)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StagingProfilesFinishRequestData {
    staged_repository_id: String,
    description: String,
}

#[instrument(skip(headers, app_state, user_auth_context))]
pub(crate) async fn staging_repository<R: Repository>(
    Host(host): Host,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    headers: HeaderMap,
    Path(repository_id): Path<String>,
    State(app_state): State<AppState<R>>,
    Extension(user_auth_context): Extension<UserAuthContext>,
) -> Result<Response, ApiError> {
    tracing::debug!("Request to get repository");

    let repository_key = RepositoryKey::from_user_context_and_repository_id(
        &user_auth_context.token_username,
        &addr.ip(),
        &repository_id,
    )?;

    let repository_state = app_state.repository.get_state(&repository_key).await?;

    let response = StagingRepositoryResponse::new(&host, &repository_id, repository_state);

    Ok(respond_to_accepts_header(&headers, response))
}

#[instrument(skip(app_state, user_auth_context, staging_bulk_promote_request))]
pub(crate) async fn staging_bulk_promote<R: Repository>(
    Host(host): Host,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    State(app_state): State<AppState<R>>,
    Extension(user_auth_context): Extension<UserAuthContext>,

    XmlOrJson(staging_bulk_promote_request): XmlOrJson<StagingBulkPromoteRequest>,
) -> Result<StatusCode, ApiError> {
    tracing::debug!(
        "Request to bulk promote repositories: {}",
        staging_bulk_promote_request
            .data
            .staged_repository_ids
            .iter()
            .map(|ws| ws.0.to_owned())
            .join(", ")
    );

    for repository_id in staging_bulk_promote_request.data.staged_repository_ids {
        let repository_key = RepositoryKey::from_user_context_and_repository_id(
            &user_auth_context.token_username,
            &addr.ip(),
            &repository_id.0,
        )?;

        app_state.repository.release(&repository_key).await?;
    }

    Ok(StatusCode::OK)
}

#[derive(Debug, PartialEq, Deserialize, ex_em_ell::FromXmlDocument)]
#[serde(rename_all = "camelCase")]
#[ex_em_ell(rename = "stagingActionRequest")]
pub(crate) struct StagingBulkPromoteRequest {
    data: StagingBulkPromoteRequestData,
}

#[derive(Debug, PartialEq, Deserialize, ex_em_ell::FromXmlElement)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StagingBulkPromoteRequestData {
    staged_repository_ids: Vec<WrappedString>,
    description: String,
    auto_drop_after_release: bool,
}

#[instrument(skip(app_state, user_auth_context, staging_bulk_close_request))]
pub(crate) async fn staging_bulk_close<R: Repository>(
    Host(host): Host,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    State(app_state): State<AppState<R>>,
    Extension(user_auth_context): Extension<UserAuthContext>,
    XmlOrJson(staging_bulk_close_request): XmlOrJson<StagingBulkPromoteRequest>,
) -> Result<StatusCode, ApiError> {
    tracing::debug!(
        "Request to bulk close repositories: {}",
        staging_bulk_close_request
            .data
            .staged_repository_ids
            .iter()
            .map(|ws| ws.0.to_owned())
            .join(", ")
    );

    let username = user_auth_context.token_username.clone();

    for repository_id in staging_bulk_close_request.data.staged_repository_ids {
        let repository_key = RepositoryKey::from_user_context_and_repository_id(
            &username,
            &addr.ip(),
            &repository_id.0,
        )?;

        publish(
            &app_state.portal_api_client,
            app_state.repository.deref(),
            &user_auth_context,
            &repository_key,
            PublishingType::Automatic,
        )
        .await?;
    }

    Ok(StatusCode::OK)
}

#[instrument(skip(app_state, user_auth_context, request))]
pub(crate) async fn staging_deploy_maven2<R: Repository>(
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(file_path): Path<String>,
    State(app_state): State<AppState<R>>,
    Extension(user_auth_context): Extension<UserAuthContext>,
    request: Request,
) -> Result<impl IntoResponse, ApiError> {
    tracing::debug!("Request to upload file to staging repository");

    let repository_key = app_state
        .repository
        .open_no_profile_repository(&user_auth_context.token_username, &addr.ip())
        .await?;

    app_state
        .repository
        .add_file(
            &user_auth_context.namespaces,
            &repository_key,
            file_path,
            request
                .into_body()
                .into_data_stream()
                .map_err(|e| eyre::eyre!("Issue with the request body: {e}")),
        )
        .await?;

    Ok(StatusCode::CREATED)
}

#[instrument(skip(_app_state, _user_auth_context))]
pub(crate) async fn staging_deploy_maven2_get<R: Repository>(
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    Path(file_path): Path<String>,
    State(_app_state): State<AppState<R>>,
    Extension(_user_auth_context): Extension<UserAuthContext>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::debug!("Request to get a file from a staging repository");

    Ok(StatusCode::NOT_FOUND)
}

#[derive(Debug, Serialize, ex_em_ell::ToXmlDocument)]
#[serde(rename_all = "camelCase")]
#[ex_em_ell(rename = "stagingProfiles")]
pub(crate) struct StagingProfilesEvaluateResponse {
    data: Vec<StagingProfile>,
}

impl StagingProfilesEvaluateResponse {
    fn new(base_url: String, namespaces: &[String]) -> Self {
        let staging_profiles = namespaces
            .iter()
            .map(|namespace| {
                StagingProfile::new(
                    &base_url,
                    namespace,
                    format!("{base_url}/service/local/staging/profile_evaluate/{namespace}"),
                )
            })
            .collect();
        Self {
            data: staging_profiles,
        }
    }
}

#[derive(Debug, Serialize, ex_em_ell::ToXmlDocument)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Serialize, ex_em_ell::ToXmlDocument)]
#[serde(rename_all = "camelCase")]
#[ex_em_ell(rename = "promoteResponse")]
pub(crate) struct StagingProfilesPromoteResponse {
    data: StagingProfilesResponseData,
}

impl StagingProfilesPromoteResponse {
    fn new(staged_repository_id: String, description: String) -> Self {
        Self {
            data: StagingProfilesResponseData {
                staged_repository_id,
                description,
            },
        }
    }
}

#[derive(Debug, Serialize, ex_em_ell::ToXmlElement)]
#[serde(rename_all = "camelCase")]
struct StagingProfilesResponseData {
    staged_repository_id: String,
    description: String,
}

#[derive(Debug, Serialize, ex_em_ell::ToXmlElement, ex_em_ell::NamedXmlElement)]
#[serde(rename_all = "camelCase")]
struct StagingProfile {
    #[serde(rename = "resourceURI")]
    #[ex_em_ell(rename = "resourceURI")]
    resource_uri: String,
    id: String,
    name: String,
    repository_type: String,
    repository_template_id: String,
    repository_target_id: String,
    in_progress: bool,
    order: u32,
    #[serde(rename = "deployURI")]
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
            order: 12345,
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

#[derive(Debug, Serialize, PartialEq, Deserialize, ex_em_ell::NamedXmlElement)]
#[ex_em_ell(name = "string")]
struct WrappedString(String);

impl ex_em_ell::ToXmlElement for WrappedString {
    fn to_xml_element<W: std::io::Write>(
        &self,
        writer: &mut ex_em_ell::xml::EventWriter<W>,
        tag: &str,
    ) -> Result<(), ex_em_ell::errors::XmlWriteError> {
        ex_em_ell::xml_utils::write_simple_tag(writer, tag, &self.0)
    }
}

impl ex_em_ell::FromXmlElement for WrappedString {
    fn from_xml_element<R: std::io::Read>(
        reader: &mut ex_em_ell::xml::EventReader<R>,
        element_name: &ex_em_ell::xml::name::OwnedName,
        _element_attributes: &[ex_em_ell::xml::attribute::OwnedAttribute],
        _element_namespace: &ex_em_ell::xml::namespace::Namespace,
    ) -> Result<Self, ex_em_ell::errors::XmlReadError>
    where
        Self: Sized,
    {
        let value: String = ex_em_ell::xml_utils::read_simple_tag(reader, element_name)?;

        Ok(WrappedString(value))
    }
}

#[derive(Debug)]
struct Properties();

impl Serialize for Properties {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("@class", "linked-hash-map")?;
        map.end()
    }
}

impl ex_em_ell::ToXmlElement for Properties {
    fn to_xml_element<W: std::io::Write>(
        &self,
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

#[derive(Debug, Serialize, ex_em_ell::ToXmlDocument)]
#[serde(rename_all = "camelCase")]
#[ex_em_ell(rename = "stagingProfileRepository")]
pub(crate) struct StagingRepositoryResponse {
    profile_id: String,
    profile_name: String,
    profile_type: String,
    repository_id: String,
    #[serde(rename = "type")]
    #[ex_em_ell(rename = "type")]
    repository_type: String,
    policy: String,
    user_id: String,
    user_agent: String,
    ip_address: String,
    #[serde(rename = "repositoryURI")]
    #[ex_em_ell(rename = "repositoryURI")]
    repository_uri: String,
    created: String,
    created_date: String,
    created_timestamp: u32,
    updated: String,
    updated_date: String,
    updated_timestamp: u32,
    description: String,
    provider: String,
    release_repository_id: String,
    release_repository_name: String,
    notifications: u32,
    transitioning: bool,
}

impl StagingRepositoryResponse {
    fn new(base_url: &str, repository_id: &str, repository_state: RepositoryState) -> Self {
        Self {
            profile_id: "profile_id".to_string(), // TODO: do we need this to be persisted?
            profile_name: "profile_name".to_string(),
            profile_type: "repository".to_string(),
            repository_id: repository_id.to_string(),
            repository_type: repository_state.to_string(),
            policy: "release".to_string(),
            user_id: "user_id".to_string(),
            user_agent: "user_agent".to_string(),
            ip_address: "ip_address".to_string(),
            repository_uri: format!("{base_url}/content/repositories/{repository_id}"),
            created: "1970-01-01T00:00:00.000Z".to_string(),
            created_date: "Thu Jan 1 00:00:00 UTC 1970".to_string(),
            created_timestamp: 0,
            updated: "1970-01-01T00:00:00.000Z".to_string(),
            updated_date: "Thu Jan 1 00:00:00 UTC 1970".to_string(),
            updated_timestamp: 0,
            description: "description".to_string(),
            provider: "maven2".to_string(),
            release_repository_id: "releases".to_string(),
            release_repository_name: "Releases".to_string(),
            notifications: 0,
            transitioning: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_serialization_staging_profiles_evaluate_response() -> eyre::Result<()> {
        let staging_profiles_evaluate_response = StagingProfilesEvaluateResponse::new(
            "https://s01.oss.sonatype.org".to_string(),
            &vec!["com.example".to_string()],
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
    fn test_json_serialization_staging_profiles_evaluate_response() -> eyre::Result<()> {
        let staging_profiles_evaluate_response = StagingProfilesEvaluateResponse::new(
            "https://s01.oss.sonatype.org".to_string(),
            &vec!["com.example".to_string()],
        );
        let actual_json = serde_json::to_string_pretty(&staging_profiles_evaluate_response)?;
        let expected_json = r#"{
  "data": [
    {
      "resourceURI": "https://s01.oss.sonatype.org/service/local/staging/profile_evaluate/com.example",
      "id": "com.example",
      "name": "com.example",
      "repositoryType": "maven2",
      "repositoryTemplateId": "default_hosted_release",
      "repositoryTargetId": "repository_target_id",
      "inProgress": false,
      "order": 12345,
      "deployURI": "https://s01.oss.sonatype.org/service/local/staging/deploy/maven2",
      "targetGroups": [
        "staging"
      ],
      "finishNotifyRoles": [
        "com.example-deployer"
      ],
      "promotionNotifyRoles": [],
      "dropNotifyRoles": [],
      "closeRuleSets": [
        "close_rule_set"
      ],
      "promoteRuleSets": [],
      "promotionTargetRepository": "releases",
      "mode": "BOTH",
      "finishNotifyCreator": true,
      "promotionNotifyCreator": true,
      "dropNotifyCreator": true,
      "autoStagingDisabled": false,
      "repositoriesSearchable": false,
      "properties": {
        "@class": "linked-hash-map"
      }
    }
  ]
}"#;

        assert_eq!(actual_json, expected_json);

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
    fn test_json_serialization_staging_profiles_response() -> eyre::Result<()> {
        let staging_profiles_evaluate_response = StagingProfilesResponse::new(
            "https://s01.oss.sonatype.org".to_string(),
            "com.example".to_string(),
        );
        let actual_json = serde_json::to_string_pretty(&staging_profiles_evaluate_response)?;
        let expected_json = r#"{
  "data": {
    "resourceURI": "https://s01.oss.sonatype.org/service/local/staging/profiles/com.example/com.example",
    "id": "com.example",
    "name": "com.example",
    "repositoryType": "maven2",
    "repositoryTemplateId": "default_hosted_release",
    "repositoryTargetId": "repository_target_id",
    "inProgress": false,
    "order": 12345,
    "deployURI": "https://s01.oss.sonatype.org/service/local/staging/deploy/maven2",
    "targetGroups": [
      "staging"
    ],
    "finishNotifyRoles": [
      "com.example-deployer"
    ],
    "promotionNotifyRoles": [],
    "dropNotifyRoles": [],
    "closeRuleSets": [
      "close_rule_set"
    ],
    "promoteRuleSets": [],
    "promotionTargetRepository": "releases",
    "mode": "BOTH",
    "finishNotifyCreator": true,
    "promotionNotifyCreator": true,
    "dropNotifyCreator": true,
    "autoStagingDisabled": false,
    "repositoriesSearchable": false,
    "properties": {
      "@class": "linked-hash-map"
    }
  }
}"#;

        assert_eq!(actual_json, expected_json);

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
    fn test_json_deserialization_staging_profiles_start_request() -> eyre::Result<()> {
        let actual_json = r#"{ "data": { "description": "com.example:example:0.1.0" } }"#;
        let actual_staging_profile_request: StagingProfilesStartRequest =
            serde_json::from_reader(actual_json.as_bytes())?;
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
        let staging_profiles_start_response = StagingProfilesPromoteResponse::new(
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

    #[test]
    fn test_json_serialization_staging_profiles_start_response() -> eyre::Result<()> {
        let staging_profiles_start_response = StagingProfilesPromoteResponse::new(
            "comexample-1".to_string(),
            "com.example:example:0.1.0".to_string(),
        );
        let actual_json = serde_json::to_string_pretty(&staging_profiles_start_response)?;
        let expected_json = r#"{
  "data": {
    "stagedRepositoryId": "comexample-1",
    "description": "com.example:example:0.1.0"
  }
}"#;

        assert_eq!(actual_json, expected_json);

        Ok(())
    }

    #[test]
    fn test_xml_deserialization_staging_profiles_finish_request() -> eyre::Result<()> {
        let actual_xml = "<promoteRequest><data><stagedRepositoryId>comexample-1</stagedRepositoryId><description>com.example:example:0.1.0</description></data></promoteRequest>";
        let actual_staging_profile_request: StagingProfilesFinishRequest =
            ex_em_ell::from_reader(actual_xml.as_bytes())?;
        let expected_staging_profile_request = StagingProfilesFinishRequest {
            data: StagingProfilesFinishRequestData {
                staged_repository_id: "comexample-1".to_string(),
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
    fn test_json_deserialization_staging_profiles_finish_request() -> eyre::Result<()> {
        let actual_json = r#"{ "data": { "stagedRepositoryId": "comexample-1", "description": "com.example:example:0.1.0" } }"#;
        let actual_staging_profile_request: StagingProfilesFinishRequest =
            serde_json::from_reader(actual_json.as_bytes())?;
        let expected_staging_profile_request = StagingProfilesFinishRequest {
            data: StagingProfilesFinishRequestData {
                staged_repository_id: "comexample-1".to_string(),
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
    fn test_xml_serialization_repository_response() -> eyre::Result<()> {
        let repository_response = StagingRepositoryResponse::new(
            "https://s01.oss.sonatype.org",
            "comexample-1",
            RepositoryState::Closed,
        );
        let actual_xml = ex_em_ell::to_string_pretty(&repository_response)?;
        let expected_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<stagingProfileRepository>
  <profileId>profile_id</profileId>
  <profileName>profile_name</profileName>
  <profileType>repository</profileType>
  <repositoryId>comexample-1</repositoryId>
  <type>closed</type>
  <policy>release</policy>
  <userId>user_id</userId>
  <userAgent>user_agent</userAgent>
  <ipAddress>ip_address</ipAddress>
  <repositoryURI>https://s01.oss.sonatype.org/content/repositories/comexample-1</repositoryURI>
  <created>1970-01-01T00:00:00.000Z</created>
  <createdDate>Thu Jan 1 00:00:00 UTC 1970</createdDate>
  <createdTimestamp>0</createdTimestamp>
  <updated>1970-01-01T00:00:00.000Z</updated>
  <updatedDate>Thu Jan 1 00:00:00 UTC 1970</updatedDate>
  <updatedTimestamp>0</updatedTimestamp>
  <description>description</description>
  <provider>maven2</provider>
  <releaseRepositoryId>releases</releaseRepositoryId>
  <releaseRepositoryName>Releases</releaseRepositoryName>
  <notifications>0</notifications>
  <transitioning>false</transitioning>
</stagingProfileRepository>"#;

        assert_eq!(actual_xml, expected_xml);

        Ok(())
    }

    #[test]
    fn test_json_serialization_repository_response() -> eyre::Result<()> {
        let repository_response = StagingRepositoryResponse::new(
            "https://s01.oss.sonatype.org",
            "comexample-1",
            RepositoryState::Closed,
        );
        let actual_json = serde_json::to_string_pretty(&repository_response)?;
        let expected_json = r#"{
  "profileId": "profile_id",
  "profileName": "profile_name",
  "profileType": "repository",
  "repositoryId": "comexample-1",
  "type": "closed",
  "policy": "release",
  "userId": "user_id",
  "userAgent": "user_agent",
  "ipAddress": "ip_address",
  "repositoryURI": "https://s01.oss.sonatype.org/content/repositories/comexample-1",
  "created": "1970-01-01T00:00:00.000Z",
  "createdDate": "Thu Jan 1 00:00:00 UTC 1970",
  "createdTimestamp": 0,
  "updated": "1970-01-01T00:00:00.000Z",
  "updatedDate": "Thu Jan 1 00:00:00 UTC 1970",
  "updatedTimestamp": 0,
  "description": "description",
  "provider": "maven2",
  "releaseRepositoryId": "releases",
  "releaseRepositoryName": "Releases",
  "notifications": 0,
  "transitioning": false
}"#;

        assert_eq!(actual_json, expected_json);

        Ok(())
    }

    #[test]
    fn test_xml_deserialization_staging_bulk_promote_request() -> eyre::Result<()> {
        let actual_xml = "<stagingActionRequest><data><stagedRepositoryIds><string>comexample-1</string></stagedRepositoryIds><description>description</description><autoDropAfterRelease>true</autoDropAfterRelease></data></stagingActionRequest>";
        let actual_staging_bulk_promote_request: StagingBulkPromoteRequest =
            ex_em_ell::from_reader(actual_xml.as_bytes())?;
        let expected_bulk_promote_request = StagingBulkPromoteRequest {
            data: StagingBulkPromoteRequestData {
                staged_repository_ids: vec![WrappedString("comexample-1".to_string())],
                description: "description".to_string(),
                auto_drop_after_release: true,
            },
        };

        assert_eq!(
            actual_staging_bulk_promote_request,
            expected_bulk_promote_request
        );

        Ok(())
    }

    #[test]
    fn test_json_deserialization_staging_bulk_promote_request() -> eyre::Result<()> {
        let actual_json = r#"{ "data": { "stagedRepositoryIds": ["comexample-1"], "description": "description", "autoDropAfterRelease": true } }"#;
        let actual_staging_bulk_promote_request: StagingBulkPromoteRequest =
            serde_json::from_reader(actual_json.as_bytes())?;
        let expected_bulk_promote_request = StagingBulkPromoteRequest {
            data: StagingBulkPromoteRequestData {
                staged_repository_ids: vec![WrappedString("comexample-1".to_string())],
                description: "description".to_string(),
                auto_drop_after_release: true,
            },
        };

        assert_eq!(
            actual_staging_bulk_promote_request,
            expected_bulk_promote_request
        );

        Ok(())
    }
}
