use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildEnvMetadata {
    #[serde(serialize_with = "time::serde::iso8601::serialize")]
    #[serde(deserialize_with = "time::serde::iso8601::deserialize")]
    pub build_time: OffsetDateTime,
    pub git: Option<RepositoryInfo>,
    #[serde(rename = "env")]
    pub env_vars: IndexMap<String, String>,
    pub automation: Option<AutomationInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryInfo {
    /// The git remote URL of the repository.
    #[serde(rename = "url")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The commit hash of the repository at the time of wheel creation.
    #[serde(rename = "commit")]
    pub commit: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AutomationInfo {
    /// Information specific to github actions.
    #[serde(flatten)]
    pub actions_info: Option<ActionsInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionsInfo {
    /// The GitHub Actions run ID.
    #[serde(rename = "run_id")]
    pub run_id: Option<String>,
    /// The GitHub Actions workflow name.
    #[serde(rename = "workflow_name")]
    pub workflow_name: Option<String>,
    /// The GitHub Actions workflow SHA.
    #[serde(rename = "workflow_sha")]
    pub workflow_sha: Option<String>,
    /// The GitHub Actions job name.
    #[serde(rename = "job_name")]
    pub job_name: Option<String>,
    /// Runner name.
    #[serde(rename = "runner_name")]
    pub runner_name: Option<String>,
}
