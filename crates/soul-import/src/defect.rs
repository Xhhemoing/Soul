//! What is wrong with a file, said out loud without quoting it.
//!
//! An import either lands whole or is refused whole. There is no partial mode:
//! a file that half-imported would leave the user with a graph built on rows
//! they never saw reported, and no way to tell which half made it.
//!
//! So a refusal has to be worth reading. Every [`Defect`] names where it is —
//! a line number for JSONL, a JSON path for Telegram — and what is wrong, in
//! terms of the field rather than the value. The reasons are built from the
//! validator's *error kind*, never its `Display`, because the rendered form
//! embeds the instance that failed; see [`crate::redact`].

use std::fmt;

use jsonschema::error::{TypeKind, ValidationErrorKind};
use jsonschema::{ValidationError, Validator};
use serde_json::Value;

use crate::model::ImportSource;
use crate::redact::{summarize_names, ContentGuard};

/// Where a defect is.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Locator {
    /// One-based line number in a JSONL file.
    Line(usize),
    /// A path into a JSON document, such as `chats.list[2].messages[0]`.
    Path(String),
}

impl fmt::Display for Locator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Locator::Line(number) => write!(f, "line {number}"),
            Locator::Path(path) => f.write_str(path),
        }
    }
}

/// One thing wrong with one place in the file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Defect {
    pub locator: Locator,
    /// The field, when the validator could point at one. Empty means the whole
    /// object.
    pub field: Option<String>,
    pub reason: String,
}

impl Defect {
    pub fn at(locator: Locator, reason: impl Into<String>) -> Defect {
        Defect {
            locator,
            field: None,
            reason: reason.into(),
        }
    }

    pub fn field(locator: Locator, field: impl Into<String>, reason: impl Into<String>) -> Defect {
        Defect {
            locator,
            field: Some(field.into()),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for Defect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.field {
            Some(field) => write!(f, "{}: `{field}` {}", self.locator, self.reason),
            None => write!(f, "{}: {}", self.locator, self.reason),
        }
    }
}

/// A refused import, with everything that was wrong with it.
///
/// The field is `format` rather than `source` because `thiserror` reads a
/// field named `source` as the underlying error of the chain.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub struct ImportFailure {
    pub format: ImportSource,
    pub defects: Vec<Defect>,
    /// How many lines or messages were looked at before giving up.
    pub items_examined: usize,
}

impl ImportFailure {
    pub fn new(format: ImportSource, defects: Vec<Defect>, items_examined: usize) -> ImportFailure {
        ImportFailure {
            format,
            defects,
            items_examined,
        }
    }

    /// Every distinct place that has a problem.
    pub fn locators(&self) -> Vec<&Locator> {
        let mut seen: Vec<&Locator> = Vec::new();
        for defect in &self.defects {
            if !seen.contains(&&defect.locator) {
                seen.push(&defect.locator);
            }
        }
        seen
    }

    pub fn mentions_field(&self, field: &str) -> bool {
        self.defects
            .iter()
            .any(|defect| defect.field.as_deref() == Some(field))
    }
}

impl fmt::Display for ImportFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the {} import was refused: {} problem(s) across {} item(s) examined",
            self.format.as_str(),
            self.defects.len(),
            self.items_examined,
        )?;
        for defect in &self.defects {
            write!(f, "\n  - {defect}")?;
        }
        Ok(())
    }
}

/// Validate one value and describe every failure without quoting it.
pub fn schema_defects(
    validator: &Validator,
    instance: &Value,
    locator: &Locator,
    guard: &ContentGuard,
) -> Vec<Defect> {
    validator
        .iter_errors(instance)
        .map(|error| {
            let field = field_of(&error);
            let reason = guard.guard(describe(&error, guard));
            Defect {
                locator: locator.clone(),
                field,
                reason,
            }
        })
        .collect()
}

/// The field a validation error is about, if it is about one.
///
/// A missing property is reported against the enclosing object, so the
/// property name comes out of the error kind rather than the instance path.
fn field_of(error: &ValidationError<'_>) -> Option<String> {
    if let ValidationErrorKind::Required { property } = &error.kind {
        if let Value::String(name) = property {
            return Some(name.clone());
        }
    }
    let path = error.instance_path.to_string();
    match path.is_empty() {
        true => None,
        false => Some(path.trim_start_matches('/').replace('/', ".")),
    }
}

/// A reason built from the error *kind*.
///
/// Everything named here comes from the schema — a required property name, an
/// enumeration, a format, a limit — except the property names in
/// `additionalProperties`, which are the one fragment out of the file and are
/// bounded and guarded by [`summarize_names`].
fn describe(error: &ValidationError<'_>, guard: &ContentGuard) -> String {
    match &error.kind {
        ValidationErrorKind::Required { property } => match property {
            Value::String(name) => format!("is required and this item has no `{name}`"),
            _ => "is missing a required field".to_owned(),
        },
        ValidationErrorKind::Enum { options } => match options {
            Value::Array(values) => format!(
                "must be one of {}",
                summarize_names(
                    &values
                        .iter()
                        .map(|value| match value {
                            Value::String(text) => text.clone(),
                            other => other.to_string(),
                        })
                        .collect::<Vec<_>>(),
                    guard,
                ),
            ),
            _ => "is not one of the values the contract allows".to_owned(),
        },
        ValidationErrorKind::Constant { expected_value } => match expected_value {
            Value::String(text) => format!("must equal `{text}`"),
            other => format!("must equal {other}"),
        },
        ValidationErrorKind::Type { kind } => match kind {
            TypeKind::Single(primitive) => format!("must be a JSON {primitive}"),
            TypeKind::Multiple(_) => "has a JSON type the contract does not allow".to_owned(),
        },
        ValidationErrorKind::Format { format } => format!("is not a valid `{format}`"),
        ValidationErrorKind::Pattern { pattern } => format!("does not match `{pattern}`"),
        ValidationErrorKind::AdditionalProperties { unexpected } => {
            format!(
                "carries {}, which the contract does not define",
                summarize_names(unexpected, guard)
            )
        }
        ValidationErrorKind::MinLength { limit } => format!("needs at least {limit} character(s)"),
        ValidationErrorKind::MaxLength { limit } => format!("allows at most {limit} character(s)"),
        ValidationErrorKind::MinItems { limit } => format!("needs at least {limit} item(s)"),
        ValidationErrorKind::MaxItems { limit } => format!("allows at most {limit} item(s)"),
        ValidationErrorKind::UniqueItems => "repeats an item".to_owned(),
        ValidationErrorKind::OneOfNotValid => {
            "matches none of the shapes the contract allows: this line is neither a valid header \
             nor a valid message"
                .to_owned()
        }
        ValidationErrorKind::OneOfMultipleValid => {
            "matches more than one shape, so it cannot be read".to_owned()
        }
        ValidationErrorKind::AnyOf => "matches none of the shapes the contract allows".to_owned(),
        _ => "does not satisfy the contract".to_owned(),
    }
}
