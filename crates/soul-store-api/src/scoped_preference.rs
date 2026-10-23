//! Explicit owner preferences for one person and one finite occasion.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Shared with the existing voice profile; serialization remains unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceRegister {
    Casual,
    Plain,
    Formal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceOccasion {
    Personal,
    Work,
    Study,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactPreferenceScope {
    pub owner_profile_id: Uuid,
    pub person_contact_id: Uuid,
    pub occasion: PreferenceOccasion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopedPreferenceState {
    Active,
    Inactive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedRegisterPreference {
    pub preference_id: Uuid,
    pub revision: Uuid,
    pub scope: ExactPreferenceScope,
    pub memory_id: Uuid,
    pub register: Option<VoiceRegister>,
    pub state: ScopedPreferenceState,
}

/// An absent scope and a stale existing scope are different expectations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedPreference {
    Absent,
    Revision { preference_id: Uuid, revision: Uuid },
}

impl ScopedRegisterPreference {
    pub fn expected(&self) -> ExpectedPreference {
        ExpectedPreference::Revision {
            preference_id: self.preference_id,
            revision: self.revision,
        }
    }
}
