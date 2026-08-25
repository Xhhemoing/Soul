# Goal 1 unblock — PROGRESS

Live stacked loop: `.agent_workspace/close-loop/PROGRESS.md` on `cursor/goal1-close-loop-a073`.
Do not treat this file’s historical unblock rounds as the live work order.

Branch: `cursor/goal1-unblock-a073` (not `agent/` — FORMAL §11.5).
Base: `origin/cursor/soul-goal1-7b1c` @ `80c9011` + merge `origin/main` (T4D/A0 crates), now absorbing `origin/main` plan-doc freeze (PR #8: D41–D60, typed `tie_strength`, `PLAN_INDEX.md`).
Parent run: `bc-b296c4d9-0feb-4f6e-b844-aeaca7a8a073`.
Do not start Goal 2. Do not silent-patch F04c. Do not pull SQLCipher into algo crates.
Plan-polish workspace notes live under `.agent_workspace/plan-polish/` (historical; not the live header).

## Already true on Goal 1 HEAD (do not redo)

- DPAPI: `soul-win-dpapi`
- Import UI `/import` + collect UI `/collect` + wizard 11Q + `/profile`
- KnownIdentifiers filled from display names (STATUS)
- Hosted CI empty-runners is minutes, not a code task

## Remaining P0 this branch must close

| ID | Work | Owner round |
|---|---|---|
| M2 | Absorb main T4D/A0 crates — **done**; absorb plan-doc freeze (PR #8) — **this merge** | parent |
| M3 | Exact fileplan corpus guard + runtime case probe; reject `>= 25` / `cfg(unix)` decoy | **done** (marker-file probe) |
| G2 | `intake` skips locked axes (`apply_intake` semantics) | R1 opus-a |
| G1+ | Owner group messages must not fan Outgoing to historical speakers | R1 opus-b |
| G1 | `Tally::band` → `soul_algo_tie::score`; one store-wide `as_of` | R2 opus |
| G3 | rebuild preserves `user_verdict`; locked effective band | **done** (R2) |
| A2 | `points_for` → `a2_render`; P1b from persisted split; no second 3/10/3 | **done** (R3) |
| G2 receipt | `IntakeReceipt.ignored`; answered does not count lock-refused rows | **done** (R3) |
| Import clock | Telegram year 1970–9999; civil month lengths | **done** (R3) |

## Round status

- Round 1: done — `.agent_workspace/unblock/R1-SYNTHESIS.md`
- Round 2: done — `.agent_workspace/unblock/R2-SYNTHESIS.md`
- Round 3: done — `.agent_workspace/unblock/R3-SYNTHESIS.md`（A2 接线、IntakeReceipt.ignored、导入时钟、store 近阈值、G2 对拍）
- 不再开 Round 4–6：原 P0 已关；剩余项见 R3 简报「仍开放」表（D34/D35/D36、作者手动、空 runner）
- 壳层缺口 `3161e02` 已 cherry-pick：profile/memory/research/crash/redirect 走产品 Session；Linux CI 另跑 ipc_roundtrip（本机无 webkit 编不了 tauri 壳）
- M3 探针改为「一种拼写写入标记、另一种拼写读回」，不再依赖 canonicalize 的大小写

## Round 1 fable-a — G1 T4D wiring spec delivered (spec only, no code)

- Spec at `.agent_workspace/unblock/round1/fable-a-T4D_WIRING.md`; Round 2 opus implements from it without redesigning.
- Wiring: `rebuild` keeps one evidence pass; rows adapted per peer via the committed `InteractionInterner` (4da2184), then `soul_algo_tie::score` per peer; band opaque to soul-graph after the swap.
- Intern: fresh interner each rebuild; dense u64 keyed on the full peer UUID AND the full conversation_ref; never hashed, truncated, or persisted.
- One as_of: max(occurred_at) over every observation read (whole store, incl. unresolved peers); never per-peer, never wall clock, never soulcore's audit `at_unix_seconds`.
- Persist: `TieStrength` gains serde-defaulted split fields (direct/group out+in, direct_active_day_count, last_direct_contact_utc, silent_days, as_of_utc, algorithm_id="T4D"); legacy rows still load; contract-legal (tie_strength is a free object).
- Delete in `build.rs`: local 3/10/3 constants, `Tally` + `band()/strength()/types()`, `utc_date`; display timestamps keep the original stored strings.
- Algo crates take zero diffs: no serde/deps, no constant edits, no third demotion gate, no storage/clock (DECISION §3/§5/§6).
- Tests specified T1–T12: decisive group-flood fixture at store level, store-wide as_of discipline, closed 180/360 edges, split round-trip, legacy-row load, broken-timestamp failure, no-second-threshold source scan, audit-clock isolation; ego_graph.rs must pass unmodified (lilei_12 anchor).
- Out of scope: soul-graph `band()` implementation this round, G3 verdict preservation, A2/COPY_ZH rewiring (R3), Goal 2.
- R1 fable-b: G3 spec delivered — `.agent_workspace/unblock/round1/fable-b-G3.md` (docs only, no
  crate changes). Refines round2/fable-b/GRAPH_CORRECTION.md against ALGO_FROZEN + frozen COPY_ZH:
  fields (3 optional lock fields on `TieStrength`, `first/last_contact_utc` → `Option` for GC-7),
  rebuild clobber rules R1–R6 (verdict preserved verbatim, band = effective, forget beats lock,
  correction-row survivor edge, byte-identical idempotence scoped to graph rows), soulcore
  `correct_tie`/`release_tie`/`band_named` (audit reuses `ProfileCorrect`, receipts are tokens —
  no new Chinese copy while COPY_ZH is frozen). GC-1..8, 10 implementable this branch; **GC-9
  split**: GC-9a (suppress the filing sentence on a locked edge — frozen P5 must not render, no
  homemade variant either) implementable now; GC-9b (「由你本人指定」 variant) blocked on an
  additive COPY_ZH key via DECISIONS (fable-a slot). G3 composes with G1 in either landing order.

## Round 2 gpt-sol-b — T4D product rebuild gates delivered

- Added `crates/soul-graph/tests/t4d_product.rs`, using the real SQLCipher test
  store and `soul_graph::rebuild` for all four requested cases.
- `cargo test -p soul-graph` compiles the gate. The 12 reciprocal direct rows
  over six days pass as Strong. The three T4D-discriminating cases are
  expected-red while rebuild still uses its local T0 tally: 100 group + one
  direct each way, group-only volume, and a dormant peer scored against a
  newer store-wide observation all return Strong instead of Weak.
- No constants or scorer code changed.

## Round 2 opus-b — G3 landed: tie correction, GC-9a suppression, evidence union

Branch `cursor/goal1-unblock-a073`. Implements `unblock/round1/fable-b-G3.md` on the
product crates. Written alongside R1/R2 (opus-a's `814064e`), which is why the two
touch `build.rs` in different places.

Landed:

- `soul-graph/src/correct.rs`: `correct_tie` / `release_tie` / `TieCorrection` /
  `corrected_relationship`, the four steps in `correct_axis`'s order. Read the edge first,
  so a correction to an edge that does not exist fails with the store's own `NotFound`
  and writes nothing (GC-4). `UserCorrection` row carries three keys — origin,
  relationship_id, band — and no prose. Audit reuses `ProfileCorrect` with
  `about = [relationship_id, evidence_id]`; no schema and no audit enum changed.
- `soul-graph/src/build.rs` R3 (evidence union): the correction rows are collected in the
  pass that already reads the evidence table and unioned into `edge.evidence_ids`. The
  inference keeps citing observations only — the row rejecting the machine's statement is
  not support for it. Both lists come out of a `BTreeSet`, so GC-10 still holds.
- `soulcore/commands/graph.rs`: two thin passthroughs plus `band_named` (closed set,
  inverse of `band_word`); `TieEdgeView` gains `locked_by_user` / `user_band` /
  `machine_band` as tokens. `band` keeps meaning "the band in force", so every existing
  reader respects a correction without knowing one happened. TS mirror updated.
- `soul-draft/src/analysis.rs` GC-9a: the filing point is not pushed on a locked edge, and
  no variant replaces it — COPY_ZH is frozen and holds no wording for a user-set band.
  Correction rows also stop being counted as support for the count sentences.
- Tests: `soul-graph/tests/graph_correction.rs` (GC-1..5, GC-10, plus the two corner
  decisions and the `machine_band`-always-present pin), `soulcore/tests/
  graph_correction_commands.rs` (GC-8 chain entry, same-clock replay, view tokens,
  `band_named` closed set), `soul-draft/tests/locked_tie_summary.rs` (GC-9a, the control
  that an uncorrected edge is still filed, and a tree-wide scan for unfrozen filing copy).

Open for Round 3, unchanged by this work:

- R4 (forget beats lock) and R5 (locked edge whose observations are all forgotten) —
  rebuild still visits only peers with a tally, so GC-6 and GC-7 are not implementable
  yet. R5 additionally needs the `first/last_contact_utc` → `Option` ruling in G3 §1.1,
  which is a decision, not a default.
- GC-9b waits on an additive COPY_ZH key (fable-a slot).
- G3 §1.1's "all three lock fields absent while unlocked" does not match what landed:
  `machine_band` is written on every rebuilt edge. The implementation choice is pinned by
  `an_unlocked_edge_still_records_what_the_counts_say`; the spec text needs the amendment
  fable-b's Round 2 review (2.4-1) proposes.
- G3 §5's tree-wide grep is scoped in the landed test to everything but the enforcing
  file, per the same review's 2.4-3.
