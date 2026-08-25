//! One stretch of time with one application in front.
//!
//! A session is the unit the collector records, and it is deliberately not the
//! unit the collector samples. Polling produces a sample every tick; writing a
//! row every tick would put the user's foreground history in the store at
//! whatever resolution the poll interval happened to be, and PRODUCT_LOCK only
//! asked for duration. So samples accumulate into a session, and the session
//! becomes one event when the user moves on.
//!
//! What lands in the store is [`SealedSessionBody`], and it has exactly two
//! fields. `event.schema.json` has nowhere to put an application name — the
//! only free-form thing on an event is `body_ref`, which is a pointer to
//! AEAD-sealed bytes — so the name and the duration travel together inside the
//! seal, under the collector's content key, where forgetting can reach them.

use serde::{Deserialize, Serialize};

use crate::source::AppIdentity;

/// Column name bound into the AEAD tag, and the only sealed field an event has.
pub const BODY_FIELD: &str = "body_ref";

/// A completed stretch of foreground time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundSession {
    pub app: AppIdentity,
    pub started_at_unix_millis: u64,
    pub ended_at_unix_millis: u64,
}

impl ForegroundSession {
    /// Milliseconds the application was in front.
    ///
    /// Saturating, because both ends come from the wall clock: it is the clock
    /// an event timestamp has to be in, and it can step backwards when the
    /// machine syncs time or wakes from sleep. A negative duration is not a
    /// thing that can be recorded, so a backwards step records zero.
    pub fn duration_ms(&self) -> u64 {
        self.ended_at_unix_millis
            .saturating_sub(self.started_at_unix_millis)
    }

    /// Whole seconds of the start instant, which is what the event's `ts` is.
    pub fn started_at_unix_seconds(&self) -> i64 {
        (self.started_at_unix_millis / 1_000) as i64
    }

    pub fn sealed_body(&self) -> SealedSessionBody {
        SealedSessionBody {
            app: self.app.as_str().to_owned(),
            duration_ms: self.duration_ms(),
        }
    }
}

/// Everything Soul keeps about one foreground session.
///
/// Two fields, and `deny_unknown_fields` on the way back in, so a later version
/// that starts sealing a window title cannot be read by this one — and a test
/// can state the shape rather than hope for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedSessionBody {
    /// Executable file name. Never a path, never a title.
    pub app: String,
    pub duration_ms: u64,
}

impl SealedSessionBody {
    /// The field names this body may have. `tests` assert against this rather
    /// than against a literal list that could drift from the struct.
    pub const FIELDS: &'static [&'static str] = &["app", "duration_ms"];

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn from_json_bytes(bytes: &[u8]) -> Result<SealedSessionBody, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// A session that has not ended yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenSession {
    pub(crate) app: AppIdentity,
    pub(crate) started_at_unix_millis: u64,
}

impl OpenSession {
    pub(crate) fn started(app: AppIdentity, at_unix_millis: u64) -> OpenSession {
        OpenSession {
            app,
            started_at_unix_millis: at_unix_millis,
        }
    }

    pub(crate) fn ended_at(self, at_unix_millis: u64) -> ForegroundSession {
        ForegroundSession {
            app: self.app,
            started_at_unix_millis: self.started_at_unix_millis,
            ended_at_unix_millis: at_unix_millis,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(started: u64, ended: u64) -> ForegroundSession {
        ForegroundSession {
            app: AppIdentity::new("code.exe").expect("a valid name"),
            started_at_unix_millis: started,
            ended_at_unix_millis: ended,
        }
    }

    #[test]
    fn a_backwards_clock_records_zero_rather_than_underflowing() {
        assert_eq!(session(5_000, 4_000).duration_ms(), 0);
        assert_eq!(session(4_000, 5_500).duration_ms(), 1_500);
    }

    #[test]
    fn the_sealed_body_carries_the_name_and_the_duration_and_nothing_else() {
        let bytes = session(1_787_529_600_000, 1_787_529_612_000)
            .sealed_body()
            .to_json_bytes()
            .expect("serialize");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        let keys: Vec<&str> = value
            .as_object()
            .expect("an object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, SealedSessionBody::FIELDS);
        assert_eq!(value["duration_ms"], 12_000);
    }

    #[test]
    fn a_body_with_an_extra_field_is_refused_on_the_way_back_in() {
        let intruder = r#"{"app":"code.exe","duration_ms":1,"window_caption":"报税 2026"}"#;
        assert!(SealedSessionBody::from_json_bytes(intruder.as_bytes()).is_err());
    }
}
