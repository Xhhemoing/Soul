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
const LIB: &str = include_str!("../src/lib.rs");
const INSTANCE: &str = include_str!("../src/instance.rs");
const CARGO_TOML: &str = include_str!("../Cargo.toml");
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

/// The other way back into a Soul that is sitting in the tray, and the one
/// Windows users reach for first. A left click that opens a menu instead of the
/// window is Tauri's default, not the platform's convention, so both the
/// handler and the turned-off default are asserted: with the menu still on the
/// left button the click event is what the menu ate.
#[test]
fn a_left_click_on_the_tray_icon_reveals_the_window_too() {
    assert!(
        TRAY.contains("on_tray_icon_event"),
        "the tray answers menu events only, so a left click does nothing",
    );
    assert!(
        TRAY.contains("TrayIconEvent::Click")
            && TRAY.contains("button: MouseButton::Left")
            && TRAY.contains("button_state: MouseButtonState::Up"),
        "the tray icon handler does not single out the end of a left click",
    );

    let handler = block_after(TRAY, "on_tray_icon_event");
    assert!(
        handler.contains("reveal_main_window"),
        "the left click is handled but does not reveal the window: {handler}",
    );
    assert!(
        TRAY.contains(".show_menu_on_left_click(false)"),
        "Tauri's default puts the menu on the left button, which swallows the click",
    );
}

/// The text of the `{ ... }` block that follows `needle`, brace-matched so a
/// nested block does not end it early. Crude, and it is reading Rust that this
/// test cannot compile against: what is being checked is the shape of `run`,
/// and the shape is what a merge loses.
fn block_after<'a>(source: &'a str, needle: &str) -> &'a str {
    let at = source
        .find(needle)
        .unwrap_or_else(|| panic!("the source never says `{needle}`"));
    let rest = &source[at..];
    let open = rest.find('{').expect("a block follows");
    let mut depth = 0usize;
    for (offset, character) in rest[open..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &rest[open..open + offset + 1];
                }
            }
            _ => {}
        }
    }
    panic!("the block after `{needle}` is never closed");
}

/// The hole this closes: closing the window leaves Soul in the tray, so the
/// obvious way to get it back is to start Soul again — and the second process
/// used to open its own handle on `soul.db` before anything noticed. Two
/// SQLCipher connections on one file are two write-ahead logs.
///
/// Source order is the assertion because it is the thing that can regress: a
/// claim that happens after the store is opened is not a claim at all.
#[test]
fn the_launch_claims_the_instance_before_it_opens_a_store() {
    let claim = LIB
        .find("instance::claim()")
        .expect("run does not claim the single instance at all");
    let opened = LIB
        .find("open_platform_directory")
        .expect("run no longer opens the store; this test is reading the wrong file");
    assert!(
        claim < opened,
        "run opens the store before it claims the instance, so a second launch opens a second one",
    );
}

/// And having found the name taken, it leaves without touching anything.
#[test]
fn the_second_launch_leaves_without_opening_anything() {
    let arm = block_after(LIB, "instance::Claim::AlreadyRunning =>");
    assert!(
        arm.contains("return"),
        "the already-running arm falls through into the rest of run: {arm}",
    );
    assert!(
        arm.contains("instance::reveal_running_instance"),
        "the second launch exits without giving the user their window back: {arm}",
    );
    for forbidden in [
        "open_platform_directory",
        "open_store",
        "Session",
        "SessionState",
        "tauri::Builder",
    ] {
        assert!(
            !arm.contains(forbidden),
            "the already-running arm names `{forbidden}`; it may only reveal and leave",
        );
    }
}

/// A claim released at the end of the statement that made it is not a claim.
/// The guard has to be bound to a name that lives as long as `run` does.
#[test]
fn the_claim_is_held_for_as_long_as_the_process_runs() {
    let line = LIB
        .lines()
        .find(|line| line.contains("instance::claim()"))
        .expect("run claims the instance");
    let binding = line
        .trim()
        .strip_prefix("let ")
        .and_then(|rest| rest.split_once(" ="))
        .map(|(name, _)| name.trim())
        .unwrap_or_else(|| panic!("the claim is not bound to anything: {line}"));
    assert_ne!(
        binding, "_",
        "the guard is dropped where it is made, which releases the name immediately",
    );
    assert!(
        !LIB.contains(&format!("drop({binding})")),
        "the guard is dropped before run ends",
    );
}

/// `Local\` is the logon session. `Global\` would be the machine: two people
/// signed into the same PC each have their own `%LOCALAPPDATA%\Soul`, so each
/// is entitled to their own Soul, and the machine-wide namespace can also need
/// a privilege `soul.exe.manifest` promises not to ask for.
#[test]
fn the_instance_name_is_this_user_and_not_the_machine() {
    let identifier = config()["identifier"]
        .as_str()
        .expect("tauri.conf.json names the application")
        .to_owned();

    assert_eq!(soul_desktop::instance::APP_IDENTIFIER, identifier);
    assert_eq!(soul_desktop::instance::MUTEX_NAMESPACE, r"Local\");
    assert_eq!(
        soul_desktop::instance::MUTEX_NAME,
        format!(r"Local\{identifier}"),
        "the mutex is named after something other than this application",
    );
    assert!(
        !soul_desktop::instance::MUTEX_NAME.contains(r"Global\"),
        "a machine-wide name would keep a second signed-in user out of their own Soul",
    );
    // And that constant is the name that is really claimed, rather than one
    // the test can read while the call uses another.
    assert!(
        INSTANCE.contains("create_mutex(MUTEX_NAME)"),
        "instance.rs claims a name other than MUTEX_NAME",
    );
}

/// The window the second launch raises is the first launch's window, found by
/// the title the configuration gives it. If that title changes, the second
/// launch silently stops finding anything.
#[test]
fn the_second_launch_looks_for_the_window_the_configuration_titles() {
    let window = &config()["app"]["windows"][0];
    assert_eq!(
        window["label"].as_str(),
        Some(soul_desktop::tray::MAIN_WINDOW),
    );
    assert_eq!(
        window["title"].as_str(),
        Some(soul_desktop::instance::WINDOW_TITLE),
        "instance.rs looks for a window title the configuration does not use",
    );
    assert!(
        INSTANCE.contains("FindWindowW") && INSTANCE.contains("SetForegroundWindow"),
        "nothing in instance.rs finds the running window and brings it forward",
    );
}

/// The claim is a kernel object, which the operating system releases when the
/// process holding it dies however it dies. A lock file would be one more thing
/// in the data directory to explain, and after a power cut it would be a stale
/// file that keeps Soul from starting at all.
#[test]
fn the_claim_leaves_nothing_on_disk_next_to_the_keys() {
    assert!(
        INSTANCE.contains("CreateMutexW"),
        "the claim is not a named mutex any more",
    );
    let set_last = INSTANCE
        .find("ffi::SetLastError")
        .expect("CreateMutexW is not preceded by SetLastError(0)");
    let created = INSTANCE
        .find("ffi::CreateMutexW")
        .expect("the claim is not a named mutex any more");
    assert!(
        set_last < created,
        "a stale last-error of 183 would make the first launch look like a second",
    );
    for forbidden in [
        "soul.lock",
        "File::",
        "OpenOptions",
        "std::fs",
        "fs::write",
        "create_dir",
    ] {
        assert!(
            !INSTANCE.contains(forbidden),
            "instance.rs reaches for `{forbidden}`; the claim writes no file",
        );
    }
}

/// The single-instance check is seven Win32 declarations in this crate rather
/// than a plugin, and a plugin is how an HTTP client or a new capability would
/// arrive without anyone deciding to add one. `tests/no_egress_path.rs` names
/// the two plugins that are already banned; this is the wider rule, because
/// v0.1 uses no Tauri plugin at all.
#[test]
fn no_plugin_was_added_to_do_this() {
    for line in CARGO_TOML.lines().map(str::trim) {
        assert!(
            !line.starts_with("tauri-plugin"),
            "the shell depends on a Tauri plugin: {line}",
        );
    }
    assert_eq!(
        config()["plugins"],
        serde_json::json!({}),
        "a plugin would also need configuring, and v0.1 configures none",
    );
}

/// `unsafe` is allowed in exactly one module, and the crate root is what keeps
/// it there: `deny` on Windows so `instance.rs` can allow it back for its Win32
/// block, `forbid` everywhere else so it cannot be allowed back at all.
#[test]
fn the_only_unsafe_code_in_the_shell_is_the_win32_block() {
    assert!(
        LIB.contains("#![cfg_attr(not(windows), forbid(unsafe_code))]")
            && LIB.contains("#![cfg_attr(windows, deny(unsafe_code))]"),
        "the crate root no longer states what it does about unsafe code",
    );
    for (name, source) in [("lib.rs", LIB), ("tray.rs", TRAY)] {
        assert!(
            !source.contains("unsafe {") && !source.contains("unsafe impl"),
            "{name} contains unsafe code, which belongs in instance.rs",
        );
    }

    let win32 = INSTANCE
        .find("mod win32 {")
        .expect("instance.rs no longer keeps its Win32 calls in one module");
    for (at, _) in INSTANCE.match_indices("unsafe {") {
        assert!(at > win32, "instance.rs has an unsafe block outside win32");
    }
    for (at, _) in INSTANCE.match_indices("unsafe impl") {
        assert!(at > win32, "instance.rs has an unsafe impl outside win32");
    }
    assert!(
        INSTANCE.contains("#![allow(unsafe_code)]"),
        "the win32 module does not say that it is the exception",
    );
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
