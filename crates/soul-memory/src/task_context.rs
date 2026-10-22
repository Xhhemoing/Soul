//! One bounded pass over explicitly permitted memories, using literal fields.
//!
//! This is deliberately not natural-language task understanding. Exactly one
//! `需要确认：字段、字段` line declares requirements; other lines are labels,
//! not extra requirements. A few explicitly unsupported constraint markers
//! cause abstention. Summaries must consist entirely of `字段：字面值` lines.
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use soul_store_api::{BlobStore, MemoryStore};
use uuid::Uuid;

use crate::selection::TaskGuard;
use crate::{
    select_for_draft, ContextSelection, ExclusionReason, MemorySelectionRequest, MemoryTimeScope,
    SelectionError, SelectionExclusion, SelectionLimits, SelectionOutcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Matched,
    Clarify,
    Unknown,
    BudgetLimited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRequirementStatus {
    Matched,
    Missing,
    Conflict,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskRequirement {
    pub field: String,
    pub status: TaskRequirementStatus,
    pub source_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskAssessment {
    pub status: TaskStatus,
    pub requirements: Vec<TaskRequirement>,
    pub supplemented_ids: Vec<Uuid>,
    pub question: Option<String>,
    pub notice: String,
}

#[derive(Debug, Clone)]
pub struct TaskSelection {
    pub selection: ContextSelection,
    pub assessment: TaskAssessment,
}

const NOTICE: &str = "仅核对本次允许候选中的显式字段字面线索；需要恰好一行“需要确认：字段、字段”，摘要逐行使用“字段：字面值”。其他任务文字只作标题，不解析其适用性；遇到不支持的条件、否定或未解析材料时保持未知。matched只表示这些字段的字面值一致，不代表事实核实、通用语义充分或起草质量，也不授予原文出网许可。";
const TASK_CONSTRAINTS: &[&str] = &["不需要", "无需", "仅", "如果", "除非", "不要"];
const VALUE_CONSTRAINTS: &[&str] = &[
    "如果", "除非", "可能", "或", "不", "但", "仅", "未知", "待定",
];

fn field_name(field: &str) -> bool {
    !field.is_empty() && field.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn requirements(task: &str, limits: SelectionLimits) -> Result<Vec<String>, TaskStatus> {
    if task.chars().count() > limits.max_query_chars {
        return Err(TaskStatus::BudgetLimited);
    }
    if TASK_CONSTRAINTS.iter().any(|marker| task.contains(marker)) {
        return Err(TaskStatus::Unknown);
    }
    let directives: Vec<_> = task
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("需要确认"))
        .collect();
    if directives.len() != 1 {
        return Err(TaskStatus::Unknown);
    }
    let declaration = directives[0]
        .strip_prefix("需要确认：")
        .or_else(|| directives[0].strip_prefix("需要确认:"))
        .ok_or(TaskStatus::Unknown)?;
    let mut fields = Vec::new();
    for part in declaration.split(['、', '，', ',']) {
        let field = part.trim();
        if !field_name(field) {
            return Err(TaskStatus::Unknown);
        }
        if !fields.iter().any(|known| known == field) {
            fields.push(field.to_owned());
        }
        if fields.len() > limits.max_selected {
            return Err(TaskStatus::BudgetLimited);
        }
    }
    Ok(fields)
}

fn literal_fields(summary: &str) -> Option<Vec<(String, String)>> {
    let mut fields = Vec::new();
    for line in summary
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let (field, value) = line.split_once(['：', ':'])?;
        let field = field.trim();
        let value = value.trim();
        if !field_name(field)
            || value.is_empty()
            || value.contains(['：', ':', '；', ';', '?', '？'])
            || VALUE_CONSTRAINTS
                .iter()
                .any(|marker| value.contains(marker))
        {
            return None;
        }
        fields.push((field.to_owned(), value.to_owned()));
    }
    (!fields.is_empty()).then_some(fields)
}

fn subset(
    pool: &ContextSelection,
    preferred: &[Uuid],
    limits: SelectionLimits,
) -> Result<ContextSelection, SelectionError> {
    let mut selection = pool.clone();
    selection.selected.clear();
    let mut exclusions = BTreeMap::new();
    for id in preferred {
        if selection
            .selected
            .iter()
            .any(|entry| entry.memory_id() == *id)
        {
            continue;
        }
        let entry = pool
            .selected
            .iter()
            .find(|entry| entry.memory_id() == *id)
            .ok_or(SelectionError::SourceChanged)?;
        if selection.selected.len() >= limits.max_selected {
            exclusions.insert(*id, ExclusionReason::SelectionLimit);
            continue;
        }
        selection.selected.push(entry.clone());
        if selection.serialized_context_bytes()? > limits.max_context_bytes {
            selection.selected.pop();
            exclusions.insert(*id, ExclusionReason::ContextBudget);
        }
    }
    let selected: BTreeSet<_> = selection
        .selected
        .iter()
        .map(|entry| entry.memory_id())
        .collect();
    selection
        .excluded
        .extend(pool.selected.iter().filter_map(|entry| {
            (!selected.contains(&entry.memory_id())).then_some(SelectionExclusion {
                memory_id: entry.memory_id(),
                reason: exclusions
                    .get(&entry.memory_id())
                    .copied()
                    .unwrap_or(ExclusionReason::SelectionLimit),
            })
        }));
    Ok(selection)
}

pub fn select_task_context<S>(
    store: &S,
    candidate_ids: Vec<Uuid>,
    task: &str,
) -> Result<TaskSelection, SelectionError>
where
    S: MemoryStore + BlobStore,
{
    let limits = SelectionLimits::default();
    // The temporary pool still has at most 32 individually bounded entries.
    // The normal 8-entry/16KiB budget applies to the actual selected context.
    let pool = select_for_draft(
        store,
        &MemorySelectionRequest {
            candidate_ids,
            query: None,
            memory_type: None,
            time_scope: MemoryTimeScope::Unspecified,
        },
        SelectionLimits {
            max_selected: limits.max_candidates,
            max_context_bytes: usize::MAX,
            ..limits
        },
    )?;
    let mut pool_budget = false;
    let mut pool_unknown = false;
    for exclusion in pool.excluded() {
        match exclusion.reason {
            ExclusionReason::DuplicateId => {}
            ExclusionReason::EntryBudget
            | ExclusionReason::ContextBudget
            | ExclusionReason::SelectionLimit => pool_budget = true,
            ExclusionReason::UnsupportedLinkedSources => pool_unknown = true,
            _ => return Err(SelectionError::SourceChanged),
        }
    }
    let initial_ids: Vec<_> = pool
        .selected()
        .iter()
        .take(4)
        .map(|entry| entry.memory_id())
        .collect();
    let mut selection = subset(&pool, &initial_ids, limits)?;
    let mut assessment = TaskAssessment {
        status: TaskStatus::Unknown,
        requirements: Vec::new(),
        supplemented_ids: Vec::new(),
        question: None,
        notice: NOTICE.to_owned(),
    };
    match requirements(task, limits) {
        Err(status) => assessment.status = status,
        Ok(fields) => {
            let parsed: Vec<_> = pool
                .selected()
                .iter()
                .map(|entry| literal_fields(entry.summary()))
                .collect();
            pool_unknown |= parsed.iter().any(Option::is_none);
            for field in fields {
                let mut values: BTreeMap<String, Vec<Uuid>> = BTreeMap::new();
                let mut sources = Vec::new();
                for (entry, parsed) in pool.selected().iter().zip(&parsed) {
                    for (_, value) in parsed.iter().flatten().filter(|(key, _)| key == &field) {
                        values
                            .entry(value.clone())
                            .or_default()
                            .push(entry.memory_id());
                        if !sources.contains(&entry.memory_id()) {
                            sources.push(entry.memory_id());
                        }
                    }
                }
                assessment.requirements.push(TaskRequirement {
                    field,
                    status: if pool_unknown || pool_budget {
                        TaskRequirementStatus::Unknown
                    } else {
                        match values.len() {
                            0 => TaskRequirementStatus::Missing,
                            1 => TaskRequirementStatus::Matched,
                            _ => TaskRequirementStatus::Conflict,
                        }
                    },
                    source_ids: sources,
                });
            }
            assessment.status = if pool_budget {
                TaskStatus::BudgetLimited
            } else if pool_unknown {
                TaskStatus::Unknown
            } else if assessment
                .requirements
                .iter()
                .all(|row| row.status == TaskRequirementStatus::Matched)
            {
                TaskStatus::Matched
            } else {
                TaskStatus::Clarify
            };
            if assessment.status == TaskStatus::Matched {
                let mut supports = Vec::new();
                for requirement in &assessment.requirements {
                    let id = *requirement
                        .source_ids
                        .first()
                        .ok_or(SelectionError::SourceChanged)?;
                    if !supports.contains(&id) {
                        supports.push(id);
                    }
                }
                // A support group is admitted whole before the initial four.
                selection = subset(&pool, &supports, limits)?;
                if supports.iter().any(|id| {
                    !selection
                        .selected()
                        .iter()
                        .any(|entry| entry.memory_id() == *id)
                }) {
                    assessment.status = TaskStatus::BudgetLimited;
                } else {
                    let mut preferred = supports;
                    preferred.extend(initial_ids.iter().copied());
                    selection = subset(&pool, &preferred, limits)?;
                    assessment.supplemented_ids = selection
                        .selected()
                        .iter()
                        .map(|entry| entry.memory_id())
                        .filter(|id| !initial_ids.contains(id))
                        .collect();
                    selection.task_guard = Some(TaskGuard {
                        pool: pool.selected().to_vec(),
                        support_groups: assessment
                            .requirements
                            .iter()
                            .map(|row| row.source_ids.clone())
                            .collect(),
                    });
                }
            }
            if assessment.status == TaskStatus::Clarify {
                let fields: Vec<_> = assessment
                    .requirements
                    .iter()
                    .filter(|row| row.status != TaskRequirementStatus::Matched)
                    .map(|row| row.field.as_str())
                    .collect();
                assessment.question = Some(format!(
                    "本次候选的显式字段线索中，{}缺少值或字面值不一致；这些字段本次应采用什么值？",
                    fields.join("、")
                ));
            }
        }
    }
    selection.outcome = match assessment.status {
        TaskStatus::Matched => SelectionOutcome::Context,
        TaskStatus::Clarify => SelectionOutcome::Clarify,
        TaskStatus::Unknown | TaskStatus::BudgetLimited => SelectionOutcome::Abstain,
    };
    Ok(TaskSelection {
        selection,
        assessment,
    })
}
