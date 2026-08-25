//! The runtime half of "no numeric ratings on a trait axis".
//!
//! `xtask denylist-audit` keeps the words `score` and `percentile` out of the
//! sources, and `fixtures/schemas/invalid/profile/numeric_axis_rating.json`
//! keeps a numeric field out of the contract. Neither catches a number that a
//! future caller puts in a field the schema happens to allow, so the write path
//! checks the serialized axes as well: PRODUCT_LOCK's "特质轴无数字分数" is a
//! property of what is stored, not of how it is spelled.

use serde_json::Value;

use soul_schema::profile::TraitAxis;

/// Where a number turned up. Carries the JSON pointer so the message names the
/// field rather than the whole document.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("trait axes must carry a direction and an evidence band, never a number; found {value} at {path}")]
pub struct NumericRating {
    pub path: String,
    pub value: String,
}

/// Refuse any number anywhere inside the serialized trait axes.
///
/// Deliberately blunt. `char_count` and other legitimate integers live
/// elsewhere in the contracts; inside `trait_axes` there is nothing a number
/// could honestly be.
pub fn reject_numeric_rating(axes: &[TraitAxis]) -> Result<(), NumericRating> {
    let value = serde_json::to_value(axes).map_err(|error| NumericRating {
        path: "/trait_axes".into(),
        value: error.to_string(),
    })?;
    reject_numeric_rating_value(&value)
}

/// The same check over arbitrary JSON.
///
/// [`TraitAxis`] as it stands today has no field a number could go in, so the
/// typed check above can never fire and would be a comforting no-op on its own.
/// This is the form that can be pointed at a doctored document, which is what
/// `tests/axes_and_evidence.rs` does to show the walker is armed — and it is
/// the form that keeps working the day the contract grows a field.
pub fn reject_numeric_rating_value(axes: &Value) -> Result<(), NumericRating> {
    walk(axes, "/trait_axes")
}

fn walk(value: &Value, path: &str) -> Result<(), NumericRating> {
    match value {
        Value::Number(number) => Err(NumericRating {
            path: path.to_owned(),
            value: number.to_string(),
        }),
        Value::Object(map) => map
            .iter()
            .try_for_each(|(key, nested)| walk(nested, &format!("{path}/{key}"))),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .try_for_each(|(index, nested)| walk(nested, &format!("{path}/{index}"))),
        _ => Ok(()),
    }
}
