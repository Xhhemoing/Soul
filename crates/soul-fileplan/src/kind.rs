//! What kind of file this looks like, judged only by the name.
//!
//! By the name and nothing else. Reading a byte of a file to classify it would
//! put the user's documents through this crate's own logic, and a read-only
//! promise that still opens every file is a smaller promise than it sounds.
//! An extension is a guess, so the preview says it is grouping by extension
//! rather than claiming to know what anything is.

/// The groups v0.1 is prepared to propose folders for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Image,
    Document,
    Sheet,
    Slides,
    Archive,
    Audio,
    Video,
    Code,
    /// Anything the table below does not recognise. Never moved.
    Unrecognised,
}

impl FileKind {
    pub const ALL: &'static [FileKind] = &[
        FileKind::Image,
        FileKind::Document,
        FileKind::Sheet,
        FileKind::Slides,
        FileKind::Archive,
        FileKind::Audio,
        FileKind::Video,
        FileKind::Code,
        FileKind::Unrecognised,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            FileKind::Image => "image",
            FileKind::Document => "document",
            FileKind::Sheet => "sheet",
            FileKind::Slides => "slides",
            FileKind::Archive => "archive",
            FileKind::Audio => "audio",
            FileKind::Video => "video",
            FileKind::Code => "code",
            FileKind::Unrecognised => "unrecognised",
        }
    }

    /// The folder a file of this kind would be proposed into, relative to the
    /// authorized root. `None` means this crate has no opinion and will leave
    /// the file where it is.
    pub const fn folder(self) -> Option<&'static str> {
        match self {
            FileKind::Image => Some("图片"),
            FileKind::Document => Some("文档"),
            FileKind::Sheet => Some("表格"),
            FileKind::Slides => Some("演示"),
            FileKind::Archive => Some("压缩包"),
            FileKind::Audio => Some("音频"),
            FileKind::Video => Some("视频"),
            FileKind::Code => Some("代码"),
            FileKind::Unrecognised => None,
        }
    }

    /// Every folder name this crate can propose, so the plan can tell a file
    /// that is already sorted from one that is not.
    pub fn folders() -> Vec<&'static str> {
        FileKind::ALL.iter().filter_map(|k| k.folder()).collect()
    }

    /// The kind whose folder is `folder`, if any.
    pub fn for_folder(folder: &str) -> Option<FileKind> {
        FileKind::ALL
            .iter()
            .copied()
            .find(|kind| kind.folder() == Some(folder))
    }

    /// Classify by lower-case extension, without the dot.
    pub fn of_extension(extension: Option<&str>) -> FileKind {
        let Some(extension) = extension else {
            return FileKind::Unrecognised;
        };
        match extension {
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "heic" | "tif" | "tiff" | "svg" => {
                FileKind::Image
            }
            "pdf" | "doc" | "docx" | "odt" | "rtf" | "txt" | "md" | "epub" => FileKind::Document,
            "xls" | "xlsx" | "ods" | "csv" | "tsv" => FileKind::Sheet,
            "ppt" | "pptx" | "odp" | "key" => FileKind::Slides,
            "zip" | "rar" | "7z" | "gz" | "bz2" | "xz" | "tar" | "iso" => FileKind::Archive,
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" => FileKind::Audio,
            "mp4" | "mkv" | "mov" | "avi" | "wmv" | "webm" | "m4v" => FileKind::Video,
            "rs" | "py" | "ts" | "tsx" | "js" | "jsx" | "java" | "go" | "c" | "h" | "cpp"
            | "hpp" | "cs" | "sh" | "ps1" | "sql" | "json" | "toml" | "yaml" | "yml" => {
                FileKind::Code
            }
            _ => FileKind::Unrecognised,
        }
    }

    /// The extension of a file name, lower-cased, if it has one worth using.
    ///
    /// A leading dot is not an extension: `.gitignore` is a name.
    pub fn extension_of(name: &str) -> Option<String> {
        let (stem, extension) = name.rsplit_once('.')?;
        if stem.is_empty() || extension.is_empty() {
            return None;
        }
        Some(extension.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dotfile_has_no_extension_to_group_by() {
        assert_eq!(FileKind::extension_of(".gitignore"), None);
        assert_eq!(FileKind::extension_of("Makefile"), None);
        assert_eq!(FileKind::extension_of("notes."), None);
        assert_eq!(FileKind::extension_of("photo.JPG").as_deref(), Some("jpg"));
        assert_eq!(
            FileKind::extension_of("archive.tar.gz").as_deref(),
            Some("gz"),
        );
    }

    #[test]
    fn every_folder_name_maps_back_to_exactly_one_kind() {
        for kind in FileKind::ALL {
            let Some(folder) = kind.folder() else {
                continue;
            };
            assert_eq!(FileKind::for_folder(folder), Some(*kind));
        }
        assert_eq!(FileKind::folders().len(), FileKind::ALL.len() - 1);
    }

    #[test]
    fn an_unknown_extension_is_left_alone_rather_than_guessed_at() {
        assert_eq!(FileKind::of_extension(Some("qqq")), FileKind::Unrecognised,);
        assert_eq!(FileKind::of_extension(None), FileKind::Unrecognised);
        assert_eq!(FileKind::Unrecognised.folder(), None);
    }
}
