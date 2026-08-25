MODEL_SLUG: claude-fable-5-thinking-xhigh

# G1 — T4D wiring spec for Goal 1 (`Tally::band` → `soul_algo_tie::score`)

Round 1, slot fable-a, branch `cursor/goal1-unblock-a073`. Audience: Round 2 opus, who
implements this **without redesigning**. Binding sources, in order: `docs/algorithms/DECISION.md`
§3 (as_of 纪律, constants) and §6 (adoption obligations), `docs/algorithms/COPY_ZH.md` §0/§1/§4,
the frozen `crates/soul-algo-tie` API. This document is a mechanical plan over those; where it
seems to disagree with DECISION.md, DECISION.md wins.

Hard constraints restated: **do not** change any T4D constant (3/10/3/180/360), **do not** add a
third demotion gate (F04c is a priced limitation, DECISION §4.2 — its only remedy is the §5
rollback chain), **do not** pull SQLCipher/Tauri/HTTP/storage into algo crates, **do not** start
Goal 2.

## 0. State of the branch when this spec was written

- `crates/soul-algo-tie` is frozen and on this branch (merge `b3cb6d4`). Entry point:
  `soul_algo_tie::score(peer_id: u64, &[Interaction], as_of_unix: i64) -> TieScore` (dispatches
  `TieAlgo::DEFAULT = T4D`). `Interaction { peer_id: u64, outgoing: bool, occurred_at_unix: i64,
  venue_direct: bool, conversation_id: u64 }`. `as_of_max(&[Interaction]) -> Option<i64>` exists
  and is the legal way to pick the default as_of.
- Commit `4da2184` (Round 1, concurrent slot) already landed
  `crates/soul-graph/src/t4d_adapt.rs`: `InteractionInterner` (UUID→u64 and
  conversation_ref→u64), a dependency-free RFC 3339 → unix-seconds parser (fractions dropped,
  numeric offsets handled), `AdaptError::InvalidTimestamp`, and unit tests. **Use it; do not
  write a second adapter.** `soul-graph/Cargo.toml` already depends on `soul-algo-tie`.
- `crates/soul-graph/src/build.rs` is still T0: local constants
  `MODERATE_MIN_INTERACTIONS/STRONG_MIN_INTERACTIONS/STRONG_MIN_ACTIVE_DAYS` (lines 40–44), a
  string-timestamp `Tally` with `band()`, no venue split beyond `any_direct`, no recency step,
  no as_of.
- `docs/schemas/relationship.schema.json` declares `tie_strength` as a free `{"type":"object"}`,
  so the split-count fields below are **additive and contract-legal**. `soul-graph::view::read_edge`
  strict-deserializes stored `tie_strength` into `TieStrength` — this is why every new field
  must carry `#[serde(default)]`.

## 1. Function-level steps (all in `crates/soul-graph`)

### 1.1 Delete from `build.rs`

- The three local constants `MODERATE_MIN_INTERACTIONS`, `STRONG_MIN_INTERACTIONS`,
  `STRONG_MIN_ACTIVE_DAYS` (DECISION §6.1: local constants deleted, no second definition
  anywhere in Goal 1). Do **not** re-import them into `build.rs` either — after this change the
  band is opaque to soul-graph; it never compares a count to a threshold again.
- `struct Tally` and its whole impl (`new`, `absorb`, `interaction_count`, `is_reciprocal`,
  `band`, `types`, `strength`).
- `fn utc_date` (active-day counting moves entirely into the scorer's `epoch_day`; the
  two spellings are pinned equal by `soul-algo-tie/tests/goal1_fidelity.rs`).

### 1.2 New per-peer accumulator (private to `build.rs`)

```rust
struct PeerEvidence {
    /// The interned id every row of this peer carries. Scratch, never persisted.
    interned_peer_id: u64,
    /// This peer's adapted rows, in evidence order. Input to the scorer.
    rows: Vec<soul_algo_tie::Interaction>,
    evidence_ids: BTreeSet<Uuid>,
    /// Original stored RFC 3339 strings, min/max lexicographically — the SAME
    /// rule build.rs uses today (writers normalize to Z). Display fields keep
    /// the exact stored string; do NOT round-trip through unix (that would
    /// drop fractional seconds and break exact-string assertions in
    /// tests/ego_graph.rs).
    first_contact: String,
    last_contact: String,
    /// Lexicographic max over Venue::Direct rows only. None = never one-to-one.
    last_direct_contact: Option<String>,
}
```

### 1.3 `rebuild` step order (signature unchanged: `pub fn rebuild<S>(store: &mut S) -> GraphResult<GraphBuild>`)

1. Owner/contact resolution, `AmbiguousOwner`, empty-store early return — **unchanged**.
2. Before the evidence loop create, per rebuild:
   `let mut interner = InteractionInterner::default();`
   `let mut peers: BTreeMap<Uuid, PeerEvidence> = BTreeMap::new();`
   `let mut as_of_unix: Option<i64> = None;`
3. Evidence loop, per observation (replaces the tally loop):
   1. `SelfLoop` / `DanglingContact` checks — unchanged.
   2. `interactions_read += 1;`
   3. Adapt **every** observation, before the known-peer check:
      `let row = interner.adapt(&observation).map_err(|_| GraphError::UnreadableInteraction { evidence_id: evidence.evidence_id })?;`
      A timestamp the adapter cannot read fails the whole rebuild. No silent skip: a row with
      a broken instant is damaged evidence, and today's lexicographic code would have silently
      absorbed garbage. New error variant, prose-free like the others:
      `#[error("interaction evidence {evidence_id} carries a timestamp this build cannot read")] UnreadableInteraction { evidence_id: Uuid }`.
   4. Fold `row.occurred_at_unix` into `as_of_unix` (max) — **also before** the known-peer
      check. See §3 for why unresolved-peer rows still move as_of.
   5. If the peer contact is unknown: `peers_unresolved += 1; continue;` — unchanged.
   6. Otherwise accumulate into `peers.entry(observation.peer_contact_id)`:
      push `row`; insert `evidence.evidence_id`; string-min/max `first_contact`/`last_contact`;
      if `observation.venue == Venue::Direct`, string-max `last_direct_contact`;
      set `interned_peer_id` on first insert (`row.peer_id`).
4. If `as_of_unix` is `None` there were no observations at all: return the build with the
   counters, as today. Otherwise this **single value scores every peer** (§3).
5. Edge/inference loop, per `(peer_uuid, acc)` in `peers` (BTreeMap order — deterministic):
   1. `relationship_id` find-or-mint by `connects(edge, owner_id, peer)` — unchanged.
   2. `let score = soul_algo_tie::score(acc.interned_peer_id, &acc.rows, as_of_unix);`
      Call the free function `score`, not `T4D::score` — `score` is the frozen product entry
      point and the rollback (T4D→T4, DECISION §5.1) then stays a one-line change inside the
      algo crate, not a Goal 1 edit. Per-peer calls over pre-partitioned rows keep the rebuild
      linear (each `Tally::of` only walks that peer's rows); `score_ego_network` was considered
      and rejected because evidence-id and display-string accumulation is per-Uuid anyway and
      would need a reverse u64→Uuid map for no gain.
   3. `let strength = tie_strength_of(&score, &acc);` (§1.4) and
      `let types = types_of(&score);` (§1.5).
   4. `SoulRelationship` and `SoulInference` writes — **unchanged shape**: same
      `TIE_STATEMENT_PREFIX` matching, same `statement_key: format!("graph.tie.{band}")`, same
      `method: Rule`, same falsifier, same `evidence_ids`, and still
      `user_verdict: Unreviewed` (preserving verdicts is G3, a separate P0 — do not attempt it
      here, do not make it harder).
6. Audit block — unchanged.

### 1.4 Score → stored strength mapping (one function, one place)

```rust
fn supported_band(band: soul_algo_tie::Band) -> SupportedBand {
    match band {
        soul_algo_tie::Band::Weak => SupportedBand::Weak,
        soul_algo_tie::Band::Moderate => SupportedBand::Moderate,
        soul_algo_tie::Band::Strong => SupportedBand::Strong,
    }
}

fn tie_strength_of(score: &TieScore, acc: &PeerEvidence) -> TieStrength {
    TieStrength {
        band: supported_band(score.band),
        interaction_count: score.interaction_count,
        outgoing_count: score.outgoing_count,
        incoming_count: score.incoming_count,
        conversation_count: score.conversation_count,
        active_day_count: score.active_day_count,
        first_contact_utc: Timestamp::new(acc.first_contact.clone()),
        last_contact_utc: Timestamp::new(acc.last_contact.clone()),
        // Round 3 split — see §4.
        direct_out_count: score.direct_out_count,
        direct_in_count: score.direct_in_count,
        group_out_count: score.group_out_count,
        group_in_count: score.group_in_count,
        direct_active_day_count: score.direct_active_day_count,
        last_direct_contact_utc: acc.last_direct_contact.clone().map(Timestamp::new),
        silent_days: score.silent_days,
        as_of_utc: Some(Timestamp::new(soul_policy::clock::rfc3339_utc(score.as_of_unix))),
        algorithm_id: score.algorithm_id.to_owned(),
    }
}
```

Notes: combined totals come from the score (they are derived sums of the split, pinned in the
algo crate); display timestamps come from the accumulator's original strings; `as_of_utc` is
formatted with `soul_policy::clock::rfc3339_utc` (soul-graph already depends on soul-policy) —
whole seconds, `Z`, which matches the parse (fractions were dropped on the way in).

### 1.5 Observed types stay any-venue

```rust
fn types_of(score: &TieScore) -> Vec<TieType> {
    vec![
        if score.any_direct() { TieType::Direct } else { TieType::GroupOnly },
        if score.is_reciprocal() { TieType::Reciprocal } else { TieType::OneSided },
    ]
}
```

`types` are observed shapes, not band inputs (model.rs doc). `Reciprocal` deliberately keeps
**any-venue** reciprocity (`score.is_reciprocal()`, not `is_direct_reciprocal()`): A2's P3
occasion sentence and the group-only sentence key off `GroupOnly`/`Direct`, and nothing frozen
asks the observed shapes to adopt the T4D counting unit. The band is where the unit changed.

### 1.6 `model.rs` — `TieStrength` additions

Every new field carries `#[serde(default)]` so a `tie_strength` object written **before** this
change still loads through `view::read_edge` (legacy rows read zeros/None/"" and are replaced
wholesale on the next rebuild; DECISION §6.5 says legacy bands are not normative anyway):

```rust
// appended to TieStrength, all #[serde(default)]
pub direct_out_count: u64,
pub direct_in_count: u64,
pub group_out_count: u64,
pub group_in_count: u64,
/// Distinct UTC days with a one-to-one exchange. Never larger than active_day_count.
pub direct_active_day_count: u64,
/// None = never one-to-one. Never a 1970 sentinel.
pub last_direct_contact_utc: Option<Timestamp>,
/// Whole UTC days between last_contact (ANY venue) and as_of. From the scorer, floored, >= 0.
pub silent_days: i64,
/// The one store-wide instant this rebuild scored against. None only on legacy rows.
pub as_of_utc: Option<Timestamp>,
/// "T4D" from the score. Empty only on legacy rows.
pub algorithm_id: String,
```

Do **not** serialize `soul_algo_tie::TieScore` itself — the algo crate has no serde and must
not grow it (§5). This field-by-field map in soul-graph *is* the persistence boundary.

### 1.7 `soulcore::commands::graph` — audit clock stays out of scoring

`rebuild(store, at_unix_seconds)` keeps its signature. `at_unix_seconds` is the **audit**
clock only. It must never reach the scorer; the scoring as_of is evidence-derived inside
`soul_graph::rebuild` (§3). Pinned by test T12. `TieEdgeView` needs no change to compile; the
split fields become view/A2 material in R3 (additive copies when that lands, COPY_ZH §4 P1b).

### 1.8 Downstream compile impact (complete list)

- `soul-draft/src/analysis.rs` (`points_for`): compiles unchanged — it reads existing
  `TieStrength` fields only. Its venue/recency points re-derive from evidence rows today; COPY_ZH
  §4 P1b/§83 forbids A2 deriving the **split counts** from evidence — the persisted fields from
  §1.6 are what R3 rewires it to. Do not touch `soul-draft` in G1.
- `soulcore/src/commands/graph.rs`: compiles unchanged.
- `soul-import/tests/import_to_graph.rs` and `soul-graph/tests/ego_graph.rs`: must keep passing
  (see §6 T2/T9). Coordinate: a concurrent Round 1 slot (G1+ fan-out, opus-b) is editing
  soul-import; rebase before landing.

## 2. UUID→u64 interning (peer AND conversation_id)

Use the committed `InteractionInterner` (`t4d_adapt.rs`, commit `4da2184`) exactly as is:

- **One interner per rebuild**, created inside `rebuild`, dropped with it. Interned ids are
  scratch coordinates for one scoring pass; they carry no meaning across rebuilds.
- **Peer**: dense sequential `u64` keyed on the **full 128-bit `Uuid`** in a `BTreeMap`.
  Never truncate, never hash, never persist. (Truncation collides two peers and merges their
  evidence; the adapter's `interned_ids_are_dense_unique_and_use_the_whole_uuid` test pins
  UUIDs that agree on either half.)
- **Conversation**: same treatment, keyed on the full `conversation_ref` `Sha256Hex` string
  (the platform id was already hashed upstream in `interaction::conversation_ref`; the interner
  must not rehash or truncate it). Distinct-conversation counts survive because the mapping is
  a bijection on the rebuild's key set — `score.conversation_count` equals the peer's distinct
  `conversation_ref` count.
- Interned ids never appear in `SoulRelationship`, `SoulInference`, audit content, errors, or
  logs. The only u64s that persist are counts.

## 3. One as_of

- **Definition**: `as_of_unix = max(occurred_at_unix)` over **every observation read in this
  rebuild pass** — the whole store, including rows whose peer contact row is gone
  (`peers_unresolved`). This is DECISION §3's default（缺省 = 整个 store 的 `max(occurred_at)`）
  and is deterministic in the evidence alone: forgetting a *contact row* does not shift anyone
  else's `silent_days`; only forgetting the *evidence rows themselves* moves as_of, which is the
  「no evidence, no row」discipline working as intended.
- **One value scores every peer.** Per-peer `max` of that peer's own rows is the frozen
  anti-pattern（休眠关系假 Strong — DECISION §3, `soul-algo-tie/tests/as_of_discipline.rs`）.
- **Never a clock.** Not `soul_policy::clock::now_unix_seconds`, not soulcore's
  `at_unix_seconds` (that is the audit clock), not `SystemTime` anywhere. judgement 一律不读墙钟.
- Empty store / zero observations → no as_of, no scores, empty build (today's behavior).
- Known consequence to leave alone: in a store whose newest evidence is itself old (a
  one-shot archive import), as_of equals that newest row, so the newest ties are not demoted.
  That is the frozen default, not a bug. DECISION §6.3 permits an explicit caller-supplied
  as_of; **do not add the parameter in G1** — no caller needs it yet, and adding an unused knob
  invites a second time source. When a caller materializes, the override threads through
  `soul_graph::rebuild`, never into the algo crate.

## 4. Persisting the split counts

- What persists, per edge, inside the free-form `tie_strength` object: the four split counts
  (`direct_out/direct_in/group_out/group_in`), `direct_active_day_count`,
  `last_direct_contact_utc` (Option), `silent_days`, `as_of_utc`, `algorithm_id` — plus every
  field already there, unchanged. Contract-legal: `relationship.schema.json` constrains
  `tie_strength` only to `{"type":"object"}`.
- Why persist rather than re-derive: DECISION §6.4 — `TieScore` carries band、原始计数、
  一对一/群聊分列、last_contact、沉寂天数与 as_of, and **A2 and the graph UI only consume that
  structure**. COPY_ZH §4 P1b: A2 renders the two split numbers the T-family computed and 「任一数
  缺席则整句不出现」; COPY_ZH §7(83): A2 源码不得包含从证据行重算分列的路径. Persisting on the edge is
  what makes the renderer a pure consumer.
- Invariants the write must satisfy (asserted in T6): `direct_out + direct_in + group_out +
  group_in == interaction_count`; `direct_out + group_out == outgoing_count` (and mirror for
  incoming); `direct_active_day_count <= active_day_count`; `last_direct_contact_utc` is `None`
  iff `direct_out + direct_in == 0`; `algorithm_id == "T4D"`.
- Legacy rows (written before this change) deserialize via the serde defaults, render as
  before, and are overwritten by the next rebuild. No migration step.

## 5. What NOT to edit in the algo crates

`crates/soul-algo-tie` and `crates/soul-algo-trait` take **zero diffs** in G1. Specifically:

1. No serde, no new dependencies of any kind — the crate's `[dependencies]` section stays
   empty. Persistence is soul-graph's field map (§1.4/§1.6), not derives on `TieScore`.
2. No constant edits: 3/10/3, 180/360 closed intervals, and the absence of a
   `GROUP_ONLY_CEILING` in T4D are frozen (DECISION §3). No third demotion gate — the F04c
   「复燃一响」 fixture is a priced limitation whose only remedy is the §5 rollback chain.
3. No storage/SQLCipher/Tauri/HTTP imports, no wall clock, no default-as_of convenience path
   (DECISION §6.2/§6.3). The dependency arrow stays Goal 1 → soul-algo-tie, only.
4. No Goal 1 types in its API: the scorer keeps eating `(u64 rows, as_of)`; Uuid/Timestamp/
   SupportedBand conversions live in soul-graph.
5. No band arithmetic re-grown in soul-graph either: after this change `soul-graph` contains no
   numeric threshold literal, no `demoted`-style step, and no comparison of a count against a
   constant. If a fixture disagrees with a band, the answer is the DECISION §5 rollback chain,
   not a local patch. Guarded by T11.
6. Existing algo-crate tests (`as_of_discipline`, `direct_gate`, `goal1_fidelity`, `explain_zh`,
   `ablation`, `roundx_opus_a`) must still pass byte-identical — trivially true if the crate is
   untouched; run them anyway in CI.

## 6. Tests that must exist (Round 2 exit criteria)

New store-level tests in `crates/soul-graph/tests/` (extend `ego_graph.rs` or add
`t4d_band.rs`), plus one in soulcore. Names are binding intent, not binding spelling.

| # | Test | Fixture | Must assert |
|---|---|---|---|
| T1 | `a_group_flood_with_one_direct_hello_each_way_is_weak_at_the_store` | 30 group exchanges / 10 days both directions + exactly 1 direct out + 1 direct in | band `weak`; `direct_out==direct_in==1`; `types` still contain `Direct` + `Reciprocal` (shape ≠ band). The decisive DECISION §1 fixture, proven **through the store**, not just in the algo crate |
| T2 | existing `an_edge_carries_strength_type_last_contact_and_evidence` | unchanged `seed_three_partners` | passes **unmodified** — lilei (12 direct / 6 days, silent 0 vs store max) stays `Strong` (the lilei_12 anchor), zhaoqi group-only stays `weak`, exact `last_contact_utc` string preserved |
| T3 | `one_store_wide_as_of_demotes_the_dormant_not_the_active` | peer A: reciprocal 12-direct history ending 2019; peer B: one row at store max (recent) | A is `weak` (silence ≥ 360 vs store max), A's `silent_days` measured against store max; then a store holding **only** A: A is `Strong`, `silent_days == 0` (store max = A's own newest — the documented default, mirror of `as_of_discipline.rs`) |
| T4 | `a_group_message_yesterday_stops_the_dormancy_step_at_the_store` | peer's direct history 200 days stale + one group row 1 day before store max, same peer | band `Strong`, `silent_days == 1`, `last_direct_contact_utc` ≈ 200 days old (any-venue clock, DECISION §4.3) |
| T5 | `demotion_edges_are_closed_at_the_store` | strong direct history; second peer's row places store max exactly 180 (then 360) days after, same time of day | 180 → `moderate`; 360 → `weak` (closed intervals survive the string→unix→days pipeline) |
| T6 | `split_counts_round_trip_through_the_store` | mixed direct+group fixture | after rebuild + `load`: all §4 invariants; `algorithm_id == "T4D"`; `as_of_utc` present; `silent_days` matches hand count |
| T7 | `a_pre_wiring_edge_still_loads_and_the_next_rebuild_upgrades_it` | hand-`put_relationship` a legacy `tie_strength` JSON without the new fields | `load` succeeds (serde defaults, no `UnreadableEdge`); after rebuild the same edge carries the split fields |
| T8 | `a_broken_timestamp_fails_the_rebuild_and_names_the_row` | one observation with a non-RFC-3339 `occurred_at` | `Err(GraphError::UnreadableInteraction { evidence_id })`; nothing written |
| T9 | existing `rebuilding_updates_the_same_edges_instead_of_duplicating_them` + the rest of `ego_graph.rs` | unchanged | pass unmodified (idempotency, unresolved-peer counting, local_only, empty store) |
| T10 | `group_only_fifty_is_weak_not_moderate_at_the_store` | 50 group / 50 days reciprocal, never direct | band `weak`, `types` contain `GroupOnly` (DECISION §4.1 cost, shipped as priced) |
| T11 | `the_graph_source_holds_no_second_threshold` | `include_str!` over `src/build.rs`, `src/model.rs`, `src/view.rs` | none of `MODERATE_MIN`, `STRONG_MIN`, `MIN_INTERACTIONS`, `MIN_ACTIVE_DAYS`, `>= 10`, `>=10`, `>= 180`, `>=180`, `>= 360`, `>=360` appear (same mechanism as `roundx_a2_no_threshold.rs`; T1–T10 are the behavioral guard it admits it needs) |
| T12 | `the_audit_clock_is_not_the_scoring_clock` (soulcore) | same store, `rebuild(store, 0)` vs `rebuild(store, far_future)` | identical bands and identical `as_of_utc` on every edge; only audit timestamps differ |

Also keep: the two adapter unit tests already committed in `t4d_adapt.rs`, and
`soul-import/tests/import_to_graph.rs` green after rebase onto the concurrent G1+ change.

## 7. Out of scope for this round / this spec

- **No `band()` implementation this round** (Round 1 instruction). This document is the
  implementation contract; Round 2 opus writes the code.
- G3 (rebuild preserves `user_verdict`, locked effective band) — separate P0; §1.3 step 5.4
  keeps today's behavior on purpose.
- A2 / `points_for` rewiring onto the persisted split fields and COPY_ZH template-binding
  tests — R3, on top of the fields §1.6 provides.
- Goal 2 entirely. Any change to T4D constants or a third demotion gate — forbidden here and
  everywhere on this branch.
