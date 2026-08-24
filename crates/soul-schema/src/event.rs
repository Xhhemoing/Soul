//! Model for `docs/schemas/event.schema.json`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::{ActorSubject, Privacy, SchemaVersion, SealedText, Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSource {
    #[serde(rename = "collector.foreground_app")]
    CollectorForegroundApp,
    #[serde(rename = "import.soul_import_v1")]
    ImportSoulImportV1,
    #[serde(rename = "import.telegram_desktop")]
    ImportTelegramDesktop,
    #[serde(rename = "ui.questionnaire")]
    UiQuestionnaire,
    #[serde(rename = "ui.paste")]
    UiPaste,
    #[serde(rename = "core.memory")]
    CoreMemory,
    #[serde(rename = "core.forget")]
    CoreForget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    #[serde(rename = "app.foreground")]
    AppForeground,
    #[serde(rename = "message.observed")]
    MessageObserved,
    #[serde(rename = "import.item")]
    ImportItem,
    #[serde(rename = "questionnaire.answer")]
    QuestionnaireAnswer,
    #[serde(rename = "memory.write")]
    MemoryWrite,
    #[serde(rename = "forget.execute")]
    ForgetExecute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulEvent {
    pub schema_version: SchemaVersion,
    pub event_id: Uuid,
    pub ts: Timestamp,
    pub source: EventSource,
    pub kind: EventKind,
    pub actor_subject: ActorSubject,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consent_id: Option<Uuid>,
    pub privacy: Privacy,
    /// Sealed pointer, never prose. `None` for events that carry no body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_ref: Option<SealedText>,
}
