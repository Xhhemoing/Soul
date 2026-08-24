//! What a scan can refuse, and how it says so.
//!
//! One rule shapes this file: a refusal must not repeat the path it refused.
//! A caller that asks about `/home/someone/private` and is told "`/home/
//! someone/private` is not authorized" has been handed an oracle — the error
//! confirms the spelling it was given, and a caller that walks a list of
//! guesses learns which ones exist. [`FilePlanError::PathNotAuthorized`]
//! therefore carries the roots the user *did* authorize and nothing else,
//! which is also the only thing the person reading the message can act on.

use std::path::PathBuf;

use soul_policy::ReasonCode;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FilePlanError {
    /// The request is not inside any authorized root.
    ///
    /// The requested path is deliberately absent; see the module note.
    #[error("that path is not inside any authorized root ({})", render(.roots))]
    PathNotAuthorized { roots: Vec<PathBuf> },

    /// A directory the user authorized, or one under it, could not be read.
    #[error("{} could not be read: {reason}", path.display())]
    RootUnreadable { path: PathBuf, reason: String },

    /// An authorized path that is not a directory. Reported only for paths
    /// that passed authorization, so it is never an existence oracle.
    #[error("{} is not a directory", .path.display())]
    NotADirectory { path: PathBuf },
}

impl FilePlanError {
    /// The code the audit entry carries.
    ///
    /// Only the authorization refusal is a policy decision. A directory that
    /// cannot be read is a fact about the disk, and recording it as a
    /// permission refusal would put a reason in the chain that nobody could
    /// act on.
    pub fn reason_code(&self) -> ReasonCode {
        match self {
            FilePlanError::PathNotAuthorized { .. } => ReasonCode::PathNotAuthorized,
            FilePlanError::RootUnreadable { .. } | FilePlanError::NotADirectory { .. } => {
                ReasonCode::Routine
            }
        }
    }
}

/// The authorized roots, as the user would recognise them.
///
/// Built with `format!` rather than a hand-written `Display` implementation:
/// the formatting macros that take a sink are part of the vocabulary
/// `tests/no_write_api.rs` refuses, so this crate composes strings and
/// returns them instead.
fn render(roots: &[PathBuf]) -> String {
    match roots.is_empty() {
        true => "no directory has been authorized".to_owned(),
        false => roots
            .iter()
            .map(|root| format!("{}", root.display()))
            .collect::<Vec<String>>()
            .join(", "),
    }
}
