claude-opus-5-thinking-high-fast

# R3 opus-a — mixed-store `UnscoredEdge` refusal, pinned

Slot: opus-a, Round 3 of Goal 1 close-loop. Branch: `cursor/goal1-close-loop-a073` (no side branch).
Scope: the one optional item R2-SYNTHESIS left open — fable-a's closing sentence, "a mixed-store
variant of the refusal test — one scoreable peer beside the forgotten one".

## Verdict: gap was real, one test added

The existing suite did **not** pin it. Every refusal test runs on a single-peer store:

- `an_edge_no_rebuild_can_score_is_refused_and_names_the_rebuild` writes the owner contact and no
  peer contact at all, so `rebuild` finds zero scoreable peers, `edges_written` is empty, and the
  refusal path never exercises the case where the rebuild inside `scored()` rewrites somebody
  else's edge. That is exactly the fixture caveat fable-a flagged at REPORT.md:58–61.
- `correcting_an_edge_that_does_not_exist_writes_nothing` refuses at `get_relationship`, before any
  rebuild runs.
- `a_corrected_band_and_its_verdict_survive_a_rebuild` (GC-1) pins that a lock survives an
  *explicit, successful* rebuild, not one run as a side effect of a call that then errors.

So the combination — a locked peer sitting beside a forgotten one, when the correction on the
forgotten one refuses — had no coverage. One test now covers it.

## What was added

`crates/soul-graph/tests/graph_correction.rs`, commit `0f8a073`, one test plus three fixture
helpers. No source change; `crates/soul-graph/src/` is untouched, and the test passes against the
code exactly as R2 left it (this is a pin, not a fix).

New test: `refusing_a_forgotten_peer_leaves_the_scoreable_peers_lock_alone`.

Store shape: the usual `seeded()` peer (machine reading Strong), corrected to Moderate and settled
with one rebuild, plus a second peer whose contact row is never written, observed once on
`2026-08-01` and holding a pre-wiring edge (`algorithm_id: ""`). The second peer's observation is
older than everything the first peer said, so the store-wide `as_of` the rebuild scores against
does not move and the byte comparison below is about the refusal rather than about the clock.

`correct_tie` on the forgotten peer's edge refuses with `GraphError::UnscoredEdge` naming that
edge, and after it:

| Asserted | Why it matters |
|---|---|
| Both edges byte-identical, verdict identical, evidence/edge/inference counts identical | the refusal rewrites neither the edge it was about nor anybody else's, and invents no edge for the forgotten peer |
| `band == Moderate`, `user_band == Some(Moderate)`, `machine_band == Some(Strong)`, `is_locked_by_user()`, verdict `Corrected` | the lock is spelled out as well as compared, so a failure names the part that went |
| The locked edge still cites the correction's evidence row | the row explaining the band survives the neighbour's failed call — constraint 10's no-black-box, applied to the refusal path |
| `list_audit().len() == before + 1`, that entry `InferenceWrite`, `subject_refs` contains the locked edge and not the refused one | the only artifact gained is the rebuild's own, which an import would have written anyway; **no** `ProfileCorrect` — a refused correction records no correction |

The audit assertion is what makes the test non-trivial: it proves the rebuild really did pass over
the scoreable peer during the refused call, so the lock assertions are checked after the risky path
ran, not after a no-op.

### Fixture helpers (delegation, no churn at existing call sites)

- `forgotten_peer()` → `id("003")`.
- `observe_peer(store, evidence_id, peer_contact_id, direction, at)`; the existing `observe` is now
  a one-line delegate with `peer()`, so its five call sites are unchanged.
- `pre_wiring_edge_to(store, relationship_id, to_contact_id, evidence_id)`; the existing
  `pre_wiring_edge` delegates with `id("321")` and `peer()`, so its two call sites are unchanged.

## Non-vacuity check

Temporarily mutated `tie_strength_of` in `src/build.rs` to ignore the held lock, ran the new test
alone: **FAILED** at `left: Strong, right: Moderate` ("the user's band"). Mutation reverted;
`git status` clean apart from the test file before commit. Worth noting which assertion caught it —
the byte-equality snapshot did *not*, because under the mutation the pre-refusal settle rebuild had
already dropped the lock, so `before` and `after` matched each other while both were wrong. The
spelled-out lock assertions are what hold, which is why they are there beside the snapshot.

## Test run

`cargo test -p soul-graph --offline`: green — 2 + 7 + **12** + 11 + 16 passed, 0 failed
(`graph_correction` went 11 → 12). `cargo fmt --check -p soul-graph` clean;
`cargo clippy -p soul-graph --tests --offline` clean. Workspace `Cargo.lock` untouched.

## Scope compliance

Nothing outside `crates/soul-graph/tests/graph_correction.rs` and this file. No Goal 2, no F04c, no
algo crate constants, no `ci.yml`, no new features, no new AC rows, no source change, no empty
commit. Stayed on `cursor/goal1-close-loop-a073`; no `cursor/goal1-r3-*` branch created.
