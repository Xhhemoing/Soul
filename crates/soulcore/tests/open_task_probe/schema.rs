use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawInput {
    pub case_id: String,
    pub task_text: String,
    pub sources: Vec<RawSource>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawSource {
    pub id: String,
    pub title: String,
    pub text: String,
    pub readable: bool,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Need {
    pub id: String,
    pub description: String,
    pub task_quote: String,
    pub status: super::data::Status,
    pub support_groups: Vec<Vec<String>>,
    pub conflict_ids: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub source_id: String,
    #[serde(deserialize_with = "required_nullable")]
    pub subject: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub scope: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub time: Option<String>,
    pub supersedes: Vec<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub correction_quote: Option<String>,
}

fn required_nullable<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OpenPrediction {
    pub case_id: String,
    pub needs: Vec<Need>,
    pub source_metadata: Vec<Metadata>,
}

pub fn validate_raw(raw: &RawInput) -> Result<(), &'static str> {
    if raw.task_text.trim().is_empty() {
        return Err("task_text_empty");
    }
    if raw.sources.iter().any(|source| {
        source.id.is_empty()
            || source.id.len() > 128
            || !source
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
            || source.title.trim().is_empty()
            || source.text.trim().is_empty()
    }) {
        return Err("source_shape");
    }
    if serde_json::to_vec(raw).map_err(|_| "input_encoding")?.len()
        > super::policy::MAX_PREDICTION_BYTES
    {
        return Err("input_budget");
    }
    super::diagnosis::validate_fixed_input(&project(raw, None)?)
}

pub fn parse_prediction(bytes: &[u8], raw: &RawInput) -> Result<OpenPrediction, &'static str> {
    validate_raw(raw)?;
    if bytes.len() > super::policy::MAX_PREDICTION_BYTES {
        return Err("prediction_budget");
    }
    let prediction: OpenPrediction =
        serde_json::from_slice(bytes).map_err(|_| "prediction_shape")?;
    validate_prediction(raw, &prediction)?;
    Ok(prediction)
}

pub fn validate_prediction(
    raw: &RawInput,
    prediction: &OpenPrediction,
) -> Result<(), &'static str> {
    validate_raw(raw)?;
    if prediction.case_id != raw.case_id {
        return Err("prediction_case_id");
    }
    if prediction.needs.len() > 8 {
        return Err("need_count");
    }
    if prediction.needs.iter().any(|need| {
        need.task_quote.trim().is_empty()
            || need.task_quote.chars().count() > 240
            || !raw.task_text.contains(&need.task_quote)
    }) {
        return Err("task_quote_origin");
    }
    let pool: std::collections::BTreeSet<_> = raw.sources.iter().map(|s| &s.id).collect();
    let metadata_ids: std::collections::BTreeSet<_> = prediction
        .source_metadata
        .iter()
        .map(|m| &m.source_id)
        .collect();
    if metadata_ids != pool || metadata_ids.len() != prediction.source_metadata.len() {
        return Err("source_metadata_ids");
    }
    for metadata in &prediction.source_metadata {
        let source = raw
            .sources
            .iter()
            .find(|s| s.id == metadata.source_id)
            .ok_or("source_metadata_ids")?;
        let text = format!("{}\n{}", source.title, source.text);
        for excerpt in [&metadata.subject, &metadata.scope, &metadata.time]
            .into_iter()
            .flatten()
        {
            if excerpt.trim().is_empty() || excerpt.chars().count() > 240 || !text.contains(excerpt)
            {
                return Err("metadata_excerpt_origin");
            }
        }
        let supersedes: std::collections::BTreeSet<_> = metadata.supersedes.iter().collect();
        if supersedes.len() != metadata.supersedes.len()
            || !supersedes.is_subset(&pool)
            || supersedes.contains(&metadata.source_id)
        {
            return Err("metadata_supersedes_ids");
        }
        match (&metadata.correction_quote, metadata.supersedes.is_empty()) {
            (None, true) => {}
            (Some(quote), false)
                if !quote.trim().is_empty()
                    && quote.chars().count() <= 240
                    && text.contains(quote)
                    && metadata.supersedes.iter().all(|id| quote.contains(id)) => {}
            _ => return Err("metadata_correction_origin"),
        }
        if metadata
            .supersedes
            .iter()
            .any(|id| raw.sources.iter().any(|s| s.id == *id && !s.readable))
        {
            return Err("metadata_unreadable_reference");
        }
    }
    let input = project(raw, Some(prediction))?;
    if prediction.needs.is_empty() {
        return super::diagnosis::validate_fixed_input(&input);
    }
    super::diagnosis::validate_input(&input)?;
    super::diagnosis::validate_prediction(&input, &legacy_prediction(prediction))
}

pub fn project(
    raw: &RawInput,
    prediction: Option<&OpenPrediction>,
) -> Result<super::data::Input, &'static str> {
    Ok(super::data::Input {
        case_id: raw.case_id.clone(),
        task: super::types::TaskInput {
            scope: "开放合成任务".into(),
            intent: raw.task_text.clone(),
            constraints: "仅合成材料，只准备不生成".into(),
        },
        information_options: prediction
            .map(|p| {
                p.needs
                    .iter()
                    .map(|need| super::data::InformationOption {
                        id: need.id.clone(),
                        description: need.description.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        candidates: raw
            .sources
            .iter()
            .map(|source| {
                let mut summary = source.text.clone();
                if let Some(metadata) = prediction
                    .and_then(|p| p.source_metadata.iter().find(|m| m.source_id == source.id))
                {
                    summary.push_str("\n机器提取元数据（仅验证原文出处，语义未经核验）：");
                    summary.push_str(
                        &serde_json::to_string(metadata).map_err(|_| "metadata_encoding")?,
                    );
                }
                Ok(super::types::Candidate {
                    id: source.id.clone(),
                    title: source.title.clone(),
                    summary,
                })
            })
            .collect::<Result<_, &'static str>>()?,
        unavailable_ids: raw
            .sources
            .iter()
            .filter(|s| !s.readable)
            .map(|s| s.id.clone())
            .collect(),
        source_metadata: Vec::new(),
    })
}

pub fn legacy_prediction(prediction: &OpenPrediction) -> super::data::Prediction {
    super::data::Prediction {
        case_id: prediction.case_id.clone(),
        required_ids: prediction
            .needs
            .iter()
            .map(|need| need.id.clone())
            .collect(),
        assessments: prediction
            .needs
            .iter()
            .map(|need| super::data::Assessment {
                need_id: need.id.clone(),
                status: need.status,
                support_groups: need.support_groups.clone(),
                conflict_ids: need.conflict_ids.clone(),
            })
            .collect(),
    }
}
