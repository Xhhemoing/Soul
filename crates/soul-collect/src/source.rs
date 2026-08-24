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

/// What a Windows executable image is called.
///
/// v0.1 collects on Windows only, and the name always arrives from the image
/// path of a running process, so it always ends in one of these. Requiring it
/// is what turns "we do not collect captions" from a habit into a rule: a
/// window title almost never ends in `.exe`, and one that does is still only a
/// name. Android in v0.3 has package names rather than image names and will
/// need its own constructor; it must not get one by loosening this.
pub const EXECUTABLE_SUFFIXES: &[&str] = &[".exe", ".com", ".scr"];

/// Why a sample was refused.
///
/// None of these carries the offending value. An error about a rejected sample
/// travels to logs and to the user interface, and the rejected value is exactly
/// the thing this crate has promised not to keep.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    #[error("a foreground sample carried no application name")]
    EmptyName,

    #[error("an application name may not contain a path, a drive or a control character")]
    NotAFileName,

    #[error(
        "an application name of {chars} characters is longer than the {MAX_APP_NAME_CHARS} \
         a program name can be; window titles are not collected"
    )]
    NameTooLong { chars: usize },

    #[error(
        "an application name must be an executable image name; \
         only the program is collected, never the window it is showing"
    )]
    NotAnExecutableName,

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
        if name.contains(['/', '\\', ':']) || name.chars().any(char::is_control) {
            return Err(SourceError::NotAFileName);
        }
        let chars = name.chars().count();
        if chars > MAX_APP_NAME_CHARS {
            return Err(SourceError::NameTooLong { chars });
        }
        let lowered = name.to_lowercase();
        if !EXECUTABLE_SUFFIXES
            .iter()
            .any(|suffix| lowered.ends_with(suffix))
        {
            return Err(SourceError::NotAnExecutableName);
        }
        Ok(AppIdentity(lowered))
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
            Err(SourceError::NotAFileName)
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
    fn a_window_caption_is_not_an_application_name() {
        for caption in [
            "报税 2026.xlsx - Excel",
            "Re: 合同条款 - 邮件",
            "soul — private notes — Visual Studio Code",
            "Excel",
        ] {
            assert!(
                matches!(
                    AppIdentity::new(caption),
                    Err(SourceError::NotAnExecutableName | SourceError::NotAFileName)
                ),
                "`{caption}` was accepted as an application name",
            );
        }
    }

    #[test]
    fn an_executable_whose_name_has_a_space_in_it_is_still_an_executable() {
        // Adobe ships several. Refusing every name with a space would be a
        // simpler rule and would quietly stop collecting real applications.
        let app = AppIdentity::new("Adobe Premiere Pro.exe").expect("a real program name");
        assert_eq!(app.as_str(), "adobe premiere pro.exe");
    }

    #[test]
    fn blank_and_control_characters_are_refused() {
        assert!(matches!(
            AppIdentity::new("   "),
            Err(SourceError::EmptyName)
        ));
        assert!(matches!(
            AppIdentity::new("note\npad.exe"),
            Err(SourceError::NotAFileName)
        ));
    }
}
