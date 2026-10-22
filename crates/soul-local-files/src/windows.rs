//! The only platform-specific operations: checking a drive and changing an
//! owned file handle's deletion disposition. No path-based cleanup.

#![allow(unsafe_code)]

use std::{
    ffi::c_void,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
const FILE_READ_ATTRIBUTES: u32 = 0x80;
const FILE_LIST_DIRECTORY: u32 = 1;
const DELETE: u32 = 0x0001_0000;
const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const SHARE_READ: u32 = 1;
const DRIVE_FIXED: u32 = 3;

#[link(name = "kernel32")]
extern "system" {
    fn GetDriveTypeW(root_path: *const u16) -> u32;
    fn QueryDosDeviceW(device_name: *const u16, target: *mut u16, capacity: u32) -> u32;
    fn SetFileInformationByHandle(
        handle: *mut c_void,
        class: i32,
        data: *const c_void,
        size: u32,
    ) -> i32;
}

struct PinnedDestination {
    path: PathBuf,
    _ancestors: Vec<File>,
}

pub struct Destination(Arc<PinnedDestination>);

impl std::fmt::Debug for Destination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Destination").finish_non_exhaustive()
    }
}

pub struct PendingFile<'a> {
    // Declare the file first so it closes before the pinned directories.
    file: File,
    _destination: Arc<PinnedDestination>,
    cancelled: &'a AtomicBool,
}

impl std::fmt::Debug for PendingFile<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingFile").finish_non_exhaustive()
    }
}

impl Destination {
    pub fn prepare(path: &Path) -> io::Result<Self> {
        let parts = validate_path(path)?;
        let root_path = PathBuf::from(format!("{}\\", parts[0]));
        let root: Vec<u16> = root_path.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: root is NUL terminated and remains alive for this read-only call.
        if unsafe { GetDriveTypeW(root.as_ptr()) } != DRIVE_FIXED {
            return Err(invalid("destination must be on a fixed local drive"));
        }
        // Resolve the drive once, then use its direct volume device for every
        // subsequent open. A later DOS drive-letter remapping cannot redirect
        // the write. SUBST mappings and non-volume devices are refused.
        let volume = direct_volume(&parts[0])?;
        let mut current = PathBuf::from(format!(r"\\?\GLOBALROOT{volume}\"));
        let mut ancestors = Vec::new();
        ancestors.push(pin_directory(&current)?);
        for component in &parts[1..parts.len() - 1] {
            current.push(component);
            ancestors.push(pin_directory(&current)?);
        }
        current.push(parts.last().expect("validated nonempty destination"));
        match fs::symlink_metadata(&current) {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "destination already exists",
                ))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        Ok(Self(Arc::new(PinnedDestination {
            path: current,
            _ancestors: ancestors,
        })))
    }

    pub fn write<'a>(
        &self,
        bytes: &[u8],
        cancelled: &'a AtomicBool,
    ) -> io::Result<PendingFile<'a>> {
        self.write_using(cancelled, |file| {
            file.write_all(bytes)?;
            file.sync_all()
        })
    }

    fn write_using<'a>(
        &self,
        cancelled: &'a AtomicBool,
        writer: impl FnOnce(&mut File) -> io::Result<()>,
    ) -> io::Result<PendingFile<'a>> {
        check_cancelled(cancelled)?;
        let file = OpenOptions::new()
            .write(true)
            .access_mode(GENERIC_READ | GENERIC_WRITE | DELETE)
            .share_mode(0)
            .create_new(true)
            .open(&self.0.path)?;
        // No content is written until deletion is armed. In the unlikely
        // failure here, leaving an empty file is safer than deleting by path.
        disposition(&file, true)?;
        let mut pending = PendingFile {
            file,
            _destination: Arc::clone(&self.0),
            cancelled,
        };
        writer(&mut pending.file)?;
        check_cancelled(cancelled)?;
        Ok(pending)
    }
}

impl PendingFile<'_> {
    pub fn publish(self) -> io::Result<()> {
        check_cancelled(self.cancelled)?;
        // Clearing this flag is the publication point. No subsequent fallible
        // operation can turn a successful publication into an error. File Drop
        // only closes the handle; it never re-arms deletion after this succeeds.
        disposition(&self.file, false)
    }
}

fn disposition(file: &File, delete: bool) -> io::Result<()> {
    // FILE_DISPOSITION_INFO contains one Windows BOOLEAN (u8), not BOOL (i32).
    let info: u8 = u8::from(delete);
    // SAFETY: the handle is owned/live with DELETE access. info points to the
    // one-byte FILE_DISPOSITION_INFO for class 4 and lives throughout the call.
    let result = unsafe {
        SetFileInformationByHandle(file.as_raw_handle(), 4, (&info as *const u8).cast(), 1)
    };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn pin_directory(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        // Attribute-only handles do not enforce sharing restrictions. Include
        // directory read access so denying write/delete really pins this node.
        .access_mode(FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY)
        .share_mode(SHARE_READ)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(invalid(
            "destination ancestors must be ordinary directories",
        ));
    }
    Ok(file)
}

fn validate_path(path: &Path) -> io::Result<Vec<String>> {
    let text = path
        .to_str()
        .ok_or_else(|| invalid("destination must be a Unicode path"))?;
    let normalized = text.replace('/', "\\");
    let parts: Vec<String> = normalized.split('\\').map(str::to_owned).collect();
    if parts.len() < 2
        || parts[0].len() != 2
        || !parts[0].as_bytes()[0].is_ascii_alphabetic()
        || parts[0].as_bytes()[1] != b':'
    {
        return Err(invalid("destination must be an absolute drive path"));
    }
    for part in &parts[1..] {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part.chars().any(|c| c < ' ' || "<>:\"|?*".contains(c))
        {
            return Err(invalid("destination has an unsafe path component"));
        }
        let stem = part
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&stem.as_str())
            || ["COM", "LPT"].iter().any(|prefix| {
                stem.strip_prefix(prefix).is_some_and(|suffix| {
                    matches!(
                        suffix,
                        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                    )
                })
            })
        {
            return Err(invalid("destination may not name a device"));
        }
    }
    Ok(parts)
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn check_cancelled(cancelled: &AtomicBool) -> io::Result<()> {
    if cancelled.load(Ordering::Acquire) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "publication cancelled",
        ))
    } else {
        Ok(())
    }
}

fn direct_volume(drive: &str) -> io::Result<String> {
    let name: Vec<u16> = drive.encode_utf16().chain(Some(0)).collect();
    let mut target = vec![0_u16; 1024];
    // SAFETY: name is NUL terminated and target is writable for its declared
    // capacity. Both buffers live for this synchronous call.
    let written =
        unsafe { QueryDosDeviceW(name.as_ptr(), target.as_mut_ptr(), target.len() as u32) };
    if written == 0 {
        return Err(io::Error::last_os_error());
    }
    // QueryDosDevice can return historical mappings after the first string;
    // the first NUL-terminated string is the current mapping.
    let first = target[..written as usize]
        .split(|c| *c == 0)
        .next()
        .unwrap_or_default();
    let device = String::from_utf16(first).map_err(|_| invalid("invalid volume device"))?;
    if !is_fixed_volume_device(&device) {
        return Err(invalid("destination must use a direct local volume"));
    }
    Ok(device)
}

fn is_fixed_volume_device(device: &str) -> bool {
    device
        .strip_prefix(r"\Device\HarddiskVolume")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_direct_volume_devices_are_accepted() {
        assert!(is_fixed_volume_device(r"\Device\HarddiskVolume3"));
        assert!(is_fixed_volume_device(r"\Device\HarddiskVolume123"));
        for device in [
            r"\??\C:\synthetic",
            r"\Device\LanmanRedirector\server\share",
            r"\Device\HarddiskVolume",
            r"\Device\HarddiskVolume3\synthetic",
        ] {
            assert!(!is_fixed_volume_device(device));
        }
    }

    #[test]
    fn partial_write_failure_deletes_owned_file() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("partial.zip");
        let destination = Destination::prepare(&target).unwrap();
        let cancelled = AtomicBool::new(false);
        let result = destination.write_using(&cancelled, |file| {
            file.write_all(b"synthetic partial bytes")?;
            Err(io::Error::other("synthetic write failure"))
        });
        assert!(result.is_err());
        assert!(!target.exists());
    }

    #[test]
    fn failure_after_write_and_sync_deletes_owned_file() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("sync.zip");
        let destination = Destination::prepare(&target).unwrap();
        let cancelled = AtomicBool::new(false);
        let result = destination.write_using(&cancelled, |file| {
            file.write_all(b"synthetic full bytes")?;
            file.sync_all()?;
            Err(io::Error::other("synthetic completion failure"))
        });
        assert!(result.is_err());
        assert!(!target.exists());
    }
}
