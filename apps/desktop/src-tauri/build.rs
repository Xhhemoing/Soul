//! The application manifest is supplied here rather than left to Tauri's
//! default, because `asInvoker` is one of the things v0.1 promises and a
//! promise kept by someone else's default is one nobody notices losing.

fn main() {
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/windows/soul.exe.manifest");
    // Integration-test EXEs (ipc_roundtrip, …) do not go through Tauri's
    // WindowsAttributes path. Without Common-Controls 6.0 in their own
    // manifest they abort at load with STATUS_ENTRYPOINT_NOT_FOUND
    // (0xc0000139) on Windows. Link the same product manifest into tests
    // only on Windows/MSVC — soul.exe still gets it from tauri-build below.
    // Gate on Cargo *target* cfg (CARGO_CFG_TARGET_*), not host cfg!(windows),
    // so GNU/non-Windows targets never see MSVC-only /MANIFEST:* link args.
    println!("cargo:rerun-if-changed=windows/soul.exe.manifest");
    if emits_msvc_test_manifest_link_args(
        &std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default(),
        &std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default(),
    ) {
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-tests=/MANIFESTINPUT:{manifest}");
    }

    let windows = tauri_build::WindowsAttributes::new()
        .app_manifest(include_str!("windows/soul.exe.manifest"));

    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("tauri-build could not process tauri.conf.json");
}

fn emits_msvc_test_manifest_link_args(target_os: &str, target_env: &str) -> bool {
    target_os == "windows" && target_env == "msvc"
}
