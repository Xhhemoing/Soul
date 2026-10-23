//! Explicit synthetic launch selection. The default path does not inspect data.
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::commands::SessionState;

#[derive(Debug)]
pub enum LaunchMode {
    Disabled,
    Synthetic(PathBuf),
}

impl LaunchMode {
    /// Pure configuration parser; tests never read the process environment.
    pub fn parse(mode: Option<&str>, root: Option<&str>) -> io::Result<Self> {
        match mode {
            None => Ok(Self::Disabled),
            Some("synthetic") => {
                let root = root
                    .map(PathBuf::from)
                    .filter(|root| root.is_absolute())
                    .ok_or_else(|| refused("absolute synthetic root required"))?;
                Ok(Self::Synthetic(root))
            }
            Some(_) => Err(refused("unknown local API mode")),
        }
    }

    pub fn from_environment() -> io::Result<Self> {
        let mode = match std::env::var("SOUL_LOCAL_API") {
            Ok(mode) => mode,
            Err(std::env::VarError::NotPresent) => return Ok(Self::Disabled),
            Err(_) => return Err(refused("invalid local API mode")),
        };
        let root = std::env::var("SOUL_API_SYNTHETIC_ROOT").ok();
        Self::parse(Some(&mode), root.as_deref())
    }
}

#[derive(Debug)]
pub struct SyntheticLaunch {
    run: PathBuf,
    data: PathBuf,
    profile: PathBuf,
}

impl SyntheticLaunch {
    pub fn create(root: &Path) -> io::Result<Self> {
        if !root.is_absolute() {
            return Err(refused("absolute synthetic root required"));
        }
        let metadata = std::fs::symlink_metadata(root)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(refused("synthetic root must be a directory"));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(refused("synthetic root cannot be a reparse point"));
            }
        }
        let root = root.canonicalize()?;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| refused("launch clock unavailable"))?
            .as_nanos();
        for _ in 0..16 {
            let run = root.join(format!(
                "run-{}-{stamp}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::create_dir(&run) {
                Ok(()) => {
                    let data = run.join("data");
                    let profile = run.join("webview");
                    std::fs::create_dir(&data)?;
                    std::fs::create_dir(&profile)?;
                    return Ok(Self { run, data, profile });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(refused("fresh synthetic run unavailable"))
    }

    pub fn data_directory(&self) -> &Path {
        &self.data
    }
    pub fn profile_directory(&self) -> &Path {
        &self.profile
    }

    /// Pins both WebView2 profile sources before any WebView is constructed.
    /// WebView2's process environment override takes precedence over the
    /// user-data directory passed through the window configuration. The host
    /// supplies the real process setter; synthetic tests capture it locally.
    pub fn configure_webview_profile(
        &self,
        windows: &mut [tauri::utils::config::WindowConfig],
        set_environment: impl FnOnce(&std::ffi::OsStr, &std::ffi::OsStr),
    ) {
        for window in windows {
            window.data_directory = Some(self.profile.clone());
        }
        set_environment(
            std::ffi::OsStr::new("WEBVIEW2_USER_DATA_FOLDER"),
            self.profile.as_os_str(),
        );
    }

    pub fn start(self) -> io::Result<(SessionState, LocalApiHost)> {
        // Open once, inside newly created run/data. No platform fallback and
        // no pre-existing config/DB is ever selected by this factory.
        let session = soulcore::commands::session::Session::open(&self.data);
        if !session.status().store_opened {
            return Err(refused("synthetic store unavailable"));
        }
        let runtime = soul_runtime::Runtime::new_synthetic(session);
        let server = soul_api::ApiServer::start(runtime.clone())?;
        let credentials = self.run.join("credentials.json");
        if server.write_credentials_new(&credentials).is_err() {
            server.shutdown();
            return Err(refused("protected credential creation failed"));
        }
        Ok((
            SessionState::from_runtime(runtime),
            LocalApiHost {
                server,
                credentials,
                cleanup: Mutex::new(false),
            },
        ))
    }
}

#[derive(Debug)]
pub struct LocalApiHost {
    server: soul_api::ApiServer,
    credentials: PathBuf,
    cleanup: Mutex<bool>,
}

impl LocalApiHost {
    pub fn credentials_path(&self) -> &Path {
        &self.credentials
    }
    pub fn request_stop(&self) {
        self.server.request_stop();
    }
    pub fn shutdown(&self) {
        self.server.shutdown();
        let mut removed = self
            .cleanup
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !*removed {
            match std::fs::remove_file(&self.credentials) {
                Ok(()) => *removed = true,
                Err(error) if error.kind() == io::ErrorKind::NotFound => *removed = true,
                Err(_) => eprintln!("soul: local API credential cleanup failed"),
            }
        }
    }
}

impl Drop for LocalApiHost {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn refused(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}
