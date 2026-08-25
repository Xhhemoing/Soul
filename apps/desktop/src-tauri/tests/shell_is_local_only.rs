//! The shell's promises, read back off the files that make them.
//!
//! Three of the things v0.1 committed to are configuration rather than code —
//! no elevation, no updater, no remote page in the WebView — and configuration
//! is the easiest kind of promise to lose in a merge. So each one is asserted
//! here against the file that actually decides it.

use serde_json::Value;

const CONFIG: &str = include_str!("../tauri.conf.json");
const MANIFEST: &str = include_str!("../windows/soul.exe.manifest");
const INSTALLER_HOOKS: &str = include_str!("../windows/installer-hooks.nsh");
const BUILD_RS: &str = include_str!("../build.rs");
const TRAY: &str = include_str!("../src/tray.rs");
const CAPABILITY: &str = include_str!("../capabilities/default.json");

const WINDOWS_INSTALL_DIR: &str = r"$LOCALAPPDATA\Programs\Soul";

fn config() -> Value {
    serde_json::from_str(CONFIG).expect("tauri.conf.json is valid JSON")
}

/// Every key in the document, at any depth.
fn keys(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                found.push(key.clone());
                keys(child, found);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| keys(item, found)),
        _ => {}
    }
}

/// AC-01, the half a file can prove: the process asks for no more rights than
/// the user who started it already has.
#[test]
fn the_windows_manifest_asks_for_no_elevation() {
    assert!(
        MANIFEST.contains(r#"level="asInvoker""#),
        "the manifest must request asInvoker",
    );
    assert!(MANIFEST.contains(r#"uiAccess="false""#));
    assert!(
        !MANIFEST.contains("requireAdministrator") && !MANIFEST.contains("highestAvailable"),
        "no elevation level may appear in the manifest at all",
    );
}

/// A manifest nobody embeds is a comment. This checks the build script really
/// hands it to `tauri-build`.
#[test]
fn the_build_script_embeds_that_manifest() {
    assert!(BUILD_RS.contains(r#"include_str!("windows/soul.exe.manifest")"#));
    assert!(BUILD_RS.contains("app_manifest"));
}

/// AC-01's other half a file can prove: the labels the author-manual checklist
/// will look for. No runner has a notification area; this stops the strings
/// drifting from what that checklist names.
#[test]
fn the_tray_menu_is_the_one_the_manual_checklist_names() {
    assert!(TRAY.contains(r#"MenuItem::with_id(app, MENU_OPEN, "打开 Soul""#));
    assert!(TRAY.contains(r#"MenuItem::with_id(app, MENU_QUIT, "退出 Soul""#));
    assert!(TRAY.contains(r#".tooltip("Soul")"#));
}

/// PRODUCT_LOCK names the process. `soulcore` holds the name; the `[[bin]]`
/// target has to agree, and `env!` here fails to compile if it does not exist.
#[test]
fn the_binary_is_the_process_the_lock_document_names() {
    let path = std::path::Path::new(env!("CARGO_BIN_EXE_soul"));
    let stem = path.file_stem().and_then(|s| s.to_str()).expect("a stem");
    assert_eq!(stem, soulcore::commands::shell::DESKTOP_BINARY_NAME);
    assert_eq!(
        config()["mainBinaryName"],
        Value::String(soulcore::commands::shell::DESKTOP_BINARY_NAME.to_owned()),
    );
}

/// E0 is meant to have no code path. An updater is the classic way one appears
/// without anyone deciding to add it, so the word may not occur in the config.
#[test]
fn nothing_in_the_configuration_updates_itself() {
    let value = config();
    assert_eq!(
        value["bundle"]["createUpdaterArtifacts"],
        Value::Bool(false)
    );

    let mut found = Vec::new();
    keys(&value, &mut found);
    let updater_keys: Vec<&String> = found
        .iter()
        .filter(|key| {
            key.to_lowercase().contains("updater") && key.as_str() != "createUpdaterArtifacts"
        })
        .collect();
    assert!(
        updater_keys.is_empty(),
        "the configuration mentions an updater: {updater_keys:?}",
    );

    assert_eq!(
        value["plugins"],
        serde_json::json!({}),
        "v0.1 configures no plugins; an updater or HTTP plugin would arrive as one",
    );
}

/// The WebView shows files that shipped with the application. The content
/// security policy is what enforces it, so it may not name a remote origin —
/// and the check is the crude one on purpose: no scheme separator at all.
#[test]
fn the_webview_can_only_load_what_shipped_with_it() {
    let value = config();
    let security = &value["app"]["security"];

    let csp = security["csp"].as_str().expect("a csp is configured");
    assert!(csp.contains("default-src 'self'"));
    assert!(csp.contains("object-src 'none'"));
    assert!(csp.contains("frame-src 'none'"));
    assert!(
        !csp.contains("://"),
        "the policy names a remote origin: {csp}",
    );

    // `connect-src` is the one directive that could let the page talk to
    // something. Everything in it is either the page itself or the local IPC
    // bridge: Tauri reaches the Rust side with a fetch to `ipc://localhost` —
    // `http://ipc.localhost` on Windows — and a bare host source matches that
    // over the page's own scheme without naming one.
    let connect = csp
        .split(';')
        .map(str::trim)
        .find(|directive| directive.starts_with("connect-src"))
        .expect("connect-src is stated rather than inherited");
    for source in connect.split_whitespace().skip(1) {
        assert!(
            source == "'self'" || source == "ipc:" || source.ends_with(".localhost"),
            "connect-src allows something that is not the page or the local IPC bridge: {source}",
        );
    }

    assert_eq!(
        security["dangerousDisableAssetCspModification"],
        Value::Bool(false)
    );
    assert_eq!(
        security["assetProtocol"]["enable"],
        Value::Bool(false),
        "nothing outside the bundle needs to be readable through the asset protocol yet",
    );

    let frontend = value["build"]["frontendDist"]
        .as_str()
        .expect("a frontend location");
    assert!(
        frontend.starts_with("../") && !frontend.contains("://"),
        "the shipped frontend must be a path on disk, not a URL: {frontend}",
    );

    // The dev server is the one address the WebView may be pointed at, and it
    // is loopback. `xtask e0-audit` allows the same prefix in source.
    let dev = value["build"]["devUrl"].as_str().expect("a dev url");
    assert!(
        dev.starts_with("http://127.0.0.1"),
        "the development server must bind loopback: {dev}",
    );

    assert_eq!(value["app"]["withGlobalTauri"], Value::Bool(false));
}

/// A per-user install needs no administrator, and skipping the WebView2
/// bootstrapper means the installer downloads nothing. Windows 11 ships the
/// runtime; Windows 10 without it is a documented gap, not a silent download.
#[test]
fn the_installer_neither_elevates_nor_downloads() {
    let value = config();
    let windows = &value["bundle"]["windows"];
    assert_eq!(
        windows["nsis"]["installMode"],
        Value::String("currentUser".into())
    );
    assert_eq!(
        windows["webviewInstallMode"]["type"],
        Value::String("skip".into())
    );
}

/// NSIS lines that are not comment-only. A `StrCpy` tucked into a comment would
/// not run at install time, so the check ignores lines whose first token is `;`.
fn nsis_executable_lines(content: &str) -> Vec<&str> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(';'))
        .collect()
}

/// The executable lines of one `!macro NAME ... !macroend` block, so a hook can
/// be read on its own: `Abort` somewhere else in the file is not the uninstall
/// refusing.
fn nsis_macro_body<'a>(content: &'a str, name: &str) -> Vec<&'a str> {
    let opener = format!("!macro {name}");
    let mut lines = content.lines().map(str::trim);
    lines
        .find(|line| *line == opener)
        .unwrap_or_else(|| panic!("the hooks file has no `{opener}`"));

    let mut body = Vec::new();
    for line in lines {
        if line == "!macroend" {
            return body;
        }
        if !line.is_empty() && !line.starts_with(';') {
            body.push(line);
        }
    }
    panic!("`{opener}` is never closed");
}

/// PRODUCT_LOCK keeps soul.db and keys.dpapi under `%LOCALAPPDATA%\Soul`.
/// Tauri currentUser NSIS defaults to the same folder as the install dir;
/// these hooks force `%LOCALAPPDATA%\Programs\Soul` so uninstall cannot
/// RMDir the data directory by accident.
#[test]
fn the_installer_ships_into_programs_not_the_data_directory() {
    let value = config();
    let hooks_path = value["bundle"]["windows"]["nsis"]["installerHooks"]
        .as_str()
        .filter(|path| !path.is_empty())
        .expect("bundle.windows.nsis.installerHooks must be a non-empty path");
    assert!(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(hooks_path.strip_prefix("./").unwrap_or(hooks_path))
            .is_file(),
        "installer hooks file must exist relative to src-tauri: {hooks_path}",
    );

    let executable = nsis_executable_lines(INSTALLER_HOOKS);
    let joined = executable.join("\n");
    assert!(
        joined.contains(&format!(r#"StrCpy $INSTDIR "{WINDOWS_INSTALL_DIR}""#)),
        "PREINSTALL must force the Programs install directory, not the data dir",
    );

    let instdir_copy = executable
        .iter()
        .position(|line| line.contains(r#"StrCpy $INSTDIR "#) && line.contains(WINDOWS_INSTALL_DIR))
        .expect("PREINSTALL must StrCpy $INSTDIR to Programs\\Soul");
    assert!(
        executable
            .iter()
            .skip(instdir_copy + 1)
            .any(|line| line.contains("SetOutPath $INSTDIR")),
        "SetOutPath must follow the StrCpy because Tauri already SetOutPath once",
    );

    for line in &executable {
        let upper = line.to_ascii_uppercase();
        if upper.contains("RMDIR /R") {
            assert!(
                !line.contains(r"$LOCALAPPDATA\Soul")
                    && !line.contains(r"$LOCALAPPDATA\${PRODUCTNAME}"),
                "installer hooks must not recursively remove the data directory: {line}",
            );
        }
    }

    assert_eq!(
        soulcore::commands::session::WINDOWS_DIRECTORY_NAME,
        "Soul",
        "the data directory leaf is still Soul",
    );
    assert_ne!(
        soulcore::commands::session::WINDOWS_DIRECTORY_NAME,
        "Programs\\Soul",
        "install dir and data dir must not be the same path",
    );
    assert_ne!(WINDOWS_INSTALL_DIR, r"$LOCALAPPDATA\Soul");
}

/// Forcing the install directory only fixes installs this build makes. The
/// uninstaller carries whatever directory it was written with — an artifact
/// from before that hook, a restored previous install location — and Tauri runs
/// PREUNINSTALL before it deletes any file, so that hook is the last place that
/// can refuse to erase keys.dpapi.
#[test]
fn the_uninstall_refuses_to_run_in_the_data_directory() {
    let body = nsis_macro_body(INSTALLER_HOOKS, "NSIS_HOOK_PREUNINSTALL");
    assert!(
        !body.is_empty(),
        "PREUNINSTALL is empty, so an uninstaller pointed at the data directory would run",
    );
    assert!(
        body.iter()
            .any(|line| line.split_whitespace().next() == Some("Abort")),
        "PREUNINSTALL never aborts: {body:?}",
    );

    for spelling in [r"$LOCALAPPDATA\Soul", r"$LOCALAPPDATA\${PRODUCTNAME}"] {
        assert!(
            body.iter()
                .any(|line| line.starts_with("StrCmp")
                    && line.contains("$INSTDIR")
                    && line.contains(spelling)),
            "PREUNINSTALL never compares $INSTDIR against {spelling}: {body:?}",
        );
    }

    // A silent uninstall has nobody to answer a dialog, and `install-smoke.ps1`
    // runs `/S` unattended: a MessageBox here is a hang, not a warning.
    for line in &body {
        assert!(
            !line.to_ascii_uppercase().contains("MESSAGEBOX"),
            "PREUNINSTALL would block a silent uninstall on a dialog: {line}",
        );
    }

    // The hook's whole job is to not delete. Refusing is allowed; removing
    // anything on the way out is what this file exists to prevent.
    for hook in [
        "NSIS_HOOK_PREINSTALL",
        "NSIS_HOOK_POSTINSTALL",
        "NSIS_HOOK_PREUNINSTALL",
        "NSIS_HOOK_POSTUNINSTALL",
    ] {
        for line in nsis_macro_body(INSTALLER_HOOKS, hook) {
            let instruction = line.split_whitespace().next().unwrap_or_default();
            assert!(
                !instruction.eq_ignore_ascii_case("Delete")
                    && !instruction.eq_ignore_ascii_case("RMDir"),
                "{hook} removes files, which no hook has a reason to do: {line}",
            );
        }
    }
}

/// AC-01's tray icon comes from `default_window_icon()`, which Tauri fills from
/// `bundle.icon`. A path listed there and missing on disk fails the bundle, and
/// on a build that got past it the tray would never appear — the one thing the
/// author-manual checklist looks for by eye.
#[test]
fn every_bundle_icon_exists_on_disk() {
    let value = config();
    let icons = value["bundle"]["icon"].as_array().expect("bundle.icon");
    assert!(!icons.is_empty(), "a bundle with no icon has no tray icon");

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for icon in icons {
        let path = icon.as_str().expect("icon paths are strings");
        assert!(
            root.join(path).is_file(),
            "tauri.conf.json lists an icon that is not in the repository: {path}",
        );
    }

    assert!(
        icons
            .iter()
            .any(|icon| icon.as_str().is_some_and(|path| path.ends_with(".ico"))),
        "Windows takes the window and tray icon from the .ico",
    );
    assert!(
        TRAY.contains("default_window_icon()"),
        "the tray reads the bundled window icon, which is what makes this list load-bearing",
    );
}

/// Tauri's permission system is the other place an egress path could be
/// granted. Only core capabilities are allowed, so no filesystem, shell, HTTP
/// or updater plugin can be reached from the WebView even if one were added to
/// the manifest by accident.
#[test]
fn the_webview_holds_only_core_permissions() {
    let capability: Value = serde_json::from_str(CAPABILITY).expect("valid capability JSON");
    let permissions = capability["permissions"]
        .as_array()
        .expect("a permission list");
    assert!(!permissions.is_empty());

    for permission in permissions {
        let name = permission.as_str().expect("permissions are strings");
        assert!(
            name.starts_with("core:"),
            "only core permissions are granted, but the capability asks for {name}",
        );
    }
}
