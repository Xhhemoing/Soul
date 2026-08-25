claude-fable-5-thinking-xhigh

# R1-REVIEW — read-only review of `9fd6870` (R-1 fix)

Slot: fable-a, Round 2 of Goal 1 close-loop. Scope: pass/fail on the five gates named in the
task, against commit `9fd6870` ("soul-graph: score a pre-wiring edge before locking its band")
as it sits in this branch's history. No code was changed; nothing outside this file was written.

## Verdict: PASS on all five gates

| Gate | Verdict |
|---|---|
| 1. Schema not loosened | PASS |
| 2. T4D zeros still serialized | PASS |
| 3. Rebuild-before-lock leaves no half-written correction on `UnscoredEdge` | PASS |
| 4. No third demotion gate | PASS |
| 5. No algo crate edits | PASS |

## Evidence per gate

### 1. Schema not loosened — PASS

`git show 9fd6870 --stat` touches exactly three files, all under `crates/soul-graph/`:
`src/correct.rs`, `src/error.rs`, `tests/graph_correction.rs`. `docs/schemas/relationship.schema.json`
was last changed at `c3d5960`, before the fix. In the current tree the contract is intact:
`tie_strength.algorithm_id` enum is still exactly `["T4D", "T4"]`, `additionalProperties: false`
holds, the eight `dependentRequired` clauses tying the split fields to `algorithm_id` hold, and the
`if algorithm_id / then required [...15 fields...]` block is unchanged. The fix closes R-1 by
rebuilding first, not by teaching the enum the empty string.

### 2. T4D zeros still serialized — PASS

`crates/soul-graph/src/model.rs` is untouched by the commit. On `TieStrength`, every T4D count
(`direct_out_count`, `direct_in_count`, `group_out_count`, `group_in_count`,
`direct_active_day_count`, `silent_days`) carries `#[serde(default)]` with **no**
`skip_serializing_if`, so zero values serialize. `skip_serializing_if = "Option::is_none"` exists
only on the genuinely optional fields (`last_direct_contact_utc`, `as_of_utc`, and the lock trio),
which matches the schema's `then.required` list — it requires all counts plus `as_of_utc` and
deliberately omits `last_direct_contact_utc`. The new test
`correcting_a_pre_wiring_edge_scores_it_before_locking_it` re-validates the corrected row against
the frozen document via `SchemaSet::load().validate_model(SchemaId::Relationship, ...)`, and it
passes.

### 3. Rebuild-before-lock, no half-written correction on `UnscoredEdge` — PASS

`scored()` is the first statement in both `correct_tie` (`correct.rs:101`) and `release_tie`
(`correct.rs:147`). All four correction writes — the `UserCorrection` evidence row, the edge
rewrite, the verdict, the correction's audit entry — happen only after `scored()` returns `Ok`.
The `UnscoredEdge` refusal is raised inside `scored()` before any of them, so a refused call
writes no correction artifact. The refusal test
(`an_edge_no_rebuild_can_score_is_refused_and_names_the_rebuild`) asserts strict before/after
equality on evidence count, inference count, audit count, and the full edge JSON, and additionally
that `release_tie` refuses the same way. Both R-1 tests pass; the full `graph_correction` suite is
11/11 green, run with `cargo test --locked` (workspace `Cargo.lock` untouched).

One honest nuance, not a fail: `scored()` appends the rebuild's own audit entries before the
unscored check, so on a mixed store (the target peer forgotten, other peers scoreable) the
rebuild's idempotent edge/inference/audit writes land even though the correction then refuses.
Those are rebuild artifacts an import would have produced anyway — not correction state — and the
strict-equality test holds in its fixture because `rebuild` emits audit only when
`edges_written` is non-empty.

### 4. No third demotion gate — PASS

`scored()` contains no band arithmetic: it either returns the stored strength unchanged
(`algorithm_id` non-empty), reruns the existing `rebuild` → frozen `tie_reading` path, or refuses.
No new threshold, no new demotion condition, no F04c-shaped logic. Searching `crates/soul-graph`
for demotion logic finds only the pre-existing `t4d_product.rs` / `t4d_band.rs` tests. The schema
comment guarding the two-value enum against a smuggled third rule is unchanged.

### 5. No algo crate edits — PASS

The commit's file list is exhaustive: `src/correct.rs` (+60/−8), `src/error.rs` (+11),
`tests/graph_correction.rs` (+188). No `soul-algo-*` path, no `docs/` path, no constant changes.

## Additional observations

- Lock lands on the rebuilt row: after `scored()` rescored the edge, `rewritten()` receives the
  fresh T4D row, so the lock is written onto a full reviewable surface (`algorithm_id: "T4D"`,
  zeros present, `as_of_utc` set) — the success test asserts exactly this, including that
  `machine_band` is the frozen rule's reading of the real observations (Weak from one hello), not
  the legacy row's claim.
- Identity preserved: `rebuild` matches the pre-wiring edge by contact pair and reuses its
  `relationship_id`, so the correction the caller asked for lands on the edge the user was
  looking at.
- The `UnscoredEdge` message names the remedy ("rebuilt"), which the refusal test pins.
- The branch moved ahead during this review (`a29f792`, `0418983` — DOC-D61; `9ccadb6` — AC-34
  test); none touch `soul-graph`, so they do not affect this verdict.

Follow-up (one sentence, optional): a mixed-store variant of the refusal test — one scoreable
peer beside the forgotten one — would pin down that the refusal path's only store changes are the
rebuild's idempotent artifacts and never correction state.
