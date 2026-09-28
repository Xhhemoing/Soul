MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 2 / gpt-sol-a — deltas only

## Priority / G4

- **Change the historical-speaker N:1 fan-out from P1 to P0.** T4D excludes
  group rows only from its count/day gates; `Tally::silent_days` deliberately
  reads any-venue `last_contact`. `direct_gate.rs` proves group rows can raise
  the final band through that clock. The current importer fabricates one
  outgoing row for every person who ever spoke in the conversation, so one
  current owner message can keep a 200-day-old direct tie Strong instead of
  demoting it. That is a band error, not only display-count pollution.
- In G4 and §5 step 4, fan-out repair must therefore be non-optional:
  owner group messages produce no peer row unless the source supplies a
  contemporaneous recipient/member set; an incoming group message still
  produces the actual sender's row.
- Keep dedup **before** T4D acceptance/wiring in §5. Wiring code may be drafted
  first, but it cannot be called correct while duplicate direct rows cross
  3/10 or duplicate/synthetic group rows refresh the recency clock. If any
  pre-dedup stores must survive, the claim/index also needs a backfill or a
  clean-store migration; future-only dedup does not repair old pollution.
- The stated key is underspecified for `soul-import-v1`: its parser accepts any
  nonempty `id` and does not require file/account-wide uniqueness, while
  Telegram already emits `chat_id:message_id`. Canonicalize per adapter
  (at least conversation/account namespace for JSONL), then hash
  `soul.import.dedup.v1|source|canonical_external_id`. Claim and message writes
  must be one transaction; persist only the digest.

## G1 / G2 type and contract deltas

- G1 persistence omits `as_of`, although DECISION §6.4 and tie `TieScore`
  carry it and A2 needs it. Persist `as_of_utc` with `silent_days`,
  any-venue `last_contact`, split counts, and `algorithm_id`; derive legacy
  totals only from the same score.
- The boundary adapter must also intern `conversation_ref` to a dense `u64`,
  not just peer UUIDs; `Interaction::conversation_id` drives the reported
  conversation count. Compute the one store-wide `as_of` before dropping an
  interaction whose peer contact is unresolved.
- **Widen A2 to `u64`.** `soul-algo-tie::TieScore` uses `u64` for all counts and
  day counts, but `soul-algo-trait::a2::TieScore` uses `u32`, including the
  optional direct/group split. Narrowing would truncate or force an artificial
  import limit. Widen interaction/out/in, active-day/conversation, and
  direct/group fields; add a value-above-`u32::MAX` renderer test.
- A2 also hard-codes `Vec<u64>` evidence IDs while Goal 1 uses UUIDs. Make the
  renderer's score/bullets generic over an opaque cloneable ID (preferred), or
  use an explicit reversible rebuild-local map; never truncate/hash UUIDs.
- G2's semantic fix is otherwise aligned with `apply_intake`: retain the
  answer event/evidence, refuse only the state transition, expose
  `axis_locked_by_user`, and remain LastWriteWins. For replay equality, map
  UUID evidence IDs bijectively to the reference crate's `u64` IDs and compare
  citations after mapping; “全等” is otherwise not type-defined.

## §5 order delta

- Keep `dedup + corrected group observation semantics` before T4D.
- Split current step 5: first wire T4D/global `as_of`/split machine score.
  Do graph correction/effective-band persistence before wiring A2, because A2
  must consume the effective user-visible band, not the raw machine band.
  A0 intake is independent and may remain between those commits.

## Missing explicit Goal 1 integration tests

- Round 1 §5 cases **1–7, 9, and 12** are not required explicitly in
  BLOCKERS.md. Standalone algorithm tests do not prove the Goal 1 adapters.
- Add product-boundary tests for: `lilei_12`; flood + one direct each way with
  exact splits; group-only Weak; direct 2/3, 9/10 and day 2/3 corners;
  179/180/359/360 recency; a dormant peer using another peer's newer store
  `as_of` plus the per-peer-max negative control; and no N-row owner fan-out.
- Add the A2 integration assertion that it renders persisted split counts and
  the **effective** band, and that changing counts alone cannot change a
  bullet's band. Retain the source guard over both A2 and the replacement for
  `soul-draft::points_for` so no second 3/10/3 table returns.
- Cases 8, 10, and 11 are already named by G4, G3/GC, and G2 respectively;
  strengthen case 8 with two concurrent claims of the same digest: exactly one
  event/evidence set commits and the other receipt reports a duplicate.
