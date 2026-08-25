# Goal 1 unblock — PROGRESS

Branch: `cursor/goal1-unblock-a073` (not `agent/` — FORMAL §11.5).
Base: `origin/cursor/soul-goal1-7b1c` @ `80c9011` + merge `origin/main` (T4D/A0 crates).
Parent run: `bc-b296c4d9-0feb-4f6e-b844-aeaca7a8a073`.
Do not start Goal 2. Do not silent-patch F04c. Do not pull SQLCipher into algo crates.

## Already true on Goal 1 HEAD (do not redo)

- DPAPI: `soul-win-dpapi`
- Import UI `/import` + collect UI `/collect` + wizard 11Q + `/profile`
- KnownIdentifiers filled from display names (STATUS)
- Hosted CI empty-runners is minutes, not a code task

## Remaining P0 this branch must close

| ID | Work | Owner round |
|---|---|---|
| M2 | Absorb main — **done** (merge commit) | parent |
| M3 | Exact fileplan corpus guard + runtime case probe; reject `>= 25` / `cfg(unix)` decoy | R1 gpt-sol-a |
| G2 | `intake` skips locked axes (`apply_intake` semantics) | R1 opus-a |
| G1+ | Owner group messages must not fan Outgoing to historical speakers | R1 opus-b |
| G1 | `Tally::band` → `soul_algo_tie::score`; one store-wide `as_of` | R2 opus |
| G3 | rebuild preserves `user_verdict`; locked effective band | R2–R3 |
| A2 | `points_for` / COPY_ZH; no second 3/10/3 | R3 |

## Round status

- Round 1: in flight

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
