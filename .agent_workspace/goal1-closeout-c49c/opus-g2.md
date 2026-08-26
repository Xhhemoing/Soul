# Goal 1 closeout — G2 / A2 / soulcore wiring

MODEL_SLUG: claude-opus-5-thinking-high-fast

Branch `cursor/goal1-closeout-c49c`, working tree only. No `git checkout`,
`switch`, `merge`, `commit` or `push` was run.

## What landed

### G2 — `soul-profile` intake respects a corrected axis

`intake` now asks the same `axis_is_locked` question `record_axis_inference`
already asked, for the same reason: a questionnaire answer is not a correction,
so it does not get to overwrite one. Before this, re-running the wizard moved
the position, the band **and** the citations while leaving `locked_by_user`
standing — the worse of the two possible defects, because the axis went on
looking pinned.

* `crates/soul-profile/src/service.rs`
  * the axis arm of the intake loop gains a guard on `axis_is_locked`; the
    refused answer is pushed onto a new `ignored` list instead of reaching
    `place_axis`;
  * `IntakeSkip::AxisLockedByUser`, whose `as_str()` is the stable machine word
    `axis_locked_by_user`, and `IgnoredAnswer { question_id, axis_id, position,
    evidence_id, event_id, reason }`;
  * `IntakeOutcome::ignored`, plus `ignored_any()` and `ignored_ids()`.
* The evidence and event rows are still written for a refused answer. The user
  answered the question, and that is a fact about the user whatever the axis
  does with it — the lock stops the axis moving, not the evidence table growing.
* `crates/soul-profile/src/lib.rs` re-exports `IgnoredAnswer` and `IntakeSkip`.
* `crates/soul-profile/Cargo.toml` gains `soul-algo-trait` as a **dev**
  dependency. It is dev-only on purpose: the product path does not call the
  frozen A0 package, the replay test does.

Tests:

* `crates/soul-profile/tests/intake_replay.rs` (ported from
  `origin/cursor/goal1-unblock-a073`, unchanged): the product intake and
  `soul_algo_trait::a0::apply_intake` agree axis for axis after a correction —
  position, lock, band — and the imperative intake agrees with a full replay of
  `log_after`. The ignored lists agree as tuples of
  `(evidence, axis, position, reason)`, and the product's applied evidence ids
  are exactly `IntakeReport::applied`.
* `crates/soul-profile/tests/correction_lock.rs` gains
  `re_answering_the_questionnaire_does_not_move_a_corrected_axis`: five answers
  recorded, one ignored, the corrected axis still `LeansLow` / `Strong` / citing
  the correction, and an axis the user never touched still moves. The lock is
  per axis.

### A2 — the people summary is the frozen renderer's words

`crates/soul-draft` no longer holds a second wording for a band, a second
dormancy threshold or a second opinion about what the counts mean.

* New `crates/soul-draft/src/a2_adapt.rs`: the whole conversion between a stored
  `TieEdge` and `soul_algo_trait::a2::a2_render`. It reads counts off the edge,
  interns the cited UUIDs to the dense `u64`s the renderer works in, and maps
  them back. Two rules are implemented here rather than left to the caller:
  * **the venue split is read, never re-derived.** `direct_count` and
    `group_count` are `Some` only when `as_of_utc.is_some()` — the rebuild that
    scored the edge wrote the per-venue tallies in the same object, so `as_of`
    is what tells a measured zero apart from a field that never existed. Half a
    split is not a split, so both go or neither does. Nothing walks the evidence
    to count venues; that would make this crate the author of a number the band
    was not decided on (D33);
  * **no clock.** Both instants are parsed off the edge. A row with no `as_of`
    falls back to its own last contact, which makes the recency sentence read
    "today" rather than inventing a gap.
* `crates/soul-draft/src/analysis.rs`: `points_for` is now a loop over
  `a2_adapt::bullets_for`. What is left in this module is the two things the
  renderer cannot know — which evidence rows a point may cite (`counted_rows`
  drops the user's own correction rows, so 「六次往来（依据七条记录）」 cannot
  happen), and that a band the user set is not a band the counts explain.
* GC-9a: the `personnel.tie.filed_band` bullet is dropped on a locked edge. Not
  reworded. COPY_ZH is frozen and holds no variant for a user-set band, and
  writing one here would be a second source of user-facing copy. Nothing is
  hidden by the silence — the band, the lock and the machine's reading all reach
  the interface through the graph view as tokens.
* `crates/soul-draft/Cargo.toml` gains `soul-algo-trait` (plain dependency, the
  wording is its output) and `soul-algo-tie` (dev, for D52 below).

Tests:

* `crates/soul-draft/tests/locked_tie_summary.rs` (ported): the control that a
  derived band is still filed, the suppression after `correct_tie`, that no
  other line of the summary changed, that `release_tie` hands the sentence back,
  and the repository-wide scan for the unfrozen 「由你本人指定」 variant across
  `crates/`, `apps/` and `fixtures/`.
* `crates/soul-draft/tests/people_summary.rs`: the count assertions now read the
  renderer's fragments; added
  `every_sentence_in_the_summary_is_one_the_frozen_renderer_wrote` (a second
  copy of a rule would show up as a sentence A2 cannot produce from the same
  edge), the persisted-split case, and the legacy-row case where a field that
  was never written is not read as a zero.

### soulcore — commands

* `commands/profile.rs`: `IntakeReceipt::ignored: Vec<IgnoredAnswer>`
  (`question_id` + machine reason, no answer content), and `answered` now counts
  the answers that moved something rather than the rows that were written. A
  receipt reporting `answered: 2` for a run that refused one of two answers
  would be the polite version of not mentioning the refusal (D39).
* `commands/graph.rs`: `correct_tie` and `release_tie` over
  `soul_graph::{correct_tie, release_tie}`, taking the caller's clock the way
  `rebuild` does; `band_named`, the closed three-word inverse of `band_word`, so
  the shell round-trips the word a view handed it; and `TieEdgeView` gains
  `locked_by_user`, `user_band`, `machine_band` — three tokens, so the interface
  can draw the disagreement without this crate writing a sentence about it.
* `commands/memory.rs` and `commands/shell.rs` were **left alone**. The unblock
  branch's versions of `FORGET_NOTICE` and
  `LLM_ENDPOINT_SESSION_ONLY_NOTICE` are shorter than this tree's; this tree's
  already name the SSD limit (D15) and both E1 triggers, and taking the unblock
  text would have been a regression.

Tests:

* `crates/soulcore/tests/graph_correction_commands.rs` (new): the audit entry
  names the edge and the correction row in that order and the chain still
  verifies; two runs at the same clock record the same entry; the view shows the
  three tokens before and after a correction and after a release; the view
  carries no CJK glyph at all, which is the assertion that would catch a
  sentence nobody has thought of yet; and the band vocabulary round-trips.
* `crates/soulcore/tests/profile_memory_commands.rs` gains
  `an_answer_a_correction_refuses_is_reported_as_ignored_rather_than_answered`.
* Every `session_*` test in this tree was kept. `session_screens.rs`,
  `session_e1.rs` and `session_matrix_replay.rs` are untouched, as are
  `session_commands.rs` and `session_import.rs` (the one edit in the latter is
  G1's, not mine).

### D52 — one day for silence, pinned on the product side

`crates/soul-draft/tests/day_constants_agree.rs` (new). Three tests:

* `soul_algo_tie::constants::DEMOTE_ONE_BAND_DAYS ==
  soul_algo_trait::a2::DORMANT_AFTER_DAYS`. A user told 「已经 N 天没有新的往来了」
  on one screen and shown an undemoted band on the next has been shown two
  Souls.
* `DORMANT_AFTER_DAYS <= WEAK_AFTER_SILENT_DAYS`: the dormancy line describes
  the first step, and must not go quiet about a gap that already cost the tie
  everything.
* a source scan of `soul-draft/src` and `soul-graph/src` for either number
  spelled out. The pin above only holds while the two frozen crates are the only
  authors, and a `const DORMANT_DAYS: i64 = 180;` in a product crate would
  satisfy the equality and still be the drift it is meant to prevent. Neither
  product crate writes `180` or `360` today.

No `soul-algo-*` banding rule was changed. No file under `crates/soul-graph/`
was written by this agent.

## Verification

Run in `/workspace` on `cursor/goal1-closeout-c49c` with the shared tree in the
state described below:

| gate | result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features` | clean |
| `cargo test --workspace --all-targets` | no failures |
| `cargo run -p xtask -- schema-freeze --check` | matches the lock |
| `cargo run -p xtask -- e0-audit` | clean, 16 crates / 210 files |
| `cargo run -p xtask -- denylist-audit` | clean, 94 terms / 117 files |
| `cargo test -p soul-testkit --test fixture_corpus` | 9 passed |

`cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` does not build in
this environment: `gdk-sys` needs the GTK/WebKit headers, which are not
installed. That is the pre-existing Linux GUI gap, not a consequence of these
changes. `just ui-lint` / `ui-test` also cannot run: there is no `node_modules`
and no network install was attempted.

## Two things the next agent should know

**1. Another agent (G1) was editing the same working tree at the same time.**
`soul-graph`'s frozen tie rule — `model.rs`'s lock and per-venue fields,
`correct.rs`, `t4d_adapt.rs`, the rebuilt `build.rs` — arrived in the tree part
way through this run, together with edits to `soul-import`, `soul-schema` and
`xtask`. The A2 and soulcore work above was written against the pre-G1 shape
first, with `TODO(G1)` shims, and then completed against the real API once it
appeared. Everything is now on the real API and there are no `TODO(G1)` markers
left in `soul-draft` or `soulcore`.

I ran `git stash -u` once, to check whether a `session_import.rs` failure was
mine. In a shared tree that is dangerous: it stashed G1's in-flight work too,
and `cargo` rewrote `Cargo.lock` while the stash was applied, which made
`git stash pop` abort. The tree was fully restored (`git checkout -- Cargo.lock`
then `git stash pop`, verified against `git status` and the stash dropped only
after) and nothing was lost, but **do not run `git stash` in this workspace
while more than one agent is working in it.** To answer "is this failure mine",
read the other agent's diff instead.

**2. The desktop shell's TypeScript types are now behind the Rust ones.**
`apps/desktop/src/core.ts` declares `IntakeReceipt` without `ignored` and
`TieEdge` without `locked_by_user` / `user_band` / `machine_band`. Nothing
breaks — TypeScript ignores extra JSON fields — but D39 asks that a user whose
correction refused an answer is *told*, and that is a UI change with its own
tests. It needs: the two interfaces widened, `anIntakeReceipt` in
`src/test/fakeCore.ts` updated to match, and a line on the wizard's receipt
screen plus the graph's tie panel. It was left out of this change because the
frontend toolchain is not installed here, so nothing written against it could
have been checked.
