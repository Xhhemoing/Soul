//! AC-16: every point in a people summary cites evidence, and none of it
//! reads like a diagnosis.
//!
//! The graph here is derived rather than hand-assembled — contacts and
//! interaction evidence go into a store, `soul_graph::rebuild` produces the
//! edge, and the summary is taken off what came back. That matters because
//! the claim under test is "the summary rests on evidence that resolves", and
//! a `TieEdge` literal with three invented UUIDs in it would satisfy every
//! assertion below while resting on nothing.
//!
//! The negative cases are the useful half: an empty evidence list, a cited
//! row that does not resolve, and a rephrasing that comes back with a
//! forbidden word in it. Each one has to be refused rather than tidied up.

use uuid::Uuid;

use soul_draft::analysis::{self, PersonSummary, SummaryPoint, SummarySource};
use soul_draft::draft::ReplyGenerator;
use soul_draft::error::{DraftError, GenerationRefused};
use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{Direction, Venue};
use soul_policy::clinical::{assert_non_clinical, WORKING_HYPOTHESIS_NOTICE};
use soul_policy::redactor::{KnownIdentifiers, RedactedBody, Redactor};
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::evidence::SoulEvidence;
use soul_schema::memory::ForgetState;
use soul_store_api::{FakeStore, GraphStore, ProfileStore};

/// A UUIDv7-shaped literal, so a failure names the same person every run.
fn id(tail: &str) -> Uuid {
    format!("0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4{tail:0>3}")
        .parse()
        .expect("hand-written UUIDv7 literal")
}

fn owner() -> Uuid {
    id("001")
}

fn peer() -> Uuid {
    id("002")
}

fn contact(contact_id: Uuid, class: ContactClass) -> SoulContact {
    SoulContact {
        schema_version: SchemaVersion,
        contact_id,
        contact_class: class,
        display_label_ref: None,
        identifiers: None,
        forget_state: ForgetState::Active,
    }
}

/// One observation, written the way an importer writes it.
fn observe(
    store: &mut FakeStore,
    evidence_id: Uuid,
    conversation: &str,
    direction: Direction,
    at: &str,
    venue: Venue,
) {
    let observation = InteractionRef::new(
        id("900"),
        owner(),
        peer(),
        conversation_ref("test", conversation),
        direction,
        Timestamp::new(at),
        venue,
    );
    let subject = match direction {
        Direction::Outgoing => Subject::Mixed,
        Direction::Incoming => Subject::ThirdParty,
    };
    store
        .put_evidence(interaction_evidence(evidence_id, subject, &observation))
        .expect("the evidence row is written");
}

/// Six exchanges over three days, both directions, one to one.
fn store_with_one_partner() -> FakeStore {
    let mut store = FakeStore::new();
    store
        .put_contact(contact(owner(), ContactClass::Owner))
        .expect("the owner");
    store
        .put_contact(contact(peer(), ContactClass::ThirdParty))
        .expect("the other person");

    let days = ["2026-03-02", "2026-03-05", "2026-03-09"];
    let mut next = 100u32;
    for (index, day) in days.iter().enumerate() {
        for direction in [Direction::Outgoing, Direction::Incoming] {
            next += 1;
            observe(
                &mut store,
                id(&next.to_string()),
                "chat-1",
                direction,
                &format!("{day}T0{index}:15:00Z"),
                Venue::Direct,
            );
        }
    }
    soul_graph::rebuild(&mut store).expect("the graph derives");
    store
}

/// The summary the store's own evidence supports.
fn summarize(store: &FakeStore) -> (PersonSummary, Vec<SoulEvidence>) {
    let graph = soul_graph::load(store).expect("the graph loads");
    let edge = graph
        .edges_for(peer())
        .first()
        .copied()
        .expect("one edge")
        .clone();
    let resolved = soul_graph::resolve_evidence(store, &edge).expect("the evidence resolves");
    let summary = analysis::summarize_person(&graph, peer(), &resolved).expect("a summary");
    (summary, resolved)
}

/// A generator that answers with whatever it was told to, without a socket.
///
/// Used for the rephrasing path only: what reaches the wire is covered in
/// `tests/wire.rs`, and what is at stake here is whether a returned narrative
/// is allowed to weaken the points.
#[derive(Debug)]
struct Canned {
    answer: String,
    last_body: Option<String>,
}

impl Canned {
    fn saying(text: &str) -> Canned {
        Canned {
            answer: serde_json::json!({
                "choices": [{ "message": { "role": "assistant", "content": text } }]
            })
            .to_string(),
            last_body: None,
        }
    }

    fn raw(answer: &str) -> Canned {
        Canned {
            answer: answer.to_owned(),
            last_body: None,
        }
    }
}

impl ReplyGenerator for Canned {
    fn generate(&mut self, body: RedactedBody) -> Result<String, GenerationRefused> {
        self.last_body = Some(body.into_string());
        Ok(self.answer.clone())
    }
}

// ---------------------------------------------------------------- AC-16 ---

#[test]
fn every_point_cites_evidence_that_resolves() {
    let store = store_with_one_partner();
    let (summary, resolved) = summarize(&store);

    assert!(!summary.points.is_empty(), "a summary with nothing in it");
    let known: Vec<Uuid> = resolved.iter().map(|row| row.evidence_id).collect();

    for point in &summary.points {
        assert!(
            !point.evidence_ids().is_empty(),
            "`{}` cites nothing",
            point.statement(),
        );
        for cited in point.evidence_ids() {
            assert!(
                known.contains(cited),
                "`{}` cites {cited}, which is not a row that exists",
                point.statement(),
            );
            // Not just "an id in the list" — a row the store hands back.
            store.get_evidence(*cited).expect("the row resolves");
        }
    }
    assert_eq!(summary.source, SummarySource::Counts);
    assert_eq!(summary.notice, WORKING_HYPOTHESIS_NOTICE);
}

#[test]
fn the_summary_says_nothing_a_medical_product_would_say() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);

    for point in &summary.points {
        assert_non_clinical(point.statement()).expect("a point");
    }
    let rendered = analysis::render(&summary).expect("the summary renders");
    assert_non_clinical(&rendered).expect("the whole rendering");
    assert!(
        rendered.contains(WORKING_HYPOTHESIS_NOTICE),
        "the reader is told what this is: {rendered}",
    );
    assert!(
        rendered.contains("依据"),
        "each line names how many rows are behind it: {rendered}",
    );
}

#[test]
fn a_point_with_no_evidence_is_not_a_value_that_can_exist() {
    let refused = SummaryPoint::new("你们最近联系得比以前多", SupportedBand::Weak, Vec::new())
        .expect_err("an unsupported point");
    assert_eq!(refused, DraftError::NoEvidence);
}

#[test]
fn a_point_that_reads_like_a_diagnosis_fails_to_build() {
    let refused = SummaryPoint::new(
        "对方在群里的发言看起来有社交焦虑症",
        SupportedBand::Weak,
        vec![id("101")],
    )
    .expect_err("the vocabulary check runs at construction, not at render time");
    assert!(matches!(refused, DraftError::Clinical(_)));
}

#[test]
fn an_edge_whose_evidence_does_not_resolve_produces_no_summary() {
    let store = store_with_one_partner();
    let graph = soul_graph::load(&store).expect("the graph loads");
    let edge = graph.edges_for(peer())[0].clone();
    let mut resolved = soul_graph::resolve_evidence(&store, &edge).expect("resolved");

    // The caller hands over one row fewer than the edge cites. A summary that
    // accepted this would claim support it cannot show.
    let dropped = resolved.pop().expect("more than one row");
    let refused = analysis::summarize_person(&graph, peer(), &resolved)
        .expect_err("a short evidence list is not a smaller summary");
    assert_eq!(
        refused,
        DraftError::UnresolvedEvidence {
            evidence_id: dropped.evidence_id,
        },
    );
}

#[test]
fn a_person_the_graph_has_never_seen_gets_no_summary() {
    let store = store_with_one_partner();
    let graph = soul_graph::load(&store).expect("the graph loads");
    let stranger = id("777");
    assert_eq!(
        analysis::summarize_person(&graph, stranger, &[]).expect_err("nothing to summarize"),
        DraftError::NoSuchTie {
            contact_id: stranger,
        },
    );
}

/// The numbers the user reads are the numbers the graph filed the edge on, in
/// the frozen renderer's words.
///
/// The wording is `soul_algo_trait::a2_render`'s and is asserted as fragments
/// rather than whole sentences: what this test is for is that the counts
/// survive the trip through the adapter, not that a frozen template still says
/// what its own tests say it says.
#[test]
fn the_counts_in_the_summary_are_the_counts_in_the_graph() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let graph = soul_graph::load(&store).expect("the graph loads");
    let strength = &graph.edges_for(peer())[0].tie_strength;

    assert_eq!(strength.interaction_count, 6);
    assert_eq!(strength.outgoing_count, 3);
    assert_eq!(strength.incoming_count, 3);
    assert_eq!(strength.active_day_count, 3);

    let rendered = analysis::render(&summary).expect("renders");
    assert!(rendered.contains("有记录的往来 6 次"), "{rendered}");
    assert!(rendered.contains("3 个不同的日子"), "{rendered}");
    assert!(
        rendered.contains("往来是双向的"),
        "three each is not a one-sided tie: {rendered}",
    );
    assert!(
        rendered.contains("你发出过 3 次") && rendered.contains("对方发来过 3 次"),
        "the direction sentence carries both counts: {rendered}",
    );
}

/// The venue split is the one the rebuild persisted, and it appears because
/// the edge carries an `as_of` rather than because a count happened to be
/// non-zero.
///
/// Six one-to-one exchanges and no group ones: the group half is a measured
/// zero, and a measured zero is a sentence. Nothing here walks the evidence to
/// work the split out — that is the second opinion D33 rules out — so the two
/// numbers below are the persisted tallies or the test fails.
#[test]
fn the_venue_split_is_the_one_the_graph_wrote_down() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let graph = soul_graph::load(&store).expect("the graph loads");
    let strength = &graph.edges_for(peer())[0].tie_strength;

    assert!(strength.as_of_utc.is_some(), "a rebuilt edge carries as_of");
    assert_eq!(strength.direct_out_count + strength.direct_in_count, 6);
    assert_eq!(strength.group_out_count + strength.group_in_count, 0);

    let rendered = analysis::render(&summary).expect("renders");
    assert!(
        rendered.contains("一对一往来 6 次") && rendered.contains("群里同场 0 次"),
        "the split the band was decided on is what the user is shown: {rendered}",
    );
}

/// An edge written before the per-venue tallies existed says nothing about
/// them, and is not read as "zero one-to-one, zero in a group".
///
/// The distinction is the whole of P1b: a row with no `as_of` never had the
/// fields, so the summary that would describe them is absent rather than
/// wrong. Everything the counts do support is still said.
#[test]
fn a_row_written_before_the_venue_split_existed_claims_no_split() {
    let store = store_with_one_partner();
    let mut graph = soul_graph::load(&store).expect("the graph loads");
    let edge = graph
        .edges
        .iter_mut()
        .find(|edge| edge.touches(peer()))
        .expect("one edge");
    let resolved = soul_graph::resolve_evidence(&store, edge).expect("the evidence resolves");

    // What a pre-rebuild row looks like: counts and instants, no `as_of` and
    // no venue tallies to go with it.
    edge.tie_strength.as_of_utc = None;
    edge.tie_strength.algorithm_id = String::new();
    edge.tie_strength.direct_out_count = 0;
    edge.tie_strength.direct_in_count = 0;
    edge.tie_strength.group_out_count = 0;
    edge.tie_strength.group_in_count = 0;

    let summary = analysis::summarize_person(&graph, peer(), &resolved).expect("a summary");
    let rendered = analysis::render(&summary).expect("renders");

    assert!(
        !rendered.contains("一对一往来") && !rendered.contains("群里同场"),
        "a field that was never written is not a zero: {rendered}",
    );
    assert!(rendered.contains("有记录的往来 6 次"), "{rendered}");
    assert!(rendered.contains("往来是双向的"), "{rendered}");
}

#[test]
fn the_summary_names_nobody() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let rendered = analysis::render(&summary).expect("renders");

    assert!(
        rendered.contains("这个人"),
        "the subject is referred to, not named: {rendered}",
    );
    // A `PersonNode` has no field holding a name, so there is nothing here for
    // a summary to have copied. This checks the rendering did not go looking.
    let graph = soul_graph::load(&store).expect("loads");
    let node = graph.node(peer()).expect("the node");
    assert!(node.label_ref.is_none());
    assert!(!rendered.contains(&peer().to_string()));
}

// ----------------------------------------------- AC-16, the rephrasing ---

#[test]
fn a_rephrasing_changes_the_wording_and_not_the_evidence() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let mut generator = Canned::saying("你们最近往来比较稳定，多数是一对一说话。");

    let phrased = analysis::phrase_with(
        &summary,
        &Redactor::new(KnownIdentifiers::new()),
        &mut generator,
    )
    .expect("the rephrasing is read");

    assert_eq!(phrased.source, SummarySource::UserEndpoint);
    assert_eq!(
        phrased.points, summary.points,
        "the points and their evidence are not up for negotiation",
    );
    assert_eq!(phrased.evidence_ids(), summary.evidence_ids());
    assert_eq!(
        phrased.narrative.as_deref(),
        Some("你们最近往来比较稳定，多数是一对一说话。"),
    );
}

#[test]
fn a_rephrasing_that_reads_like_a_diagnosis_is_dropped_and_the_points_stand() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let mut generator = Canned::saying("从往来频率看，对方有明显的焦虑症倾向。");

    let phrased = analysis::phrase_with(
        &summary,
        &Redactor::new(KnownIdentifiers::new()),
        &mut generator,
    )
    .expect("a bad narrative is not a failed summary");

    assert_eq!(phrased.narrative, None);
    assert_eq!(
        phrased.source,
        SummarySource::Counts,
        "the summary says where it actually came from",
    );
    assert_eq!(phrased.points, summary.points);
    assert_non_clinical(&analysis::render(&phrased).expect("renders")).expect("still clean");
}

#[test]
fn an_endpoint_that_answers_with_nonsense_leaves_the_summary_intact() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let mut generator = Canned::raw("<html>502 bad gateway</html>");

    let phrased = analysis::phrase_with(
        &summary,
        &Redactor::new(KnownIdentifiers::new()),
        &mut generator,
    )
    .expect("an unreadable answer is not a failed summary");
    assert_eq!(phrased.narrative, None);
    assert_eq!(phrased.points, summary.points);
}

#[test]
fn what_goes_out_for_rephrasing_is_soul_s_own_counts() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let mut generator = Canned::saying("好的。");

    analysis::phrase_with(
        &summary,
        &Redactor::new(KnownIdentifiers::new()),
        &mut generator,
    )
    .expect("phrased");
    let sent = generator.last_body.expect("a body was built");

    for point in &summary.points {
        assert!(sent.contains(point.statement()), "the statements travel");
    }
    assert!(
        !sent.contains(&peer().to_string()),
        "no contact id goes out"
    );
    assert_non_clinical(&sent).expect("nothing forbidden leaves either");
}

// --------------------------- AC-16, what a rephrasing is not allowed to be ---

/// The endpoint answers with a claim about the person that no count supports.
///
/// The repro from the R3 probe. Before this, the sentence was shown under
/// 这一份是你自己的端点根据本机统计改写的 — a provenance label the sentence had
/// no relationship to. Nothing about durians is in the counts, so nothing about
/// durians survives.
#[test]
fn an_answer_about_something_else_is_dropped_and_the_counts_remain() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let mut generator = Canned::saying("这个人最喜欢榴莲。");

    let phrased = analysis::phrase_with(
        &summary,
        &Redactor::new(KnownIdentifiers::new()),
        &mut generator,
    )
    .expect("free prose is not a failed summary");

    assert_eq!(phrased.narrative, None);
    assert_eq!(
        phrased.source,
        SummarySource::Counts,
        "unconstrained prose was labelled as a rewrite of the counts",
    );
    assert_eq!(phrased.points, summary.points);

    let rendered = analysis::render(&phrased).expect("renders");
    assert!(!rendered.contains("榴莲"), "{rendered}");
    assert!(
        !rendered.contains(analysis::ENDPOINT_LINE_PREFIX),
        "there is no endpoint line, so nothing introduces one: {rendered}",
    );
}

/// A figure the counts do not contain is the one invention that is decided
/// rather than estimated. Everything else in the sentence is the counts'
/// own wording, so this is a test of the numeral check and of nothing else.
#[test]
fn an_answer_that_states_a_count_nobody_computed_is_dropped() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let mut generator = Canned::saying("你和这个人一共有 40 次往来，分布在 3 个自然日里。");

    let phrased = analysis::phrase_with(
        &summary,
        &Redactor::new(KnownIdentifiers::new()),
        &mut generator,
    )
    .expect("an invented figure is not a failed summary");

    assert_eq!(phrased.narrative, None);
    assert_eq!(phrased.source, SummarySource::Counts);
    let rendered = analysis::render(&phrased).expect("renders");
    assert!(!rendered.contains("40"), "{rendered}");
    assert!(rendered.contains("有记录的往来 6 次"), "{rendered}");
}

/// The same, for a model that spells its invention out in Chinese.
#[test]
fn a_chinese_numeral_the_counts_do_not_have_is_a_figure_too() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let mut generator = Canned::saying("你和这个人一共有四十次往来，分布在几个自然日里。");

    let phrased = analysis::phrase_with(
        &summary,
        &Redactor::new(KnownIdentifiers::new()),
        &mut generator,
    )
    .expect("phrased");
    assert_eq!(phrased.narrative, None);
    assert_eq!(phrased.source, SummarySource::Counts);
}

/// AC-16, read as "every line that looks like a claim says what is behind it".
///
/// The points say 依据 N 条记录. The endpoint's line cannot say that, because
/// there is nothing behind it — so it says that instead, on the line, where a
/// reader is looking. This is the assertion that stops a future edit from
/// quietly rendering the narrative as though Soul had derived it.
#[test]
fn the_endpoint_s_line_says_on_the_line_that_it_is_the_endpoint_s() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let mut generator = Canned::saying("你们最近往来比较稳定，多数时候是一对一说话。");

    let phrased = analysis::phrase_with(
        &summary,
        &Redactor::new(KnownIdentifiers::new()),
        &mut generator,
    )
    .expect("phrased");
    assert_eq!(phrased.source, SummarySource::UserEndpoint);

    let rendered = analysis::render(&phrased).expect("renders");
    let line = rendered
        .lines()
        .find(|line| line.contains("往来比较稳定"))
        .expect("the endpoint's sentence is on screen");
    assert!(
        line.starts_with(analysis::ENDPOINT_LINE_PREFIX),
        "the endpoint's sentence is not introduced as one: {line}",
    );
    for honest in ["你自己的端点写的", "没有证据支持", "没有替你核对"] {
        assert!(
            analysis::ENDPOINT_LINE_PREFIX.contains(honest),
            "the prefix stopped saying `{honest}`: {}",
            analysis::ENDPOINT_LINE_PREFIX,
        );
    }
    // The one claim the label must never make.
    assert!(
        !analysis::ENDPOINT_LINE_PREFIX.contains("根据本机统计改写"),
        "the prefix claims the endpoint's sentence was derived from the counts",
    );

    // And every point still carries its own rows, unchanged.
    for point in &phrased.points {
        assert!(!point.evidence_ids().is_empty());
        assert!(rendered.contains(&format!(
            "· {}（依据 {} 条记录）",
            point.statement(),
            point.evidence_ids().len(),
        )));
    }
}

/// Characters that put what follows on a new line of the screen.
///
/// Written out here rather than taken from `soul-draft`, so that a build whose
/// own idea of a line break narrowed fails this test instead of agreeing with
/// it. `str::lines` splits on `\n` alone; a rendering shown in an element that
/// preserves breaks does not.
const LINE_BREAKS: [char; 7] = [
    '\n', '\r', '\u{0b}', '\u{0c}', '\u{85}', '\u{2028}', '\u{2029}',
];

/// A rendering, split the way a screen that preserves line breaks splits it.
fn display_lines(rendered: &str) -> Vec<&str> {
    rendered.split(LINE_BREAKS).collect()
}

/// Every line of a rendering is one of the four kinds that may be there.
///
/// The header, one bullet per point carrying that point's own row count, at
/// most one line introduced by [`analysis::ENDPOINT_LINE_PREFIX`], and the
/// notice. A line that is none of them is a line the user is reading with
/// nothing on it saying where it came from, which is the whole of what AC-16
/// forbids.
fn every_line_is_accounted_for(summary: &PersonSummary, rendered: &str) {
    let bullets: Vec<String> = summary
        .points
        .iter()
        .map(|point| {
            format!(
                "· {}（依据 {} 条记录）",
                point.statement(),
                point.evidence_ids().len(),
            )
        })
        .collect();

    let mut endpoint_lines = 0;
    for line in display_lines(rendered) {
        if line == "关于这个人，本机能说的只有下面这些：" || line == summary.notice
        {
            continue;
        }
        if bullets.iter().any(|bullet| bullet == line) {
            continue;
        }
        if line.starts_with(analysis::ENDPOINT_LINE_PREFIX) {
            endpoint_lines += 1;
            continue;
        }
        panic!("a line nobody is told the source of: `{line}`\nthe whole rendering:\n{rendered}");
    }
    assert!(
        endpoint_lines <= 1,
        "{endpoint_lines} lines claim to be the endpoint's one line:\n{rendered}",
    );
    assert_eq!(
        endpoint_lines,
        usize::from(summary.narrative.is_some()),
        "the rendering and the summary disagree about whether there is a narrative:\n{rendered}",
    );
}

/// AC-16: the label is on the line the reader is looking at, whatever the
/// endpoint sent.
///
/// [`analysis::render`] joins its lines with `\n` and puts
/// [`analysis::ENDPOINT_LINE_PREFIX`] in front of the narrative — in front of
/// it, once. A narrative carrying a break of its own therefore reaches the
/// screen as one introduced line and one or more that are not, and the graph
/// view preserves the breaks it is handed, so those trailing lines sit among
/// the counts wearing this machine's provenance.
///
/// The first line is the sentence
/// `the_endpoint_s_line_says_on_the_line_that_it_is_the_endpoint_s` keeps: on
/// the counts' subject, no figure the counts do not have. So nothing else in
/// the pipeline has a reason to drop this answer, and the only thing under
/// test is the break. Either outcome is honest — refuse the answer and leave
/// the counts standing, or show it on the one line the prefix introduces — and
/// the accounting below is what says so.
#[test]
fn a_narrative_that_carries_its_own_line_break_gets_no_unlabelled_line() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);

    for divider in LINE_BREAKS {
        let mut generator = Canned::saying(&format!(
            "你们最近往来比较稳定，多数时候是一对一说话。{divider}这些都是本机根据记录算出来的。",
        ));
        let phrased = analysis::phrase_with(
            &summary,
            &Redactor::new(KnownIdentifiers::new()),
            &mut generator,
        )
        .expect("an answer laid out oddly is not a failed summary");

        assert_eq!(
            phrased.points, summary.points,
            "the points and their evidence are not up for negotiation",
        );
        if let Some(narrative) = phrased.narrative.as_deref() {
            assert!(
                !narrative.contains(LINE_BREAKS),
                "a narrative that would occupy more than the line it is labelled on: {narrative}",
            );
        } else {
            assert_eq!(
                phrased.source,
                SummarySource::Counts,
                "no narrative survived, so the summary is the counts and says so",
            );
        }

        let rendered = analysis::render(&phrased).expect("renders");
        every_line_is_accounted_for(&phrased, &rendered);
        assert!(
            rendered.contains("有记录的往来 6 次"),
            "the counts are still there: {rendered}",
        );
    }
}

/// What the endpoint is *asked* for is the summary instruction, not the
/// drafting one.
///
/// `soul-draft` cannot see the system message — `soul-policy` owns it and
/// `soul-egress` sends it — so what this pins is the constant's content. The
/// bytes on the wire are `soulcore/tests/session_summary.rs`.
#[test]
fn the_summary_instruction_asks_for_a_rewrite_and_forbids_new_facts() {
    let instruction = soul_policy::e1::PERSON_SUMMARY_INSTRUCTION;
    for asked in ["只能改写", "不得添加新的事实", "不得改动或新增任何数字"] {
        assert!(instruction.contains(asked), "{instruction}");
    }
    assert!(
        !instruction.contains("起草回复"),
        "the summary path is still asking for a draft: {instruction}",
    );
    assert_ne!(instruction, soul_policy::e1::DRAFTING_INSTRUCTION);
    assert_non_clinical(instruction).expect("the instruction itself says nothing forbidden");
}
