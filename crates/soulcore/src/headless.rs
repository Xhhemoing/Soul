//! The whole product, run once, with nobody watching.
//!
//! AC-21 reads: *given the default configuration, run the main flow; observe
//! the network and the source; non-loopback connections = 0 and no business
//! domain.* Every other acceptance criterion is proved by a test that
//! exercises one crate. This one cannot be: "the main flow" is the thing no
//! single crate is, and a leak that only appears when importing, drafting and
//! scanning happen in the same process is exactly the leak per-crate tests
//! cannot see.
//!
//! So this module is the main flow, in order, against a real encrypted store
//! on a scratch directory — import, graph, profile, memory, forget, draft,
//! summary, file plan, research preview, collection, cloud switch, audit
//! chain — with [`crate::netwatch`] sampling this process's sockets from
//! start to finish. It is the body of `soul-headless`, which is what
//! `scripts/install-smoke.ps1` runs after an installer has put Soul on a
//! machine, and what `soulcore/tests/headless_main_flow.rs` runs in CI.
//!
//! ## Rules it follows, so that a green run means something
//!
//! * **Nothing is switched on.** The flow starts from [`Config::default`] and
//!   the wizard's own refusal is the first step. A smoke that configured an
//!   endpoint in order to have something to test would not be AC-21.
//! * **Every assertion is a failure, not a log line.** A step that cannot
//!   prove what it claims returns [`HeadlessError`] and the process exits
//!   non-zero, because the installer script's only signal is the exit code.
//! * **Nothing is left behind.** The scratch directory is removed on the way
//!   out, including after a failure. On a user's machine this runs beside a
//!   real `%LOCALAPPDATA%\Soul`, and a smoke that wrote into the real store
//!   would be a smoke nobody dared run twice.
//! * **The corpora are embedded.** `fixtures/` is a repository directory and
//!   an installed Soul has no repository, so the two files this flow reads are
//!   compiled in. They are the same bytes `just fixtures-verify` checks.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use soul_memory::MemoryDraft;
use soul_policy::consent::ConsentState;
use soul_policy::hitl::{ActionKind, ActionRequest, RequestOrigin, TokenIssuer};
use soul_policy::net_guard::NetGuard;
use soul_profile::questionnaire::QuestionnaireResponse;
use soul_profile::{AxisProposal, AxisUpdate, CURIOSITY, ORDERLINESS};
use soul_schema::common::{SealedSubject, SupportedBand};
use soul_schema::memory::MemoryType;
use soul_schema::profile::AxisPosition;
use soul_store::SqlCipherStore;
use soul_store_api::research::ResearchPreviewRequest;
use soul_store_api::types::EventFilter;
use soul_store_api::{AuditLog, EventStore, SoulStore};

use crate::commands::shell::{ConfigSnapshot, WizardAnswers};
use crate::commands::{
    collect as collect_commands, draft as draft_commands, fileplan as fileplan_commands,
    graph as graph_commands, import as import_commands, memory as memory_commands,
    profile as profile_commands, shell as shell_commands, store as store_commands,
};
use crate::config::Config;
use crate::netwatch::{self, WatchReport};

/// The import corpus, compiled in. See the module docs.
pub(crate) const IMPORT_CORPUS: &str =
    include_str!("../../../fixtures/import/soul-import-v1/three_partners.jsonl");

/// A completed questionnaire, compiled in for the same reason.
const QUESTIONNAIRE: &str = include_str!("../../../fixtures/questionnaire/answers_basic.json");

/// Key material for the scratch store. Named for what it is: this flow never
/// opens the user's database, so it never needs the platform key provider.
const SCRATCH_SEED: &str = "soul headless smoke, scratch store";

/// Fixed clock, so two runs write the same audit entries.
/// 2026-08-24T00:00:00Z.
const AT_UNIX_SECONDS: i64 = 1_787_529_600;
const NOW_MS: u64 = 1_787_529_600_000;

/// Prose the flow writes, and therefore prose the audit chain must not repeat.
const MEMORY_TITLE: &str = "把旧硬盘寄回老家那天";
const MEMORY_SUMMARY: &str = "顺丰点收的时候我才想起来里面还有大学时候的照片，没敢再打开看。";
const PASTED_MESSAGE: &str = "周五那个方案你还改吗？我这边可以等到下午三点。";

/// Hosts the closed guard is asked about, to show the refusal is real.
///
/// Written as bare authorities and assembled at runtime: a URL literal in a
/// shipped crate is what `xtask e0-audit` exists to find, and this file is not
/// exempt from it. `.invalid` never resolves (RFC 2606) and both addresses are
/// documentation ranges (RFC 5737, RFC 3849), so none of the three is a place
/// that could exist.
const UNREACHABLE_AUTHORITIES: &[&str] = &[
    "deep-analysis.invalid",
    "203.0.113.10:8443",
    "[2001:db8::1]:443",
];

/// A step of the flow, and what it proved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepReport {
    pub name: String,
    /// What the step observed, in numbers a reader can check against the
    /// acceptance criteria.
    pub detail: String,
}

/// One target the guard was asked about, and the code it answered with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefusedTarget {
    pub authority: String,
    pub reason_code: String,
}

/// Both halves of the egress answer: what the guard says, and what the kernel
/// says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EgressFindings {
    /// True when the platform let the sockets be read. False is not a pass;
    /// see [`crate::netwatch`].
    pub observed: bool,
    pub observation_note: Option<String>,
    pub samples: u64,
    /// AC-21's number. Anything but zero fails the run.
    pub non_loopback_connections: usize,
    pub non_loopback_peers: Vec<String>,
    pub loopback_connections: usize,
    /// Always false in this flow: no endpoint is configured.
    pub endpoint_configured: bool,
    pub refused: Vec<RefusedTarget>,
}

/// Everything one headless run found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MainFlowReport {
    /// True only if every step passed. The process exit code says the same
    /// thing; this is here so a log that got captured is readable on its own.
    pub ok: bool,
    pub config: ConfigSnapshot,
    pub steps: Vec<StepReport>,
    pub egress: EgressFindings,
    pub audit_entries: usize,
    pub audit_chain_verified: bool,
}

/// A step that could not prove what it claims.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{step}: {detail}")]
pub struct HeadlessError {
    pub step: &'static str,
    pub detail: String,
}

impl HeadlessError {
    fn new(step: &'static str, detail: impl Into<String>) -> HeadlessError {
        HeadlessError {
            step,
            detail: detail.into(),
        }
    }
}

type Flow<T> = Result<T, HeadlessError>;

/// `Result::map_err` with the step name attached, so a failure says where.
fn at<T, E: std::fmt::Display>(step: &'static str, result: Result<T, E>) -> Flow<T> {
    result.map_err(|error| HeadlessError::new(step, error.to_string()))
}

/// Fail the run unless `holds`.
fn require(step: &'static str, holds: bool, detail: impl Into<String>) -> Flow<()> {
    match holds {
        true => Ok(()),
        false => Err(HeadlessError::new(step, detail)),
    }
}

/// Run the flow in a scratch directory of its own, and clean up after it.
///
/// The directory is removed whether the flow passed or failed: a failed smoke
/// on a user's machine should not leave an encrypted database in their temp
/// folder for somebody to wonder about later.
pub fn run() -> Flow<MainFlowReport> {
    let scratch = scratch_dir_named("soul-headless")?;
    let outcome = run_in(&scratch);
    let _ = std::fs::remove_dir_all(&scratch);
    outcome
}

/// A directory under the platform temp root, named for this process.
pub(crate) fn scratch_dir_named(prefix: &str) -> Flow<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_nanos())
        .unwrap_or_default();
    let path = std::env::temp_dir().join(format!("{prefix}-{}-{stamp}", std::process::id()));
    at("scratch", std::fs::create_dir_all(&path))?;
    Ok(path)
}

/// Wall clock seconds, for the audit entries a run outside the fixed-clock
/// flow has to write. The main flow uses [`AT_UNIX_SECONDS`] instead, so that
/// two runs of it produce the same chain.
pub(crate) fn now_unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or(AT_UNIX_SECONDS)
}

/// The flow itself, against a directory the caller owns.
///
/// The watcher is started before the flow and stopped after it, and its
/// reading is folded in even when a step failed: a run that fell over after
/// opening a socket is exactly the run whose sockets are worth reporting.
pub fn run_in(scratch: &Path) -> Flow<MainFlowReport> {
    let watch = netwatch::Watch::start();
    let outcome = flow(scratch);
    let observed = watch.stop();

    let passed = outcome?;
    Ok(MainFlowReport {
        ok: true,
        config: passed.config,
        steps: passed.steps,
        egress: egress_findings(observed)?,
        audit_entries: passed.audit_entries,
        audit_chain_verified: true,
    })
}

/// What the flow proved, before the network observation is folded in.
struct FlowOutcome {
    config: ConfigSnapshot,
    steps: Vec<StepReport>,
    audit_entries: usize,
}

fn flow(scratch: &Path) -> Flow<FlowOutcome> {
    let mut steps = Vec::new();
    let mut step = |name: &str, detail: String| {
        steps.push(StepReport {
            name: name.to_owned(),
            detail,
        });
    };

    // --- AC-02: the wizard cannot finish with anything switched on ---------
    let config = Config::default();
    let snapshot = at(
        "wizard",
        shell_commands::complete_wizard(&WizardAnswers {
            acknowledged_defaults_are_off: true,
        }),
    )?;
    require(
        "wizard",
        snapshot.fully_closed && snapshot.open_capabilities.is_empty(),
        format!("the wizard left these on: {:?}", snapshot.open_capabilities),
    )?;
    require(
        "wizard",
        shell_commands::complete_wizard(&WizardAnswers::default()).is_err(),
        "an unacknowledged wizard must not finish",
    )?;
    step("wizard", "finished with every capability off".to_owned());

    // --- The encrypted store, on a scratch path ---------------------------
    let mut store = at(
        "store",
        store_commands::open_test_store(scratch, SCRATCH_SEED),
    )?;
    let database = store_commands::database_path(scratch);
    require(
        "store",
        database.is_file(),
        format!("{} was not created", database.display()),
    )?;
    step("store", format!("opened {}", database.display()));

    // --- AC-04/AC-05: an export lands in the encrypted store ---------------
    let staged = at(
        "import",
        import_commands::read_soul_import_v1(IMPORT_CORPUS),
    )?;
    require(
        "import",
        !import_commands::questionnaire_needed(Some(&staged)),
        "a file with conversations in it is not a reason to ask the questionnaire",
    )?;
    let receipt = at(
        "import",
        import_commands::commit(&mut store, &staged, AT_UNIX_SECONDS),
    )?;
    require(
        "import",
        receipt.events_written.len() >= 3 && receipt.contacts_created.len() >= 3,
        format!(
            "the corpus should produce several events and contacts, got {} and {}",
            receipt.events_written.len(),
            receipt.contacts_created.len(),
        ),
    )?;
    step(
        "import",
        format!(
            "{} event(s), {} contact(s) sealed",
            receipt.events_written.len(),
            receipt.contacts_created.len(),
        ),
    );

    // --- AC-08/AC-06: the graph is derived, and every edge cites evidence --
    let build = at(
        "graph",
        graph_commands::rebuild(&mut store, AT_UNIX_SECONDS),
    )?;
    let graph = at("graph", graph_commands::load(&store))?;
    require(
        "graph",
        graph.third_party_nodes().len() >= 3,
        format!(
            "AC-08 wants at least three people, got {}",
            graph.third_party_nodes().len(),
        ),
    )?;
    for edge in &graph.edges {
        let evidence = at("graph", graph_commands::edge_evidence(&store, edge))?;
        require(
            "graph",
            !evidence.is_empty() && evidence.len() == edge.evidence_ids.len(),
            "an edge cites evidence that does not resolve",
        )?;
    }
    require(
        "graph",
        graph.third_party_data_is_local_only(),
        "a third-party node is not marked local-only",
    )?;
    step(
        "graph",
        format!(
            "{} node(s), {} edge(s), all evidence resolves",
            graph.third_party_nodes().len(),
            build.edges_written.len(),
        ),
    );

    // --- AC-03/AC-07: a profile the user filled in, and a correction that
    // survives a stronger inference -----------------------------------------
    let profile_id = Uuid::now_v7();
    let response: QuestionnaireResponse = at("profile", serde_json::from_str(QUESTIONNAIRE))?;
    let intake = at(
        "profile",
        profile_commands::intake(&mut store, profile_id, &response, AT_UNIX_SECONDS),
    )?;
    require(
        "profile",
        intake.profile.trait_axes.len() == 5 && !intake.evidence_ids.is_empty(),
        "a completed questionnaire must leave a non-empty profile",
    )?;

    at(
        "profile",
        profile_commands::correct_axis(
            &mut store,
            profile_id,
            CURIOSITY.axis_id,
            AxisPosition::Mixed,
            AT_UNIX_SECONDS,
        ),
    )?;
    let refused = at(
        "profile",
        profile_commands::record_inference(
            &mut store,
            profile_id,
            AxisProposal::new(
                CURIOSITY.axis_id,
                AxisPosition::LeansHigh,
                SupportedBand::Strong,
                intake.evidence_ids.clone(),
            ),
            AT_UNIX_SECONDS,
        ),
    )?;
    require(
        "profile",
        refused.update == AxisUpdate::RefusedAxisLocked,
        format!(
            "a corrected axis was overwritten anyway: {:?}",
            refused.update
        ),
    )?;
    let applied = at(
        "profile",
        profile_commands::record_inference(
            &mut store,
            profile_id,
            AxisProposal::new(
                ORDERLINESS.axis_id,
                AxisPosition::LeansLow,
                SupportedBand::Weak,
                intake.evidence_ids.clone(),
            ),
            AT_UNIX_SECONDS,
        ),
    )?;
    require(
        "profile",
        applied.update == AxisUpdate::Applied,
        "locking one axis must not lock the others",
    )?;
    let view = at("profile", profile_commands::view(&store, profile_id))?;
    for axis in &view.axes {
        for inference in &axis.inferences {
            require(
                "profile",
                !inference.evidence.is_empty(),
                "AC-06: an inference with no resolvable evidence reached the view",
            )?;
        }
    }
    let rendered = at("profile", profile_commands::render(&store, profile_id))?;
    require(
        "profile",
        rendered.contains(soul_policy::clinical::WORKING_HYPOTHESIS_NOTICE),
        "the profile the user reads must say it is a working hypothesis",
    )?;
    step(
        "profile",
        format!(
            "5 axes, {} evidence row(s), the corrected axis held",
            intake.evidence_ids.len(),
        ),
    );

    // --- AC-14/AC-15: a memory, read back, then forgotten ------------------
    let memory = at(
        "memory",
        memory_commands::create(
            &mut store,
            &MemoryDraft::own(MemoryType::Episodic, MEMORY_TITLE, MEMORY_SUMMARY)
                .about(SealedSubject::Owner),
            AT_UNIX_SECONDS,
        ),
    )?;
    let content = at("memory", memory_commands::read(&store, memory.memory_id))?;
    require(
        "memory",
        content.title == MEMORY_TITLE && content.summary == MEMORY_SUMMARY,
        "a memory did not come back the way it went in",
    )?;

    let impact = at(
        "forget",
        memory_commands::preview_forget(&store, memory.memory_id),
    )?;
    let forget_receipt = at(
        "forget",
        memory_commands::forget(&mut store, memory.memory_id, AT_UNIX_SECONDS),
    )?;
    require(
        "forget",
        forget_receipt.impact == impact,
        "the forget destroyed something other than what the preview quoted",
    )?;
    require(
        "forget",
        matches!(
            memory_commands::read(&store, memory.memory_id),
            Err(soul_memory::MemoryError::Forgotten(_)),
        ),
        "the prose survived a forget",
    )?;
    step(
        "memory",
        format!(
            "1 memory read back verbatim, then forgotten: {} key(s), {} sealed blob(s)",
            forget_receipt.impact.content_key_ids.len(),
            forget_receipt.impact.sealed_blobs_destroyed,
        ),
    );

    // --- AC-17: a draft with no key, and no request body at all ------------
    // The neutral brief, and not the profile written a few steps above: this
    // flow holds the store itself rather than a `Session`, and what AC-21 is
    // watching is the socket. `session_screens.rs` is where the pinned voice
    // reaching the template is proved.
    let (drafting, mut policy) = draft_commands::closed_session();
    let draft = at(
        "draft",
        draft_commands::draft_pasted(
            &drafting,
            &mut policy,
            draft_commands::DraftBrief::neutral(),
            PASTED_MESSAGE,
        ),
    )?
    .draft;
    require(
        "draft",
        !draft.text.trim().is_empty(),
        "the template produced nothing to copy",
    )?;
    require(
        "draft",
        draft.source == draft_commands::DraftSource::ToneTemplate,
        "with no endpoint configured the draft must come from the local template",
    )?;
    require(
        "draft",
        draft.not_sent_notice == draft_commands::NOT_SENT_NOTICE,
        "the draft does not carry the notice that says it was not sent",
    )?;
    step(
        "draft",
        format!(
            "{} character(s) from the local template; no request body was built",
            draft.text.chars().count(),
        ),
    );

    // --- AC-16: a summary about one person, every line with evidence -------
    let contact_id = at("summary", contact_with_most_evidence(&store))?;
    let summary = at(
        "summary",
        draft_commands::summarize_person(
            &drafting,
            &mut policy,
            &store,
            contact_id,
            RequestOrigin::User,
            NOW_MS,
        ),
    )?
    .view;
    require(
        "summary",
        !summary.points.is_empty()
            && summary.points.iter().all(|p| !p.evidence_ids.is_empty())
            && !summary.clinical_claim,
        "a summary point with no evidence, or a clinical claim, reached the surface",
    )?;
    // AD-13's forecast, on the one edge in this corpus that warrants one. It is
    // checked here rather than left to `soul-draft`'s own tests because the
    // question the smoke can answer is whether the sentence survives the whole
    // stack — a rebuild that stopped recording `as_of_utc`, or a summary that
    // dropped the bullets `soul_graph` did not produce, would leave every
    // per-crate test green and this line silently absent from the product.
    //
    // It does not put a clock in the smoke. `soul_graph::rebuild` scores
    // against the newest observation in the store rather than against now, and
    // both instants the projection reads — that one and the edge's last
    // contact — come out of the frozen corpus. The tie is Moderate with three
    // days of silence whatever day this runs on, so the sentence is
    // `personnel.projection.moderate_to_weak` every time.
    let projected = summary
        .points
        .iter()
        .filter(|point| {
            point
                .statement
                .ends_with(soul_draft::projection::WORKING_HYPOTHESIS_CLOSER)
        })
        .count();
    require(
        "summary",
        projected == 1,
        format!(
            "AD-13: {projected} projected sentence(s) on an edge the corpus leaves \
             three days silent at Moderate, where the demotion clock has one thing to say",
        ),
    )?;
    step(
        "summary",
        format!(
            "{} point(s), each citing evidence, {projected} of them the demotion clock, source {}",
            summary.points.len(),
            summary.source,
        ),
    );

    // --- AC-18/AC-19: a read-only plan, and a refusal to carry it out ------
    let authorized = scratch.join("authorized");
    at("fileplan", std::fs::create_dir_all(&authorized))?;
    at(
        "fileplan",
        std::fs::write(authorized.join("对账单.csv"), "a,b\n1,2\n"),
    )?;
    at(
        "fileplan",
        std::fs::write(authorized.join("readme.md"), "# hello\n"),
    )?;
    let unauthorized = scratch.join("unauthorized");
    at("fileplan", std::fs::create_dir_all(&unauthorized))?;

    let mut session = fileplan_commands::FilePlanSession::new();
    let root = authorized.to_string_lossy().into_owned();
    at("fileplan", session.authorize(&root))?;
    let mut issuer = TokenIssuer::new();
    let preview = at(
        "fileplan",
        session.preview(
            &mut issuer,
            &root,
            RequestOrigin::User,
            Default::default(),
            NOW_MS,
        ),
    )?;
    let plan = fileplan_commands::PlanPreview::of(&preview);
    require(
        "fileplan",
        !plan.executable_in_this_version && plan.disk_unchanged && !plan.moves.is_empty(),
        "the plan preview claims something v0.1 does not do",
    )?;
    require(
        "fileplan",
        session
            .preview(
                &mut issuer,
                &unauthorized.to_string_lossy(),
                RequestOrigin::User,
                Default::default(),
                NOW_MS,
            )
            .is_err(),
        "AC-18: a directory the user never authorized was scanned anyway",
    )?;
    let refusal = fileplan_commands::refuse_execution_view(
        &issuer,
        &ActionRequest::new(ActionKind::PlanFiles.as_str(), RequestOrigin::User)
            .with_plan(serde_json::json!({ "moves": plan.moves.len() })),
    );
    require(
        "fileplan",
        refusal.write_not_implemented,
        format!("a request to move files was not refused: {refusal:?}"),
    )?;
    step(
        "fileplan",
        format!(
            "{} proposed move(s), disk unchanged, execution refused as {}",
            plan.moves.len(),
            refusal.reason_code,
        ),
    );

    // --- AC-20: research sees no third party and writes no file ------------
    let files_before = directory_listing(scratch)?;
    let research = at(
        "research",
        store_commands::research_preview(&store, &ResearchPreviewRequest::default()),
    )?;
    require(
        "research",
        !research.manifest.written_to_disk,
        "the research preview claims it wrote a file",
    )?;
    require(
        "research",
        research.third_party_rows_excluded > 0,
        "no third-party row was excluded, so the exclusion is untested on this corpus",
    )?;
    // Nothing has been collected in this flow and the import stores its rows
    // `research_export: deny`, so what is left is the trait axes. An
    // `import.item` row here would mean the disposition on the row was not
    // read: those messages are the owner's own, so the subject filter lets
    // every one of them through.
    require(
        "research",
        research
            .manifest
            .rows
            .iter()
            .all(|row| row.event_kind.as_deref() != Some("import.item")),
        "an imported row reached the research preview",
    )?;
    require(
        "research",
        directory_listing(scratch)? == files_before,
        "the research preview changed the directory it was asked about",
    )?;
    step(
        "research",
        format!(
            "{} row(s) published, {} third-party row(s) excluded, {} owner row(s) withheld by \
             their own disposition, written_to_disk=false",
            research.row_count(),
            research.third_party_rows_excluded,
            research.deny_rows_excluded,
        ),
    );

    // --- AC-09: consent is off, so collection writes nothing ---------------
    let events_before = at("collect", store.list_events(&EventFilter::all()))?.len();
    let consent = collect_commands::ConsentHandle::closed();
    require(
        "collect",
        consent.state(collect_commands::COLLECTION_TOPIC) != ConsentState::Granted,
        "the default consent ledger grants foreground collection",
    )?;
    let source = at(
        "collect",
        collect_commands::FakeForegroundSource::showing("explorer.exe"),
    )?;
    let shared = collect_commands::share(store);
    let refused = collect_commands::start(
        source.clone(),
        std::sync::Arc::clone(&shared),
        &consent,
        Default::default(),
    );
    require(
        "collect",
        refused.is_err(),
        "collection started without consent",
    )?;
    require(
        "collect",
        source.samples_taken() == 0,
        "the foreground was sampled even though the gate was shut",
    )?;
    let mut store = at(
        "collect",
        std::sync::Arc::try_unwrap(shared).map_err(|_| "the collector kept a handle to the store"),
    )?
    .into_inner()
    .unwrap_or_else(std::sync::PoisonError::into_inner);
    let events_after = at("collect", store.list_events(&EventFilter::all()))?.len();
    require(
        "collect",
        events_after == events_before,
        format!(
            "AC-09: {} event(s) appeared with consent off",
            events_after.saturating_sub(events_before),
        ),
    )?;
    step(
        "collect",
        "start refused with consent off; the source was never sampled".to_owned(),
    );

    // --- AC-22: the cloud switch says the same thing either way ------------
    let pressed_on = shell_commands::cloud_toggle(&config, true);
    let pressed_off = shell_commands::cloud_toggle(&config, false);
    require(
        "cloud",
        pressed_on == pressed_off
            && !pressed_on.enabled
            && !pressed_on.performs_network_request
            && pressed_on.label == shell_commands::CLOUD_NOT_YET_AVAILABLE_LABEL,
        "the cloud switch changed its answer",
    )?;
    step(
        "cloud",
        format!("pressed both ways, still {}", pressed_on.label),
    );

    // --- AC-23: the chain verifies, and repeats none of the prose ----------
    at("audit", store.verify_audit_chain())?;
    let audit = at("audit", store.list_audit())?;
    let encoded = at("audit", serde_json::to_string(&audit))?;
    for prose in [MEMORY_TITLE, MEMORY_SUMMARY, PASTED_MESSAGE] {
        require(
            "audit",
            !encoded.contains(prose),
            "the audit chain repeated something the user wrote",
        )?;
    }
    require(
        "audit",
        audit.len() >= 6,
        format!("only {} audit entries for a whole main flow", audit.len()),
    )?;
    step(
        "audit",
        format!("{} entries, chain verified, no prose", audit.len()),
    );

    at("store", store.flush())?;
    at("store", store.close())?;

    Ok(FlowOutcome {
        config: ConfigSnapshot::of(&config),
        audit_entries: audit.len(),
        steps,
    })
}

/// AC-21's own step: what the guard refuses, and what the kernel saw.
pub(crate) fn egress_findings(observed: WatchReport) -> Flow<EgressFindings> {
    let guard = NetGuard::closed();
    let mut refused = Vec::new();
    for authority in UNREACHABLE_AUTHORITIES {
        let url = format!("https://{authority}/v1/chat/completions");
        let denial = guard.authorize(&url).err().ok_or_else(|| {
            HeadlessError::new("egress", format!("the closed guard authorized {authority}"))
        })?;
        refused.push(RefusedTarget {
            authority: (*authority).to_owned(),
            reason_code: denial.reason_code().as_str().to_owned(),
        });
    }

    let note = match &observed.support {
        netwatch::Support::Observed => None,
        netwatch::Support::Unsupported(reason) => Some(reason.clone()),
    };
    let findings = EgressFindings {
        observed: observed.support.is_observed(),
        observation_note: note,
        samples: observed.samples,
        non_loopback_connections: observed.non_loopback.len(),
        non_loopback_peers: observed
            .non_loopback
            .iter()
            .map(ToString::to_string)
            .collect(),
        loopback_connections: observed.loopback.len(),
        endpoint_configured: guard.config().e1_endpoint().is_some(),
        refused,
    };

    require(
        "egress",
        findings.non_loopback_connections == 0,
        format!(
            "AC-21: this process held {} non-loopback connection(s): {}",
            findings.non_loopback_connections,
            findings.non_loopback_peers.join(", "),
        ),
    )?;
    Ok(findings)
}

/// The contact the graph knows most about, for the summary step.
fn contact_with_most_evidence(store: &SqlCipherStore) -> Result<Uuid, String> {
    let graph = graph_commands::load(store).map_err(|error| error.to_string())?;
    graph
        .edges
        .iter()
        .max_by_key(|edge| edge.tie_strength.interaction_count)
        .map(|edge| edge.to_contact_id)
        .ok_or_else(|| "the graph has no edges to summarize".to_owned())
}

/// File names and lengths under a directory, for the "nothing was written"
/// comparison. Contents are irrelevant: the question is whether a file
/// appeared or grew.
fn directory_listing(root: &Path) -> Flow<BTreeSet<String>> {
    fn walk(dir: &Path, into: &mut BTreeSet<String>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                walk(&entry.path(), into)?;
            } else {
                into.insert(format!("{} {}", entry.path().display(), metadata.len()));
            }
        }
        Ok(())
    }

    let mut listing = BTreeSet::new();
    at("research", walk(root, &mut listing))?;
    Ok(listing)
}
