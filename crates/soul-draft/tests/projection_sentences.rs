//! AD-13: the demotion clock, read one step forwards, and everything that
//! stops it saying something it cannot support.
//!
//! The projection is the whole of what v0.1 means by prediction (AD-9): no
//! model, no new threshold, no second rule — the two day constants the tie rule
//! already acts on, plus the dates already on the edge, turned into a sentence
//! that names the day this band changes if nothing else happens.
//!
//! Both intervals are **closed**, and the boundaries are where a forecast goes
//! wrong in the way a user would notice: a step that is announced on the day it
//! has already been taken, or a floor still being promised to a tie that hit it
//! months ago. So 179 / 180 and 359 / 360 are checked as pairs, on the pure
//! function where the arithmetic lives and — for the first pair — through a
//! real rebuild, because a boundary that only holds against a hand-built
//! reading is a boundary in the test rather than in the product.

use uuid::Uuid;

use soul_algo_tie::constants::{DEMOTE_ONE_BAND_DAYS, FORCE_WEAK_DAYS};
use soul_algo_tie::{civil_from_epoch_day, epoch_day, SECONDS_PER_DAY};
use soul_draft::analysis;
use soul_draft::projection::{
    self, ClockReading, ProjectedBullet, FORCED_WEAK_KEY, GROUP_MAINTAINED_KEY,
    MODERATE_TO_WEAK_KEY, PROJECTION_STATEMENT_KEYS, STRONG_TO_MODERATE_KEY,
    WORKING_HYPOTHESIS_CLOSER,
};
use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{Direction, Venue};
use soul_policy::clinical::assert_non_clinical;
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::memory::ForgetState;
use soul_store_api::{FakeStore, GraphStore, ProfileStore};

/// 2026-08-24T12:00:00Z: midday on the day the frozen fixtures score against,
/// and the newest row in every store built below. Midday rather than midnight
/// so that a fixture can put two exchanges on one civil date without either of
/// them sliding into the day before.
const AS_OF: i64 = 1_787_529_600 + 12 * 3_600;

// ------------------------------------------------------ the pure function ---

fn rows() -> Vec<Uuid> {
    vec![Uuid::from_u128(1)]
}

fn reading(band: SupportedBand, silent_days: i64) -> ClockReading {
    let last = AS_OF - silent_days * SECONDS_PER_DAY;
    ClockReading {
        band,
        as_of_unix: AS_OF,
        last_contact_unix: last,
        last_direct_contact_unix: Some(last),
        locked_by_user: false,
    }
}

fn keys(bullets: &[ProjectedBullet]) -> Vec<&str> {
    bullets.iter().map(|bullet| bullet.statement_key).collect()
}

fn projected(band: SupportedBand, silent_days: i64) -> Vec<ProjectedBullet> {
    projection::project(&reading(band, silent_days), &rows())
}

#[test]
fn the_day_before_the_demotion_day_the_step_is_still_ahead() {
    assert_eq!(
        keys(&projected(SupportedBand::Strong, DEMOTE_ONE_BAND_DAYS - 1)),
        vec![STRONG_TO_MODERATE_KEY, FORCED_WEAK_KEY],
        "at one day short of the threshold the rule has not acted yet",
    );
    assert_eq!(
        keys(&projected(
            SupportedBand::Moderate,
            DEMOTE_ONE_BAND_DAYS - 1
        )),
        vec![MODERATE_TO_WEAK_KEY],
        "a Moderate tie's next step is the floor, so the floor is not said twice",
    );
}

/// The closed interval, from the other side. On the day itself the rule has
/// already demoted, so the step is history and only the floor is left — and a
/// stored band that has not caught up is not forecast backwards into a date
/// that has been and gone.
#[test]
fn on_the_demotion_day_the_step_is_no_longer_a_forecast() {
    assert_eq!(
        keys(&projected(SupportedBand::Moderate, DEMOTE_ONE_BAND_DAYS)),
        vec![FORCED_WEAK_KEY],
    );
    assert_eq!(
        keys(&projected(SupportedBand::Strong, DEMOTE_ONE_BAND_DAYS)),
        vec![FORCED_WEAK_KEY],
    );
}

#[test]
fn the_day_before_the_floor_the_floor_is_still_ahead() {
    let bullets = projected(SupportedBand::Moderate, FORCE_WEAK_DAYS - 1);
    assert_eq!(keys(&bullets), vec![FORCED_WEAK_KEY]);
    assert!(
        bullets[0].text_zh.contains(&FORCE_WEAK_DAYS.to_string()),
        "{}",
        bullets[0].text_zh,
    );
}

#[test]
fn on_the_floor_day_there_is_nothing_left_to_forecast() {
    for silent in [FORCE_WEAK_DAYS, FORCE_WEAK_DAYS + 1, FORCE_WEAK_DAYS * 2] {
        for band in [
            SupportedBand::Strong,
            SupportedBand::Moderate,
            SupportedBand::Weak,
        ] {
            assert!(
                projected(band, silent).is_empty(),
                "everything the clock had to do is done at {silent} days",
            );
        }
    }
}

#[test]
fn a_weak_tie_has_no_band_below_it_to_project() {
    for silent in [0, DEMOTE_ONE_BAND_DAYS - 1, DEMOTE_ONE_BAND_DAYS] {
        assert!(projected(SupportedBand::Weak, silent).is_empty());
    }
}

/// GC-9a's argument, applied to the clock: on an edge the user has ruled on,
/// the band is their verdict and the machine's clock does not move it.
#[test]
fn a_band_the_user_locked_is_not_forecast() {
    for band in [SupportedBand::Strong, SupportedBand::Moderate] {
        let mut clock = reading(band, 3);
        clock.locked_by_user = true;
        assert!(projection::project(&clock, &rows()).is_empty());
    }
}

#[test]
fn nothing_to_cite_is_nothing_to_say() {
    assert!(projection::project(&reading(SupportedBand::Strong, 3), &[]).is_empty());
}

/// The group sentence has two halves, and the boundary lives on the first one.
#[test]
fn the_group_sentence_starts_on_the_day_the_private_channel_would_have_cost_a_band() {
    let mut clock = reading(SupportedBand::Moderate, 2);

    clock.last_direct_contact_unix = Some(AS_OF - (DEMOTE_ONE_BAND_DAYS - 1) * SECONDS_PER_DAY);
    assert_eq!(
        keys(&projection::project(&clock, &rows())),
        vec![MODERATE_TO_WEAK_KEY],
        "one day short, the private silence has cost nothing yet",
    );

    clock.last_direct_contact_unix = Some(AS_OF - DEMOTE_ONE_BAND_DAYS * SECONDS_PER_DAY);
    let bullets = projection::project(&clock, &rows());
    assert_eq!(
        keys(&bullets),
        vec![MODERATE_TO_WEAK_KEY, GROUP_MAINTAINED_KEY]
    );
    assert!(
        bullets[1]
            .text_zh
            .contains(&DEMOTE_ONE_BAND_DAYS.to_string()),
        "the sentence counts the private silence out loud: {}",
        bullets[1].text_zh,
    );
}

/// The other half: when every venue has gone quiet, nothing is holding the
/// band up and the group sentence would be false.
#[test]
fn a_tie_that_is_quiet_everywhere_is_not_being_held_up_by_a_group() {
    let clock = reading(SupportedBand::Moderate, DEMOTE_ONE_BAND_DAYS);
    assert!(!keys(&projection::project(&clock, &rows())).contains(&GROUP_MAINTAINED_KEY));
}

#[test]
fn a_tie_that_has_never_been_one_to_one_says_nothing_about_one() {
    let mut clock = reading(SupportedBand::Moderate, 2);
    clock.last_direct_contact_unix = None;
    assert_eq!(
        keys(&projection::project(&clock, &rows())),
        vec![MODERATE_TO_WEAK_KEY],
    );
}

// ----------------------------------------------------------- the sentences ---

/// Every sentence the module can produce, for the screening tests below.
fn every_sentence() -> Vec<ProjectedBullet> {
    let mut bullets = Vec::new();
    bullets.extend(projected(SupportedBand::Strong, DEMOTE_ONE_BAND_DAYS - 1));
    bullets.extend(projected(SupportedBand::Moderate, 0));
    let mut held_up = reading(SupportedBand::Moderate, 1);
    held_up.last_direct_contact_unix = Some(AS_OF - FORCE_WEAK_DAYS * SECONDS_PER_DAY);
    bullets.extend(projection::project(&held_up, &rows()));
    bullets
}

#[test]
fn every_projected_sentence_is_screened_and_signed() {
    let bullets = every_sentence();
    let emitted: Vec<&str> = keys(&bullets);
    for key in PROJECTION_STATEMENT_KEYS {
        assert!(
            emitted.contains(&key),
            "`{key}` has no case that produces it"
        );
    }

    for bullet in &bullets {
        let text = &bullet.text_zh;
        assert_non_clinical(text).expect("a projected sentence");
        assert!(
            text.ends_with(WORKING_HYPOTHESIS_CLOSER),
            "COPY_ZH §0.6 asks every explanation to close with it: {text}",
        );
        assert!(
            !text.chars().any(|c| c.is_ascii_alphabetic()),
            "COPY_ZH §0.3 forbids latin letters in copy: {text}",
        );
        assert!(
            !bullet.evidence_ids.is_empty(),
            "a sentence with nothing behind it: {text}",
        );
        for forbidden in ["今天", "现在会", "大概", "可能会降"] {
            assert!(!text.contains(forbidden), "{text}");
        }
    }
}

/// The band words are the three COPY_ZH allows, written into the templates
/// rather than computed beside them, and each sentence names exactly one step.
#[test]
fn each_sentence_names_the_bands_it_is_about_and_no_others() {
    let sentences: Vec<(&str, String)> = every_sentence()
        .into_iter()
        .map(|bullet| (bullet.statement_key, bullet.text_zh))
        .collect();

    let text = |key: &str| {
        sentences
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, text)| text.clone())
            .unwrap_or_else(|| panic!("`{key}` was not produced"))
    };

    let step = text(STRONG_TO_MODERATE_KEY);
    assert!(
        step.contains("「强」") && step.contains("「中等」"),
        "{step}"
    );
    assert!(!step.contains("「弱」"), "one step per sentence: {step}");

    let down = text(MODERATE_TO_WEAK_KEY);
    assert!(
        down.contains("「中等」") && down.contains("「弱」"),
        "{down}"
    );
    assert!(!down.contains("「强」"), "one step per sentence: {down}");

    let floor = text(FORCED_WEAK_KEY);
    assert!(floor.contains("「弱」"), "{floor}");
    assert!(
        !floor.contains("「强」") && !floor.contains("「中等」"),
        "the floor is one band, not a chain: {floor}",
    );
}

/// The two day figures the user reads are the tie rule's own constants, and
/// the closed interval is said in words as well as in arithmetic.
#[test]
fn the_days_in_the_copy_are_the_constants_the_rule_acts_on() {
    let step = &projected(SupportedBand::Strong, 0)[0].text_zh;
    assert!(step.contains(&DEMOTE_ONE_BAND_DAYS.to_string()), "{step}");
    assert!(
        step.contains("从那天起"),
        "the closed interval, said: {step}"
    );

    let floor = &projected(SupportedBand::Moderate, DEMOTE_ONE_BAND_DAYS)[0].text_zh;
    assert!(floor.contains(&FORCE_WEAK_DAYS.to_string()), "{floor}");
    assert!(floor.contains("从那天起"), "{floor}");
}

/// D52's ban, checked on the file this slice added. `day_constants_agree.rs`
/// scans every product source; this names the one that would be tempted.
#[test]
fn the_projection_source_spells_out_neither_day() {
    let source = include_str!("../src/projection.rs");
    for line in source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
    {
        for day in [DEMOTE_ONE_BAND_DAYS, FORCE_WEAK_DAYS] {
            assert!(
                !line.contains(&day.to_string()),
                "the projection spells a day threshold out: {line}",
            );
        }
    }
}

/// Copy reaches `COPY_ZH.md` before it reaches code. The keys are how the two
/// are matched up, so a template added here without a line in the frozen file
/// fails.
#[test]
fn the_frozen_copy_file_holds_every_key_this_module_emits() {
    let copy = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("crates/soul-draft sits two levels below the repository root")
            .join("docs/algorithms/COPY_ZH.md"),
    )
    .expect("the frozen copy file");

    for key in PROJECTION_STATEMENT_KEYS {
        assert!(copy.contains(key), "COPY_ZH has no template for `{key}`");
    }
    assert!(copy.contains(WORKING_HYPOTHESIS_CLOSER));
    assert!(
        copy.contains("从那天起"),
        "the closed interval is a copy decision, not only an implementation one",
    );
}

// ------------------------------------------------- through a real rebuild ---

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

/// Somebody who wrote on the newest day in the store, so `as_of` is fixed by
/// the data rather than by the peer under test.
fn anchor() -> Uuid {
    id("003")
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

/// A Unix instant in the RFC 3339 subset the graph contract writes.
fn rfc3339(unix: i64) -> String {
    let (year, month, day) = civil_from_epoch_day(epoch_day(unix));
    let seconds = unix.rem_euclid(SECONDS_PER_DAY);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        seconds / 3_600,
        (seconds % 3_600) / 60,
        seconds % 60,
    )
}

fn observe(
    store: &mut FakeStore,
    evidence_id: Uuid,
    with: Uuid,
    direction: Direction,
    at: i64,
    venue: Venue,
) {
    let observation = InteractionRef::new(
        id("900"),
        owner(),
        with,
        conversation_ref(
            "test",
            match venue {
                Venue::Direct => "chat-1",
                Venue::Group => "group-1",
            },
        ),
        direction,
        Timestamp::new(&rfc3339(at)),
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

/// Twelve one-to-one exchanges over six days — a Strong record on the counts —
/// ending `quiet_days` before the newest row in the store, plus `group_days`
/// ago a word in a group when one is asked for.
fn store_with_a_quiet_partner(quiet_days: i64, group_days: Option<i64>) -> FakeStore {
    let mut store = FakeStore::new();
    for (id, class) in [
        (owner(), ContactClass::Owner),
        (peer(), ContactClass::ThirdParty),
        (anchor(), ContactClass::ThirdParty),
    ] {
        store.put_contact(contact(id, class)).expect("a contact");
    }

    // The newest one-to-one exchange lands exactly `quiet_days` before `as_of`,
    // so the silence the rebuild counts is the number this fixture was asked
    // for rather than that number minus a rounding.
    let last_exchange = AS_OF - quiet_days * SECONDS_PER_DAY;
    let mut next = 100u32;
    for day in 0..6i64 {
        let close_of_day = last_exchange - (5 - day) * SECONDS_PER_DAY;
        for (slot, direction) in [Direction::Outgoing, Direction::Incoming]
            .into_iter()
            .enumerate()
        {
            next += 1;
            observe(
                &mut store,
                id(&next.to_string()),
                peer(),
                direction,
                close_of_day - (1 - slot as i64) * 600,
                Venue::Direct,
            );
        }
    }

    if let Some(days) = group_days {
        next += 1;
        observe(
            &mut store,
            id(&next.to_string()),
            peer(),
            Direction::Incoming,
            AS_OF - days * SECONDS_PER_DAY,
            Venue::Group,
        );
    }

    // The newest row in the store, and the only thing that fixes `as_of`.
    observe(
        &mut store,
        id("999"),
        anchor(),
        Direction::Incoming,
        AS_OF,
        Venue::Direct,
    );
    soul_graph::rebuild(&mut store).expect("the graph derives");
    store
}

/// The summary the store's own evidence supports, and the edge behind it.
fn summarize(store: &FakeStore) -> (Vec<String>, soul_graph::model::TieEdge) {
    let graph = soul_graph::load(store).expect("the graph loads");
    let edge = graph
        .edges_for(peer())
        .into_iter()
        .max_by_key(|edge| edge.tie_strength.interaction_count)
        .expect("one edge")
        .clone();
    let resolved = soul_graph::resolve_evidence(store, &edge).expect("the evidence resolves");
    let summary = analysis::summarize_person(&graph, peer(), &resolved).expect("a summary");
    let statements = summary
        .points
        .iter()
        .map(|point| point.statement().to_owned())
        .collect();
    (statements, edge)
}

/// The product boundary. One day short of the threshold the rebuild still
/// files the tie as Strong, and the summary names both days ahead of it.
#[test]
fn a_tie_one_day_short_of_the_demotion_day_is_told_which_day_it_is() {
    let store = store_with_a_quiet_partner(DEMOTE_ONE_BAND_DAYS - 1, None);
    let (statements, edge) = summarize(&store);

    assert_eq!(edge.tie_strength.silent_days, DEMOTE_ONE_BAND_DAYS - 1);
    assert_eq!(edge.tie_strength.band, SupportedBand::Strong);

    // 179 days of silence on 2026-08-24 means the 180th is the day after, and
    // the floor is 180 days past that.
    let step = statements
        .iter()
        .find(|line| line.contains("从「强」降到「中等」"))
        .unwrap_or_else(|| panic!("no step sentence in {statements:?}"));
    assert!(step.contains("2026 年 8 月 25 日"), "{step}");
    assert!(
        statements
            .iter()
            .any(|line| line.contains("会算「弱」") && line.contains("2027 年 2 月 21 日")),
        "{statements:?}",
    );
}

/// The same store one day later. The rule has demoted the tie, so the step is
/// not offered again — only the floor is still ahead.
#[test]
fn on_the_demotion_day_the_summary_stops_promising_the_step_it_already_took() {
    let store = store_with_a_quiet_partner(DEMOTE_ONE_BAND_DAYS, None);
    let (statements, edge) = summarize(&store);

    assert_eq!(edge.tie_strength.silent_days, DEMOTE_ONE_BAND_DAYS);
    assert_eq!(
        edge.tie_strength.band,
        SupportedBand::Moderate,
        "the closed interval demotes on the day itself",
    );
    assert!(
        !statements.iter().any(|line| line.contains("降到「中等」")),
        "a step that has been taken is not a forecast: {statements:?}",
    );
    assert!(
        statements.iter().any(|line| line.contains("会算「弱」")),
        "{statements:?}",
    );
}

/// The case the group sentence exists for: no private word in half a year, a
/// band that has not moved, and a group both people are still in.
#[test]
fn a_band_held_up_by_a_group_says_so() {
    let store = store_with_a_quiet_partner(DEMOTE_ONE_BAND_DAYS + 20, Some(10));
    let (statements, edge) = summarize(&store);

    assert_eq!(edge.tie_strength.silent_days, 10);
    assert_eq!(edge.tie_strength.band, SupportedBand::Strong);
    assert!(
        statements
            .iter()
            .any(|line| line.contains("靠群里的往来维持")
                && line.contains(&(DEMOTE_ONE_BAND_DAYS + 20).to_string())),
        "{statements:?}",
    );
}

/// Every projected point rests on the row holding the last exchange, and on
/// nothing else: the date is what the forecast is made of, and citing the
/// whole edge would be padding.
#[test]
fn a_projected_point_cites_the_exchange_its_date_is_counted_from() {
    let store = store_with_a_quiet_partner(30, None);
    let graph = soul_graph::load(&store).expect("the graph loads");
    let edge = graph.edges_for(peer())[0].clone();
    let resolved = soul_graph::resolve_evidence(&store, &edge).expect("the evidence resolves");
    let summary = analysis::summarize_person(&graph, peer(), &resolved).expect("a summary");

    let last = edge.tie_strength.last_contact_utc.as_str();
    let expected: Vec<Uuid> = resolved
        .iter()
        .filter(|row| {
            soul_graph::interaction::interactions_in(row)
                .iter()
                .any(|seen| seen.occurred_at.as_str() == last)
        })
        .map(|row| row.evidence_id)
        .take(1)
        .collect();
    assert_eq!(expected.len(), 1, "the fixture has one newest exchange");

    let projected: Vec<_> = summary
        .points
        .iter()
        .filter(|point| point.statement().ends_with(WORKING_HYPOTHESIS_CLOSER))
        .collect();
    assert!(!projected.is_empty(), "no projection on a live Strong tie");
    for point in projected {
        assert_eq!(point.evidence_ids(), expected, "{}", point.statement());
    }
}

/// The whole rendering, screened once more where the user actually reads it.
#[test]
fn the_rendered_summary_with_a_forecast_in_it_still_says_nothing_forbidden() {
    let store = store_with_a_quiet_partner(DEMOTE_ONE_BAND_DAYS - 1, Some(1));
    let graph = soul_graph::load(&store).expect("the graph loads");
    let edge = graph.edges_for(peer())[0].clone();
    let resolved = soul_graph::resolve_evidence(&store, &edge).expect("the evidence resolves");
    let summary = analysis::summarize_person(&graph, peer(), &resolved).expect("a summary");

    let rendered = analysis::render(&summary).expect("the summary renders");
    assert_non_clinical(&rendered).expect("the whole rendering");
    assert!(rendered.contains(WORKING_HYPOTHESIS_CLOSER), "{rendered}");
    assert!(!rendered.contains("今天"), "COPY_ZH §0.5: {rendered}");
}
