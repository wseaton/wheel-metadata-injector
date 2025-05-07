use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildEnvMetadata {
    #[serde(with = "chrono_format")]
    pub build_time: DateTime<Utc>,
    pub git: Option<RepositoryInfo>,
    #[serde(rename = "env")]
    pub env_vars: IndexMap<String, String>,
    pub automation: Option<AutomationInfo>,
}

mod chrono_format {
    use chrono::{DateTime, ParseError, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let rfc3339 = DateTime::to_rfc3339(date);
        rfc3339
            .serialize(serializer)
            .map_err(serde::ser::Error::custom)
    }

    // Custom parser that can handle the +00 year prefix in timestamps
    fn parse_datetime(s: &str) -> Result<DateTime<Utc>, ParseError> {
        // Handle the case where the year has a +00 prefix (e.g., +002025)
        if let Some(stripped) = s.strip_prefix("+00") {
            return DateTime::parse_from_rfc3339(stripped).map(|dt| dt.with_timezone(&Utc));
        }

        // Try regular RFC3339 parsing
        DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        parse_datetime(&s).map_err(|e| {
            serde::de::Error::custom(format!("Failed to parse datetime '{}': {}", s, e))
        })
    }
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
