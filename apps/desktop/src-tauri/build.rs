//! The application manifest is supplied here rather than left to Tauri's
//! default, because `asInvoker` is one of the things v0.1 promises and a
//! promise kept by someone else's default is one nobody notices losing.

fn main() {
    let windows = tauri_build::WindowsAttributes::new()
        .app_manifest(include_str!("windows/soul.exe.manifest"));

    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("tauri-build could not process tauri.conf.json");
}
