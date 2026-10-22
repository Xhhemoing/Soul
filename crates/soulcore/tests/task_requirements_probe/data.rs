use super::types::{Candidate, InputCase, TaskInput};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InformationOption {
    pub id: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceMetadata {
    pub source_id: String,
    pub subject: String,
    pub scope: String,
    pub valid_time: String,
    pub topics: Vec<String>,
    pub supersedes: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub case_id: String,
    pub task: TaskInput,
    pub information_options: Vec<InformationOption>,
    pub candidates: Vec<Candidate>,
    pub unavailable_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_metadata: Vec<SourceMetadata>,
}

impl Input {
    pub fn case(&self) -> InputCase {
        let mut candidates = self.candidates.clone();
        for candidate in &mut candidates {
            if let Some(metadata) = self
                .source_metadata
                .iter()
                .find(|metadata| metadata.source_id == candidate.id)
            {
                candidate.summary.push_str("\n来源元数据：");
                candidate.summary.push_str(
                    &serde_json::to_string(metadata).expect("serializable source metadata"),
                );
            }
        }
        InputCase {
            case_id: self.case_id.clone(),
            task: self.task.clone(),
            candidates,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Supported,
    Missing,
    Conflict,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Assessment {
    pub need_id: String,
    pub status: Status,
    pub support_groups: Vec<Vec<String>>,
    pub conflict_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Prediction {
    pub case_id: String,
    pub required_ids: Vec<String>,
    pub assessments: Vec<Assessment>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Ready,
    ClarifyMissing,
    ClarifyConflict,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gold {
    pub required_ids: Vec<String>,
    pub assessments: Vec<Assessment>,
    pub action: Action,
    pub rationale: String,
    #[serde(default)]
    pub conflict_groups: std::collections::BTreeMap<String, Vec<Vec<String>>>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub family_id: String,
    pub split: String,
    pub variant: String,
    pub input: Input,
    pub gold: Gold,
}
