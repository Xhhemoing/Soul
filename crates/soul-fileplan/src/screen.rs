//! What a path has to look like before this crate will consider reading it.
//!
//! Everything here works on the raw string rather than on [`std::path::Path`],
//! and that is the point. `Path` parses differently on every platform: on
//! Linux, `\\?\C:\Windows` is one relative component with no drive, no root and
//! no separators, so a screen written against `Path::components` would report
//! that string as harmless on the CI host and dangerous only on the machine
//! the product actually ships to. AC-18 asks for a refusal that can be
//! demonstrated, so the rules are spelled out over characters and hold the same
//! on both.
//!
//! The rules are a list of ways two different strings can name the same file,
//! or a different one than they appear to:
//!
//! * `..` walks out of the authorized root, and `.` is a name that is not one;
//! * `\\server\share`, `\\?\` and `\\.\` reach network shares, bypass Win32
//!   path normalization, and open devices respectively;
//! * `NUL`, `CON`, `COM1` and their relatives are devices no matter which
//!   directory they appear to be in, and opening one can block forever;
//! * Windows strips trailing dots and spaces, so `secret.txt.` and `secret.txt`
//!   are one file with two spellings;
//! * a colon after the drive letter opens an alternate data stream, which is a
//!   second file hiding behind the name of the first.
//!
//! Several of these are Windows facts being enforced on Linux too. That is
//! deliberate: Linux is only a CI host for this product, and a rule that is
//! only active on the platform without the tests is a rule with no tests.

use std::fmt;
use std::path::PathBuf;

/// Longer than any path either supported platform will open, and short enough
/// that a pathological string does not turn a refusal into a long walk.
pub const MAX_PATH_CHARS: usize = 4_096;

/// Names that address a device rather than a file, in any directory.
pub const RESERVED_DEVICE_NAMES: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9", "conin$",
    "conout$",
];

/// Why a path was refused before anything on disk was touched.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathDefect {
    #[error("路径为空")]
    Empty,
    #[error("`{0}` 不是本机绝对路径")]
    NotAbsolute(String),
    #[error("路径里有 `..` 段，它可以指向任何地方")]
    ParentSegment,
    #[error("路径里有 `.` 段，它不是一个名字")]
    CurrentSegment,
    #[error("以两个分隔符开头的路径指向网络共享或设备，本版本不读")]
    UncOrDevicePrefix,
    #[error("路径里有空的一段（连续分隔符）")]
    RepeatedSeparator,
    #[error("`{0}` 是保留设备名，无论它看起来在哪个目录里")]
    ReservedDeviceName(String),
    #[error("`{0}` 以点或空格结尾，Windows 会悄悄去掉它")]
    TrailingDotOrSpace(String),
    #[error("`{0}` 里有冒号，那是备用数据流，不是文件名")]
    ColonInSegment(String),
    #[error("路径里有控制字符")]
    ControlCharacter,
    #[error("路径有 {0} 个字符，超过本版本会考虑的长度")]
    TooLong(usize),
}

/// What kind of absolute path this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPrefix {
    /// A POSIX path rooted at `/`.
    PosixRoot,
    /// A Windows path rooted at a drive letter, kept upper case because drive
    /// letters are case-insensitive on every filesystem that has them.
    Drive(char),
}

impl fmt::Display for PathPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathPrefix::PosixRoot => f.write_str("/"),
            PathPrefix::Drive(letter) => write!(f, "{letter}:\\"),
        }
    }
}

/// An absolute path that passed every rule above, split into its segments.
///
/// Holding the segments rather than a `PathBuf` is what lets containment be
/// decided without asking the platform to parse anything: two screened paths
/// are compared segment by segment, and the answer is the same on the CI host
/// as on the target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenedPath {
    prefix: PathPrefix,
    segments: Vec<String>,
}

impl ScreenedPath {
    pub fn prefix(&self) -> PathPrefix {
        self.prefix
    }

    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// The path as this platform would spell it.
    ///
    /// On Linux a [`PathPrefix::Drive`] path comes back relative, which is
    /// correct: there are no drive letters to open, and the caller finds out by
    /// failing to read it rather than by reading something else.
    pub fn to_path_buf(&self) -> PathBuf {
        let mut path = match self.prefix {
            PathPrefix::PosixRoot => PathBuf::from("/"),
            PathPrefix::Drive(letter) => PathBuf::from(format!("{letter}:\\")),
        };
        for segment in &self.segments {
            path.push(segment);
        }
        path
    }

    /// The path as it should be shown to the user, one spelling only.
    pub fn display(&self) -> String {
        match self.prefix {
            PathPrefix::PosixRoot => format!("/{}", self.segments.join("/")),
            PathPrefix::Drive(letter) => format!("{letter}:\\{}", self.segments.join("\\")),
        }
    }
}

/// Apply every rule to one path.
pub fn screen(raw: &str) -> Result<ScreenedPath, PathDefect> {
    if raw.is_empty() {
        return Err(PathDefect::Empty);
    }
    let length = raw.chars().count();
    if length > MAX_PATH_CHARS {
        return Err(PathDefect::TooLong(length));
    }
    if raw.chars().any(|c| c.is_control()) {
        return Err(PathDefect::ControlCharacter);
    }

    let bytes: Vec<char> = raw.chars().collect();
    if is_separator(bytes[0]) && bytes.get(1).copied().is_some_and(is_separator) {
        return Err(PathDefect::UncOrDevicePrefix);
    }

    let (prefix, rest) = if bytes[0] == '/' {
        (PathPrefix::PosixRoot, &raw[1..])
    } else if bytes[0] == '\\' {
        // A lone leading backslash is drive-relative on Windows: it resolves
        // against whichever drive the process happens to be on.
        return Err(PathDefect::NotAbsolute(shorten(raw)));
    } else if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == ':' {
        if !is_separator(bytes[2]) {
            // `C:relative` resolves against the process's per-drive current
            // directory, which is not a place the user authorized.
            return Err(PathDefect::NotAbsolute(shorten(raw)));
        }
        let letter = bytes[0].to_ascii_uppercase();
        (PathPrefix::Drive(letter), &raw[3..])
    } else {
        return Err(PathDefect::NotAbsolute(shorten(raw)));
    };

    // One trailing separator is how people write a directory, and it names the
    // same directory either way.
    let rest = rest.strip_suffix(['/', '\\']).unwrap_or(rest);

    let mut segments = Vec::new();
    if !rest.is_empty() {
        for segment in rest.split(['/', '\\']) {
            screen_segment(segment)?;
            segments.push(segment.to_owned());
        }
    }

    Ok(ScreenedPath { prefix, segments })
}

/// Apply the per-segment rules to one name.
///
/// Used on its own by the scan, which has to decide whether a file it found can
/// be named in a plan at all.
pub fn screen_segment(segment: &str) -> Result<(), PathDefect> {
    if segment.is_empty() {
        return Err(PathDefect::RepeatedSeparator);
    }
    if segment == ".." {
        return Err(PathDefect::ParentSegment);
    }
    if segment == "." {
        return Err(PathDefect::CurrentSegment);
    }
    if segment.chars().any(|c| c.is_control()) {
        return Err(PathDefect::ControlCharacter);
    }
    if segment.ends_with('.') || segment.ends_with(' ') {
        return Err(PathDefect::TrailingDotOrSpace(shorten(segment)));
    }
    if segment.contains(':') {
        return Err(PathDefect::ColonInSegment(shorten(segment)));
    }
    let stem = segment.split('.').next().unwrap_or(segment).to_lowercase();
    if RESERVED_DEVICE_NAMES.contains(&stem.as_str()) {
        return Err(PathDefect::ReservedDeviceName(shorten(segment)));
    }
    Ok(())
}

/// Bound what a refusal message repeats back.
///
/// A path can arrive from a file name, which is external content, and WP06
/// settled that a refusal explains itself without quoting what it refused at
/// length. The head of the string is enough to recognise which path was meant.
pub fn shorten(text: &str) -> String {
    const LIMIT: usize = 80;
    let cleaned: String = text.chars().filter(|c| !c.is_control()).collect();
    if cleaned.chars().count() <= LIMIT {
        return cleaned;
    }
    let head: String = cleaned.chars().take(LIMIT).collect();
    format!("{head}…")
}

fn is_separator(c: char) -> bool {
    c == '/' || c == '\\'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_posix_path_survives() {
        let screened = screen("/home/roy/文档").expect("a plain path");
        assert_eq!(screened.prefix(), PathPrefix::PosixRoot);
        assert_eq!(screened.segments(), ["home", "roy", "文档"]);
        assert_eq!(screened.display(), "/home/roy/文档");
    }

    #[test]
    fn a_drive_path_keeps_its_letter_upper_case_and_reads_both_separators() {
        let screened = screen(r"c:\Users\Roy/Docs").expect("a drive path");
        assert_eq!(screened.prefix(), PathPrefix::Drive('C'));
        assert_eq!(screened.segments(), ["Users", "Roy", "Docs"]);
        assert_eq!(screened.display(), r"C:\Users\Roy\Docs");
    }

    #[test]
    fn a_trailing_separator_names_the_same_directory() {
        assert_eq!(screen("/home/roy/"), screen("/home/roy"));
        assert_eq!(screen(r"C:\Users\"), screen(r"C:\Users"));
    }

    /// The same strings a Windows box would resolve to somewhere else, refused
    /// identically on the CI host.
    #[test]
    fn every_way_of_spelling_somewhere_else_is_refused() {
        for (raw, expected) in [
            ("", PathDefect::Empty),
            (
                "relative/path",
                PathDefect::NotAbsolute("relative/path".into()),
            ),
            (r"C:relative", PathDefect::NotAbsolute("C:relative".into())),
            (r"\windows", PathDefect::NotAbsolute(r"\windows".into())),
            ("/home/roy/../../etc", PathDefect::ParentSegment),
            (r"C:\Users\..\Windows", PathDefect::ParentSegment),
            ("/home/./roy", PathDefect::CurrentSegment),
            (r"\\server\share\x", PathDefect::UncOrDevicePrefix),
            (r"\\?\C:\Windows", PathDefect::UncOrDevicePrefix),
            (r"\\.\PhysicalDrive0", PathDefect::UncOrDevicePrefix),
            ("//server/share/x", PathDefect::UncOrDevicePrefix),
            ("/home//roy", PathDefect::RepeatedSeparator),
            (
                r"C:\Users\NUL",
                PathDefect::ReservedDeviceName("NUL".into()),
            ),
            (
                r"C:\Users\nul.txt",
                PathDefect::ReservedDeviceName("nul.txt".into()),
            ),
            (
                r"C:\Users\COM1",
                PathDefect::ReservedDeviceName("COM1".into()),
            ),
            (
                r"C:\Users\secret.txt.",
                PathDefect::TrailingDotOrSpace("secret.txt.".into()),
            ),
            (
                r"C:\Users\secret.txt ",
                PathDefect::TrailingDotOrSpace("secret.txt ".into()),
            ),
            (
                r"C:\Users\notes.txt:hidden",
                PathDefect::ColonInSegment("notes.txt:hidden".into()),
            ),
            ("/home/roy/a\u{7}b", PathDefect::ControlCharacter),
        ] {
            assert_eq!(screen(raw).unwrap_err(), expected, "screening `{raw}`");
        }
    }

    #[test]
    fn a_path_longer_than_the_limit_is_refused_by_length_not_by_content() {
        let long = format!("/{}", "a".repeat(MAX_PATH_CHARS));
        assert_eq!(
            screen(&long).unwrap_err(),
            PathDefect::TooLong(MAX_PATH_CHARS + 1),
        );
    }

    #[test]
    fn the_root_itself_screens_to_no_segments() {
        assert_eq!(screen("/").expect("root").segments(), &[] as &[String]);
        assert_eq!(
            screen(r"C:\").expect("drive root").segments(),
            &[] as &[String],
        );
    }

    #[test]
    fn a_refusal_does_not_repeat_the_whole_of_a_long_name() {
        let long = "x".repeat(500);
        assert!(shorten(&long).chars().count() <= 81);
    }
}
