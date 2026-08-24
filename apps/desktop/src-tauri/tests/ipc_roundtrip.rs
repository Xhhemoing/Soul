//! AC-02 and AC-22 over the real IPC.
//!
//! `soulcore` tests the decisions and vitest tests the screen; between them
//! sits the part neither can see — command registration, the camelCase to
//! snake_case argument conversion Tauri does, and the JSON the WebView
//! actually receives. Tauri's mock runtime has no window and no WebView, so
//! this runs on a CI host with no display while still going through
//! `invoke_handler`.
//!
//! The registration under test is `soul_desktop::configure`, which is the same
//! function the shipped binary calls. A second list here would test itself.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::{json, Value};
use soulcore::commands::session::Session;
use tauri::test::{get_ipc_response, mock_builder, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindowBuilder;

/// A directory no installation uses.
///
/// `Session::open` writes a configuration file and opens a database, so a test
/// that used the real data directory would be editing whatever is on the
/// machine running it. Every scratch directory here is its own, which is also
/// what lets a test say "restart" by opening a second session on the same one.
fn scratch() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let nth = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("soul-ipc-{}-{nth}", std::process::id(),));
    let _ = std::fs::remove_dir_all(&path);
    path
}

/// Call a command the way the WebView would, and return what it answered.
///
/// The context is the real one — `generate_context!` reads `tauri.conf.json`
/// and `capabilities/`, so the access-control list under test is the one that
/// ships. A synthetic context would allow or refuse commands on its own terms.
fn app_url<R: tauri::Runtime>(app: &tauri::App<R>) -> tauri::Url {
    app.config()
        .build
        .dev_url
        .clone()
        .expect("the configuration names a development server")
}

fn invoke(command: &str, body: Value) -> Result<Value, Value> {
    Shell::on(scratch()).invoke(command, body)
}

/// As [`invoke`], with the option to claim a different origin.
fn invoke_from(command: &str, body: Value, origin: Option<&str>) -> Result<Value, Value> {
    Shell::on(scratch()).invoke_from(command, body, origin)
}

/// One launched application: one session, one store handle, one WebView.
///
/// Most tests below want a single command and do not care what session it ran
/// against. Two do care. The endpoint draft path is prepare then generate, and
/// a second session would have no preparation to approve; and "the wizard
/// survives a restart" is only a claim about disk if the restart is a second
/// [`Shell`] on the same directory.
struct Shell {
    directory: PathBuf,
    webview: tauri::WebviewWindow<tauri::test::MockRuntime>,
    url: tauri::Url,
}

impl Shell {
    fn on(directory: impl Into<PathBuf>) -> Shell {
        let directory = directory.into();
        let session = Session::open(&directory);
        let app = soul_desktop::configure(
            mock_builder(),
            soul_desktop::commands::SessionState::new(session),
        )
        .build(tauri::generate_context!())
        .expect("the mock application builds");
        let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("a mock webview");
        let url = app_url(&app);
        Shell {
            directory,
            webview,
            url,
        }
    }

    /// Shut this one down and launch another on the same directory.
    fn restart(self) -> Shell {
        let directory = self.directory.clone();
        drop(self);
        Shell::on(directory)
    }

    fn invoke(&self, command: &str, body: Value) -> Result<Value, Value> {
        self.invoke_from(command, body, None)
    }

    fn invoke_from(
        &self,
        command: &str,
        body: Value,
        origin: Option<&str>,
    ) -> Result<Value, Value> {
        let url = match origin {
            Some(url) => url.parse().expect("a url"),
            None => self.url.clone(),
        };

        let response = get_ipc_response(
            &self.webview,
            InvokeRequest {
                cmd: command.to_owned(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url,
                body: body.into(),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_owned(),
            },
        );

        match response {
            Ok(value) => Ok(value.deserialize::<Value>().expect("a JSON body")),
            Err(value) => Err(value),
        }
    }
}

/// The snapshot the shell renders on launch has nothing switched on.
#[test]
fn the_configuration_that_crosses_the_ipc_is_fully_closed() {
    let snapshot = invoke("config_snapshot", json!({})).expect("the command answers");

    assert_eq!(snapshot["collect_enabled"], json!(false));
    assert_eq!(snapshot["llm_endpoint_configured"], json!(false));
    assert_eq!(snapshot["authorized_root_count"], json!(0));
    assert_eq!(snapshot["fully_closed"], json!(true));
    assert_eq!(snapshot["open_capabilities"], json!([]));
    assert_eq!(snapshot["cloud"]["state"], json!("not_yet_available"));
    assert_eq!(snapshot["cloud"]["label"], json!("尚未启用"));
}

/// AC-02, end to end: the wizard's answer is a closed configuration.
#[test]
fn a_finished_wizard_answers_with_everything_off() {
    let snapshot = invoke(
        "complete_wizard",
        json!({ "answers": { "acknowledged_defaults_are_off": true } }),
    )
    .expect("the wizard finishes");

    assert_eq!(snapshot["fully_closed"], json!(true));
    assert_eq!(snapshot["cloud"]["enabled"], json!(false));
}

/// WP13's second slice, over the IPC: the wizard is asked once.
///
/// Before this the shell held the answer in a React prop, so every launch was
/// a first launch. The restart here is a second application on the same
/// directory, which means the only thing carrying the answer across is the
/// file `Session` wrote.
#[test]
fn a_finished_wizard_is_still_finished_after_a_restart() {
    let shell = Shell::on(scratch());
    assert_eq!(
        shell.invoke("session_status", json!({})).expect("a status")["wizard_completed"],
        json!(false),
    );

    shell
        .invoke(
            "complete_wizard",
            json!({ "answers": { "acknowledged_defaults_are_off": true } }),
        )
        .expect("the wizard finishes");

    let restarted = shell.restart();
    let status = restarted
        .invoke("session_status", json!({}))
        .expect("a status");
    assert_eq!(status["wizard_completed"], json!(true));

    // AC-02: what came back from disk is the wizard's answer and nothing else.
    let snapshot = restarted
        .invoke("config_snapshot", json!({}))
        .expect("a snapshot");
    assert_eq!(snapshot["fully_closed"], json!(true));
    assert_eq!(snapshot["collect_enabled"], json!(false));
    assert_eq!(snapshot["llm_endpoint_configured"], json!(false));
    assert_eq!(snapshot["cloud"]["enabled"], json!(false));
}

/// The refusal reaches the WebView as an error rather than as a configuration
/// that quietly claims to be finished.
#[test]
fn an_unacknowledged_wizard_comes_back_as_an_error() {
    let refusal = invoke(
        "complete_wizard",
        json!({ "answers": { "acknowledged_defaults_are_off": false } }),
    )
    .expect_err("the wizard refuses");

    assert_eq!(refusal["reason"], json!("not_acknowledged"));
}

/// AC-22, end to end: asking for the cloud gets the same notice back.
#[test]
fn pressing_the_cloud_switch_returns_the_same_notice() {
    let asked_on = invoke("cloud_toggle", json!({ "requestedOn": true })).expect("an answer");
    let asked_off = invoke("cloud_toggle", json!({ "requestedOn": false })).expect("an answer");

    assert_eq!(asked_on, asked_off);
    assert_eq!(asked_on["enabled"], json!(false));
    assert_eq!(asked_on["state"], json!("not_yet_available"));
    assert_eq!(asked_on["label"], json!("尚未启用"));
    assert_eq!(asked_on["performs_network_request"], json!(false));
}

/// `requestedOn` is what `core.ts` sends. Tauri converts it to `requested_on`;
/// if that ever stopped being true the switch would silently receive `false`
/// and still look correct, so the conversion is asserted rather than assumed.
#[test]
fn the_webview_spelling_of_the_argument_is_the_one_that_arrives() {
    invoke("cloud_toggle", json!({ "requestedOn": true }))
        .expect("core.ts sends requestedOn, and it has to reach requested_on");

    assert!(
        invoke("cloud_toggle", json!({})).is_err(),
        "a command that tolerates a missing argument would hide a renamed one",
    );
}

/// WP10 over the IPC: a paste goes in, a draft comes back, and the JSON the
/// WebView receives has no field on it that could name a person.
#[test]
fn a_paste_comes_back_as_a_draft_with_nowhere_to_send_it() {
    let draft = invoke(
        "draft_reply",
        json!({ "pasted": "周五的场地我已经订好了，你直接过来就行" }),
    )
    .expect("drafting works with nothing configured");

    assert_eq!(draft["source"], json!("tone_template"));
    assert_eq!(draft["delivery"], json!(false));
    assert!(
        draft["text"].as_str().is_some_and(|text| !text.is_empty()),
        "a draft with no text is not a draft",
    );
    assert_eq!(
        draft["not_sent_notice"],
        json!(soulcore::commands::draft::NOT_SENT_NOTICE),
    );

    // The value the WebView holds is the one `soul-draft` pins, and there is
    // nothing on it a screen could read a recipient out of.
    let fields: Vec<&str> = draft
        .as_object()
        .expect("a record")
        .keys()
        .map(String::as_str)
        .collect();
    for field in &fields {
        for forbidden in ["recipient", "to", "address", "contact", "send", "channel"] {
            assert_ne!(*field, forbidden, "the draft carries a `{forbidden}`");
        }
    }
}

/// WP10's other half, over the IPC. What the confirmation screen is handed is
/// counts and two identifiers — no prose from anywhere, and above all not the
/// paste. The user approves a shape; they have already read the text.
#[test]
fn preparing_a_generation_describes_the_request_without_quoting_it() {
    let shell = Shell::on(scratch());
    let pasted = "周五的场地我已经订好了，你直接过来就行";

    let plan = shell
        .invoke("prepare_draft", json!({ "pasted": pasted }))
        .expect("a paste can always be described");

    assert_eq!(plan["third_party_turns"], json!(1));
    assert_eq!(plan["placeheld_turns"], json!(1));
    assert_eq!(plan["carries_exempted_original"], json!(false));
    assert_eq!(
        plan["notice"],
        json!(soulcore::commands::draft::E1_PLAN_NOTICE),
    );
    assert!(
        plan["plan_hash"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64),
        "a plan the user cannot echo back is one nobody can approve: {plan}",
    );

    // The counts are the request; the text is not in them. A plan carrying the
    // paste would put third-party words on a confirmation screen, which is the
    // one place they must not appear.
    let rendered = plan.to_string();
    assert!(
        !rendered.contains(pasted),
        "the plan quotes the paste: {plan}"
    );
    assert!(
        !rendered.contains("场地"),
        "the plan carries a fragment of the paste: {plan}",
    );

    // AC-02 holds through the confirmation: no endpoint was configured, so the
    // approval the user could give reaches nothing.
    let refusal = shell
        .invoke(
            "generate_draft",
            json!({
                "approval": {
                    "preparation_id": plan["preparation_id"],
                    "plan_hash": plan["plan_hash"],
                }
            }),
        )
        .expect_err("nothing is configured to generate against");
    assert!(
        refusal["reason_code"]
            .as_str()
            .is_some_and(|code| !code.is_empty()),
        "a refusal the screen cannot name is one it has to invent words for: {refusal}",
    );
}

/// A confirmation the user did not give sends nothing.
///
/// The preparation is real and the approval echoes the wrong hash, which is the
/// shape a replayed or tampered-with confirmation arrives in. `discard_draft`
/// is the other half of the same promise: the user read the plan and said no.
#[test]
fn an_approval_that_does_not_echo_the_plan_generates_nothing() {
    let shell = Shell::on(scratch());
    let plan = shell
        .invoke("prepare_draft", json!({ "pasted": "一句话" }))
        .expect("a plan");

    let refusal = shell
        .invoke(
            "generate_draft",
            json!({
                "approval": {
                    "preparation_id": plan["preparation_id"],
                    "plan_hash": "d5".repeat(32),
                }
            }),
        )
        .expect_err("a mismatched approval is refused");
    assert!(refusal["explanation"].is_string(), "unexpected: {refusal}");

    // And there is nothing left to approve a second time.
    assert_eq!(
        shell.invoke("discard_draft", json!({})).expect("an answer"),
        json!(false),
    );
}

/// An approval nobody prepared is the same refusal, arriving on a session that
/// has never been asked to prepare anything.
#[test]
fn an_approval_with_no_preparation_behind_it_generates_nothing() {
    let refusal = invoke(
        "generate_draft",
        json!({
            "approval": {
                "preparation_id": "0192f000-0000-7000-8000-0000000000f1",
                "plan_hash": "d5".repeat(32),
            }
        }),
    )
    .expect_err("an approval nobody prepared is refused");

    assert!(refusal["explanation"].is_string(), "unexpected: {refusal}");
}

/// The user read the plan and said no: the preparation goes away, and a later
/// approval of it reaches nothing.
#[test]
fn discarding_a_plan_leaves_nothing_an_approval_could_reach() {
    let shell = Shell::on(scratch());
    let plan = shell
        .invoke("prepare_draft", json!({ "pasted": "一句话" }))
        .expect("a plan");

    assert_eq!(
        shell.invoke("discard_draft", json!({})).expect("an answer"),
        json!(true),
        "there was a preparation to throw away",
    );
    assert!(
        shell
            .invoke(
                "generate_draft",
                json!({
                    "approval": {
                        "preparation_id": plan["preparation_id"],
                        "plan_hash": plan["plan_hash"],
                    }
                }),
            )
            .is_err(),
        "a discarded plan was still approvable",
    );
}

/// `approval` is a record rather than two arguments, so the conversion Tauri
/// does to it is the one that would silently drop half of a confirmation.
#[test]
fn the_approval_argument_is_required_and_must_carry_both_halves() {
    for body in [
        json!({}),
        json!({ "approval": { "preparation_id": "0192f000-0000-7000-8000-0000000000f1" } }),
        json!({ "approval": { "plan_hash": "d5".repeat(32) } }),
    ] {
        assert!(
            invoke("generate_draft", body.clone()).is_err(),
            "a half-filled approval resolved to something: {body}",
        );
    }
}

/// WP11 over the IPC. A directory nobody authorized is not scannable, and the
/// view the shell starts from says so with the core's own sentence.
#[test]
fn the_file_screen_starts_with_nothing_authorized_and_no_way_to_execute() {
    let shell = Shell::on(scratch());
    let view = shell.invoke("files_view", json!({})).expect("a view");

    assert_eq!(view["roots"], json!([]));
    assert_eq!(view["unavailable_roots"], json!([]));
    assert_eq!(view["executable_in_this_version"], json!(false));
    assert_eq!(
        view["read_only_notice"],
        json!(soulcore::commands::fileplan::READ_ONLY_NOTICE),
    );

    assert!(
        shell
            .invoke("preview_plan", json!({ "path": "/nowhere/in/particular" }))
            .is_err(),
        "an unauthorized directory was scanned",
    );
}

/// A directory the user named, the plan a scan of it produced, and the
/// authorization surviving a restart because it went to disk.
#[test]
fn an_authorized_directory_scans_read_only_and_is_remembered() {
    let shell = Shell::on(scratch());
    let root = scratch().join("下载");
    std::fs::create_dir_all(&root).expect("a directory to authorize");
    std::fs::write(root.join("预算.csv"), b"a,b\n1,2\n").expect("a file in it");

    let path = root.to_str().expect("a utf-8 path").to_owned();
    let view = shell
        .invoke("authorize_directory", json!({ "path": path }))
        .expect("the directory is authorizable");
    assert_eq!(view["roots"].as_array().map(Vec::len), Some(1));

    let plan = shell
        .invoke("preview_plan", json!({ "path": path }))
        .expect("an authorized directory scans");
    assert_eq!(plan["executable_in_this_version"], json!(false));
    assert_eq!(
        plan["read_only_notice"],
        json!(soulcore::commands::fileplan::READ_ONLY_NOTICE),
    );
    assert!(
        plan["plan_hash"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64),
        "a plan without a hash is not one anybody could approve: {plan}",
    );
    assert!(
        root.join("预算.csv").exists(),
        "the scan moved a file, and this version has no code that may",
    );

    // The authorization is in the file beside the store, not in this process.
    let restarted = shell.restart();
    let view = restarted.invoke("files_view", json!({})).expect("a view");
    assert_eq!(view["roots"].as_array().map(Vec::len), Some(1));

    let _ = std::fs::remove_dir_all(root.parent().unwrap_or(&root));
}

/// WP08 over the IPC. An empty store is an empty graph rather than an error,
/// and the notice on it is `soul-policy`'s, not a sentence written here.
#[test]
fn the_people_screen_reads_an_empty_store_as_an_empty_graph() {
    let graph = match invoke("people_graph", json!({})) {
        Ok(graph) => graph,
        // On a machine whose key provider could not produce a key the store
        // does not open, and a refusal is the honest answer. It must still be
        // a refusal with a code on it rather than an empty graph, because the
        // two must not look the same on screen.
        Err(refusal) => {
            assert!(refusal["reason_code"].is_string(), "unexpected: {refusal}");
            return;
        }
    };

    assert_eq!(graph["people"], json!([]));
    assert_eq!(graph["ties"], json!([]));
    assert_eq!(graph["third_party_data_is_local_only"], json!(true));
    assert_eq!(
        graph["notice"],
        json!(soulcore::commands::graph::WORKING_HYPOTHESIS_NOTICE),
    );
}

/// `contactId` is what `core.ts` sends, and Tauri has to make it `contact_id`.
#[test]
fn the_person_argument_is_required_and_spelled_the_way_the_webview_spells_it() {
    assert!(
        invoke("person_summary", json!({})).is_err(),
        "a summary of nobody in particular resolved to something",
    );
    // A well-formed identifier for a person who is not in the store: the
    // command has to answer, and the answer has to be a refusal.
    let refusal = invoke(
        "person_summary",
        json!({ "contactId": "0192f000-0000-7000-8000-000000000002" }),
    )
    .expect_err("nobody is in an empty store");
    assert!(refusal["reason_code"].is_string(), "unexpected: {refusal}");
}

/// One line of a fixture in this repository, read the way the WebView reads a
/// file the user picked: as text, from outside the core.
fn fixture(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join("fixtures")
        .join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The whole import path over the real IPC: counts out, then people.
///
/// This is the one thing neither side's own tests can see. `soulcore` proves
/// the file parses and lands sealed, and vitest proves the screen renders
/// counts, but only here is the file text carried across `invoke_handler` the
/// way an installed app carries it — and only here does the JSON that reaches
/// the WebView get searched for the sentences the export contained.
#[test]
fn an_export_crosses_the_ipc_as_counts_and_becomes_people() {
    let text = fixture("import/soul-import-v1/three_partners.jsonl");
    let shell = Shell::on(scratch());

    let preview = match shell.invoke("preview_soul_import_v1", json!({ "text": text })) {
        Ok(preview) => preview,
        // No key, no store, no import — and the screen has to be told which.
        Err(refusal) => {
            assert!(refusal["reason_code"].is_string(), "unexpected: {refusal}");
            return;
        }
    };
    assert_eq!(preview["source"], json!("soul-import-v1"));
    assert_eq!(preview["participants"], json!(5));
    assert_eq!(preview["messages"], json!(16));
    assert_eq!(preview["writes_anything"], json!(false));

    let receipt = shell
        .invoke("commit_soul_import_v1", json!({ "text": text }))
        .expect("the same text commits");
    assert_eq!(receipt["contacts_created"], json!(5));
    assert_eq!(receipt["events_written"], json!(16));
    assert_eq!(receipt["ties_rebuilt"], json!(4));

    let graph = shell.invoke("people_graph", json!({})).expect("a graph");
    assert_eq!(graph["people"].as_array().map(Vec::len), Some(5));

    // Nothing anybody wrote in that file came back across the IPC, on any of
    // the three answers the screen renders.
    let answered = format!("{preview}{receipt}{graph}");
    for line in text.lines().filter(|line| line.contains("\"text\"")) {
        let said = line
            .rsplit_once("\"text\":\"")
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(said, _)| said)
            .expect("every message line carries a body");
        assert!(
            !answered.contains(said),
            "the IPC answered with something the export said: {said}",
        );
    }

    let _ = std::fs::remove_dir_all(&shell.directory);
}

/// `text` is what `core.ts` sends, for all four import commands.
#[test]
fn the_import_argument_is_required_and_spelled_the_way_the_webview_spells_it() {
    for command in [
        "preview_soul_import_v1",
        "preview_telegram",
        "commit_soul_import_v1",
        "commit_telegram",
    ] {
        assert!(
            invoke(command, json!({})).is_err(),
            "{command} imported from a missing argument, which would hide a renamed one",
        );
        // Something that is not either format: the command has to answer, and
        // the answer has to be a refusal with a code on it rather than a panic.
        let refusal = invoke(command, json!({ "text": "not an export" }))
            .expect_err("neither format accepts that");
        assert!(refusal["reason_code"].is_string(), "unexpected: {refusal}");
    }
}

/// `pasted` is what `core.ts` sends. It is a single word, so Tauri's camelCase
/// conversion leaves it alone — which is exactly the kind of thing that is
/// true until somebody renames the argument, so it is asserted.
#[test]
fn the_paste_argument_is_required_and_spelled_the_way_the_webview_spells_it() {
    invoke("draft_reply", json!({ "pasted": "一句话" })).expect("core.ts sends `pasted`");
    assert!(
        invoke("draft_reply", json!({})).is_err(),
        "a command that drafts from a missing argument would hide a renamed one",
    );
}

/// ------------------------------------------------------------- collect ---
///
/// Collection over the real handler, including the restart. This is the half
/// `soulcore/tests/session_collect.rs` cannot reach: that one holds a
/// `Session` directly, so it proves the gate but not that the gate is
/// registered, spelled the way `core.ts` spells it, and answering with a JSON
/// object the screen can read.
///
/// A CI host has no foreground source, so the grant here records consent and
/// starts nothing — and the answer has to say both halves rather than the
/// comfortable one.
#[test]
fn collection_can_be_granted_and_taken_back_over_the_ipc() {
    let shell = Shell::on(scratch());

    let off = shell.invoke("collect_status", json!({})).expect("a status");
    assert_eq!(off["consent_granted"], json!(false));
    assert_eq!(off["collector_running"], json!(false));
    assert_eq!(off["survives_restart"], json!(false));

    let granted = shell
        .invoke("grant_collect_consent", json!({}))
        .expect("the store opened, so consent can be recorded");
    assert_eq!(granted["consent_granted"], json!(true));
    assert_eq!(
        granted["collector_running"],
        json!(cfg!(windows)),
        "only a machine with a foreground source may report a running collector",
    );

    let revoked = shell
        .invoke("revoke_collect_consent", json!({}))
        .expect("taking it back always works");
    assert_eq!(revoked["consent_granted"], json!(false));
    assert_eq!(revoked["collector_running"], json!(false));

    let _ = std::fs::remove_dir_all(&shell.directory);
}

/// AC-02 over the IPC: the consent does not come back with the application.
#[test]
fn a_restart_finds_collection_off_again() {
    let shell = Shell::on(scratch());
    assert_eq!(
        shell
            .invoke("grant_collect_consent", json!({}))
            .expect("consent is recorded")["consent_granted"],
        json!(true),
    );

    let next_launch = shell.restart();
    let status = next_launch
        .invoke("collect_status", json!({}))
        .expect("a status");
    assert_eq!(
        status["consent_granted"],
        json!(false),
        "a granted consent came back across a restart",
    );

    let config = std::fs::read_to_string(
        next_launch
            .directory
            .join(soulcore::commands::session::CONFIG_FILE_NAME),
    )
    .unwrap_or_default();
    for word in ["collect", "consent"] {
        assert!(
            !config.contains(word),
            "`{word}` reached config.json: {config}",
        );
    }

    let _ = std::fs::remove_dir_all(&next_launch.directory);
}

/// What crosses the IPC is counts and booleans. `CollectStatus` has no field
/// that could hold an application name, and this is where that is checked
/// against the JSON the WebView would actually receive.
#[test]
fn the_collection_status_carries_no_name_of_anything() {
    let status = invoke("collect_status", json!({})).expect("a status");
    let object = status.as_object().expect("an object");

    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "collector_running",
            "consent_granted",
            "duration_only_notice",
            "events_collected",
            "notice",
            "source",
            "survives_restart",
        ],
        "the collection status grew a field, and there is no field it could grow \
         that is not a name of something",
    );
    assert!(
        object["events_collected"].is_number() || object["events_collected"].is_null(),
        "the count is the only number the screen gets: {status}",
    );
}

/// The screen's empty state is the core's sentence, and its answer about
/// sending is fixed.
#[test]
fn the_notices_the_screen_starts_from_come_from_the_core() {
    let notices = invoke("draft_notices", json!({})).expect("an answer");

    assert_eq!(
        notices["not_sent"],
        json!(soulcore::commands::draft::NOT_SENT_NOTICE),
    );
    assert_eq!(notices["can_send"], json!(false));
}

/// A command the shell does not have must not resolve to something.
#[test]
fn an_unknown_command_is_refused() {
    assert!(invoke("execute_file_plan", json!({})).is_err());
}

/// The names a send button would have to be bound to. None of them exist.
#[test]
fn there_is_no_command_that_sends_a_draft_to_anybody() {
    for name in [
        "send_draft",
        "send_reply",
        "send_message",
        "deliver_draft",
        "post_draft",
    ] {
        assert!(
            invoke(name, json!({})).is_err(),
            "`{name}` resolved to something",
        );
    }
}

/// Nothing but the application's own page may reach the commands. The WebView
/// loads no remote page, so this is a second lock on a door that is already
/// shut — but it is the lock that would matter if the first one ever opened.
#[test]
fn a_call_claiming_a_different_origin_is_refused() {
    let refusal = invoke_from(
        "config_snapshot",
        json!({}),
        Some("https://example.invalid/page"),
    )
    .expect_err("a foreign origin gets nothing");
    assert!(
        refusal.to_string().contains("not allowed"),
        "unexpected refusal: {refusal}",
    );
}
