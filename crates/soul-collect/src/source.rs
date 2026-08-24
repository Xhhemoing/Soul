//! Where a foreground sample comes from, and what a sample is allowed to say.
//!
//! [`AppIdentity`] is the whole of what Soul learns from a window. PRODUCT_LOCK
//! puts window titles in the "不做" column, so the type that carries a sample
//! has nowhere to put one: it holds an executable file name, it refuses a value
//! that still has a directory in it, and it refuses anything long enough to be
//! a sentence. `tests/window_titles_are_not_collected.rs` reads this crate's
//! sources back and checks that no title API is named anywhere in them.
//!
//! The trait is the seam WP07 was asked for. Windows implements it against the
//! real desktop; CI implements it with [`crate::FakeForegroundSource`], and both
//! go through the same collector, the same consent gate and the same store.

use std::fmt;

/// Longest name a real executable has. Anything longer is not a program name,
/// and refusing it here means a caller that reaches for a window title finds
/// the door shut rather than half open.
pub const MAX_APP_NAME_CHARS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    #[error("a foreground sample carried no application name")]
    EmptyName,

    #[error("`{name}` is a path, not an application name; only the file name is collected")]
    NotAFileName { name: String },

    #[error(
        "an application name of {chars} characters is longer than the {MAX_APP_NAME_CHARS} \
         a program name can be; window titles are not collected"
    )]
    NameTooLong { chars: usize },

    /// The platform call failed. Carries the call that failed, never a name.
    #[error("the platform foreground lookup failed: {0}")]
    Platform(String),

    /// This build has no collector for the machine it is running on. Linux CI
    /// hosts hit this, which is why the fake source exists.
    #[error("this platform has no foreground collector")]
    Unsupported,
}

/// Which application was in the foreground. Never which window, never its
/// title, never its document.
///
/// Stored lower-cased: Windows paths are case-insensitive, and `Code.exe` and
/// `code.exe` are one application, not two sessions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AppIdentity(String);

impl AppIdentity {
    /// An executable file name, such as `notepad.exe`.
    pub fn new(name: impl AsRef<str>) -> Result<AppIdentity, SourceError> {
        let name = name.as_ref().trim();
        if name.is_empty() {
            return Err(SourceError::EmptyName);
        }
        if name.contains(['/', '\\']) || name.contains(':') {
            return Err(SourceError::NotAFileName {
                name: name.to_owned(),
            });
        }
        if name.chars().any(char::is_control) {
            return Err(SourceError::NotAFileName {
                name: name.escape_debug().to_string(),
            });
        }
        let chars = name.chars().count();
        if chars > MAX_APP_NAME_CHARS {
            return Err(SourceError::NameTooLong { chars });
        }
        Ok(AppIdentity(name.to_lowercase()))
    }

    /// The file name at the end of a full executable path.
    ///
    /// The directory is dropped rather than stored. `C:\Users\罗伊\Desktop\
    /// 报税 2026\tax.exe` says who the user is and what they were doing; the
    /// product only asked for how long they were in `tax.exe`.
    pub fn from_executable_path(path: impl AsRef<str>) -> Result<AppIdentity, SourceError> {
        let path = path.as_ref();
        let file_name = path
            .rsplit(['\\', '/'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if file_name.is_empty() {
            return Err(SourceError::EmptyName);
        }
        AppIdentity::new(file_name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AppIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Something that can say which application is in the foreground right now.
///
/// `Ok(None)` means nothing is — a locked session, a desktop with no focused
/// window, or a process this build declines to identify. It ends the open
/// session rather than guessing at one.
pub trait ForegroundSource {
    fn sample(&mut self) -> Result<Option<AppIdentity>, SourceError>;

    /// A fixed label for reports and audit-free logs. Never user data.
    fn describe(&self) -> &'static str;
}

impl<T: ForegroundSource + ?Sized> ForegroundSource for Box<T> {
    fn sample(&mut self) -> Result<Option<AppIdentity>, SourceError> {
        (**self).sample()
    }

    fn describe(&self) -> &'static str {
        (**self).describe()
    }
}

/// The source for the machine this build is running on.
///
/// Windows gets the real one. Everywhere else this fails rather than returning
/// a source that quietly reports nothing: a collector that silently collects
/// nothing looks exactly like a working one, and the user would be told
/// collection was on.
#[cfg(windows)]
pub fn platform_source() -> Result<Box<dyn ForegroundSource + Send>, SourceError> {
    Ok(Box::new(crate::windows::WindowsForegroundSource::new()))
}

#[cfg(not(windows))]
pub fn platform_source() -> Result<Box<dyn ForegroundSource + Send>, SourceError> {
    Err(SourceError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_path_collapses_to_the_executable_name() {
        let app = AppIdentity::from_executable_path("C:\\Program Files\\Soul\\Soul.exe")
            .expect("a normal Windows path");
        assert_eq!(app.as_str(), "soul.exe");
    }

    #[test]
    fn the_directory_is_dropped_even_when_it_names_the_user() {
        let app = AppIdentity::from_executable_path("C:\\Users\\罗伊\\报税 2026\\tax.exe")
            .expect("a path with personal data in the directory");
        assert_eq!(app.as_str(), "tax.exe");
    }

    #[test]
    fn a_name_that_is_still_a_path_is_refused() {
        assert!(matches!(
            AppIdentity::new("C:\\Windows\\notepad.exe"),
            Err(SourceError::NotAFileName { .. })
        ));
    }

    #[test]
    fn a_sentence_is_not_an_application_name() {
        let title = "会议纪要 2026-08-24".repeat(20);
        assert!(matches!(
            AppIdentity::new(&title),
            Err(SourceError::NameTooLong { .. })
        ));
    }

    #[test]
    fn blank_and_control_characters_are_refused() {
        assert!(matches!(AppIdentity::new("   "), Err(SourceError::EmptyName)));
        assert!(matches!(
            AppIdentity::new("note\npad.exe"),
            Err(SourceError::NotAFileName { .. })
        ));
    }
}
