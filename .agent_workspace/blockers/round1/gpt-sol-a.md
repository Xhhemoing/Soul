# Round 1 / gpt-sol-a — Goal 1 algorithm-adoption blockers

## Verdict

Goal 1 cannot be called a T4D/A0 implementation yet. The reviewed Goal 1 branch still:

- bands on all venues with the old reciprocal 3/10/3 rule and has no recency step;
- lets questionnaire intake overwrite an axis that is visibly user-locked;
- duplicates messages on re-import and fans one owner group message out to every historical speaker in the conversation;
- recreates tie inferences with `user_verdict = Unreviewed` on every rebuild;
- has a separate `soul-draft::analysis::points_for` summary path rather than using the frozen A2 renderer.

Priority calls:

| Item | Priority | Release consequence |
|---|---|---|
| T4D wiring, one store-wide `as_of`, venue-split fields | **P0** | Goal 1 otherwise still ships T0. |
| A0 intake lock semantics and replay equality | **P0** | Current intake silently moves a user-corrected axis while leaving the lock displayed. |
| Import dedup using `sha256("soul.import.dedup.v1|source|external_id")` | **P0 for T4D correctness**, not P1 after wiring | Re-importing the same direct messages can cross T4D's 3/10 gates; duplicate group rows can also refresh T4D's any-venue recency clock. |
| Remove unsupported group N:1 owner-message fan-out | **P0 for T4D correctness** | T4D excludes group rows from count gates, but group rows deliberately drive `last_contact`; fan-out can therefore prevent 180/360 demotion for historical speakers who were not shown to have received the message. |
| Graph correction and verdict survival | **P0 product-lock** | “User can correct the people graph” is a product contract. Merely preserving a decorative verdict is insufficient if rebuild still overwrites the effective band. |
| A2 replacement of the existing people-summary rule path | **P0 adoption obligation** | Keeping both renderers creates two explanations and invites a second band table. |

## Evidence reviewed

- `docs/algorithms/DECISION.md`, especially §3 and §6.
- `crates/soul-algo-tie` on `origin/main` (`7b35bde`): default `score` is T4D; its tally already carries direct/group direction counts, direct days, any-venue `last_contact`, and caller-supplied `as_of`.
- `crates/soul-algo-trait` on `origin/main`: A0 `apply_intake` and its replay-equality test; A2's threshold-free renderer.
- Goal 1 branch `origin/cursor/soul-goal1-7b1c` (`3e88b48`): `crates/soul-graph/src/build.rs`, graph model/interaction code, `soul-profile/src/service.rs`, importer commit/Telegram paths, and `soul-draft/src/analysis.rs`.

The decision document calls the Goal 1 file `graph_build.rs`; the live branch path is `crates/soul-graph/src/build.rs`.

## 1. Exact T4D and A2 wiring plan

### 1.1 Workspace dependency direction

When Goal 1 is integrated, add both algorithm crates to the product workspace and workspace dependencies.

- `soul-graph -> soul-algo-tie`
- `soul-profile -> soul-algo-trait`
- `soul-draft -> soul-algo-trait`
- A2's descriptive dormancy constant must alias the tie crate's `DEMOTE_ONE_BAND_DAYS`; while the crates remain separate, the acceptable internal dependency is `soul-algo-trait -> soul-algo-tie`.

No arrow may point from either algorithm crate to `soul-store`, `rusqlite`, SQLCipher, Tauri, HTTP, or a wall clock.

### 1.2 Replace `Tally::band`, do not wrap it

In `soul-graph/src/build.rs`:

1. Delete the three local threshold constants and the current `Tally::band`.
2. Convert every `InteractionRef` at the graph boundary into `soul_algo_tie::Interaction`:
   - `Direction::Outgoing` -> `outgoing = true`;
   - `Venue::Direct` -> `venue_direct = true`;
   - parse the normalized RFC 3339 timestamp to Unix seconds, returning a graph/import error on invalid input;
   - intern UUID peers and conversation hashes to dense rebuild-local `u64` keys. Do not truncate or hash a UUID into `u64`, because a collision would merge peers or conversations.
3. A minimal linear implementation may retain one Goal 1 `Tally` per peer, but that tally should hold the peer's algorithm rows plus Goal 1 evidence UUIDs. Give all rows in that peer-local vector the same local peer key and call:

   ```text
   soul_algo_tie::score(local_peer_key, &tally.rows, rebuild_as_of)
   ```

   This is the semantic replacement for `Tally::band`; there must be no fallback call to the old method. Alternatively, one globally interned vector may use `score_ego_network` once.
4. Map the returned band exactly to `SupportedBand`. Derive `TieType::Direct/GroupOnly` and reciprocity from the returned counts, not from a second local tally.
5. Keep evidence selection and persistence in Goal 1. The scorer receives no store and no evidence UUIDs.

This preserves linear work. It also prevents a “temporary” local 3/10/3 implementation from becoming a permanent second source of truth.

### 1.3 One `as_of` per rebuild

Materialize the interaction observations once. Before scoring any peer, calculate:

```text
rebuild_as_of = max(occurred_at_unix across every interaction row in this rebuild)
```

The maximum is computed before skipping an observation whose peer contact is unresolved: it is a store-level evidence time, not a peer-level time. Pass that same integer to every `score` call. For an empty interaction store no edge is scored; an explicit-as-of test/API may still be provided, but no path may read wall time.

Add an entry point that makes the discipline visible, for example:

```text
rebuild(store)                    # derives the one store maximum
rebuild_at(store, as_of_unix)     # deterministic explicit value for replay/tests
```

`rebuild_at` must still use its one argument for every peer. Never derive `as_of` inside a per-peer loop or from `tally.last_contact`.

### 1.4 Persist the scorer's direct/group split

Extend Goal 1 `TieStrength` (and its serialized free-form relationship object) with at least:

- `direct_out_count`, `direct_in_count`;
- `group_out_count`, `group_in_count`;
- `direct_active_day_count`;
- `last_direct_contact_utc` as optional/absent when no direct row exists;
- `silent_days`, `as_of_utc`, and `algorithm_id`.

Retain the existing combined totals for compatibility, but construct them from the same `soul_algo_tie::TieScore`. Assert:

```text
interaction_count = direct_out + direct_in + group_out + group_in
outgoing_count = direct_out + group_out
incoming_count = direct_in + group_in
```

`last_contact_utc` remains the newest row in any venue. That is the only clock used by the frozen recency rule.

### 1.5 A2 must replace, not join, `soul-draft`'s table

`soul-draft/src/analysis.rs::points_for` is currently a second renderer. Replace its count/direction/venue/recency/filing construction with `soul_algo_trait::a2_render`; do not append A2 bullets to the old five points.

Required adapter work:

- Promote A2 count fields from `u32` to `u64`, matching graph/tie counts, so a large import cannot truncate.
- Make A2 evidence identifiers generic (`TieScore<Id>`, `SummaryBullet<Id>`, `Id: Clone + Eq`) or otherwise opaque to the renderer. Then Goal 1 can pass UUIDs without adding `uuid` to the pure algorithm crate.
- Map the persisted effective `TieStrength.band` to A2's band and pass the already-computed direct/group counts, `last_contact`, global `as_of`, and evidence IDs.
- Preserve Goal 1's existing evidence-resolution check and non-clinical check around A2 output.
- `phrase_with` may continue to rephrase canonical bullets, but it may not change their facts, bands, or evidence.

A2 must not query the evidence store to reconstruct venue counts, and neither `soul-draft` nor A2 may define 3/10/3. Add a source-level guard over both renderer files for `STRONG_MIN`, `MODERATE_MIN`, `MIN_INTERACTIONS`, and literal 3/10 comparisons.

There is one existing constant drift to fix during wiring: `soul-algo-trait/src/a2.rs` currently defines `DORMANT_AFTER_DAYS: i64 = 180`. The decision says this must be the same symbol/value as tie demotion, so export an alias of `soul_algo_tie::constants::DEMOTE_ONE_BAND_DAYS`; do not retain a second `180` literal.

## 2. A0 intake adoption

The defect is specifically in `soul-profile::service::intake`: it records the answer and then calls `place_axis(..., locked_by_user: None)` without checking the standing lock. `record_axis_inference` does check the lock, so the two write paths disagree.

Adopt `apply_intake` semantics:

1. Continue recording every nonblank answer as an event and evidence row. A lock rejects the state transition, not the historical fact that the user answered.
2. Load the standing profile after recording.
3. For each staged axis answer, check the lock on that axis before `place_axis`.
4. If locked, leave position, band, cited correction evidence, and `locked_by_user` unchanged. Record the answer in an `ignored` result with stable reason `axis_locked_by_user`.
5. Apply unlocked axes normally under frozen `LastWriteWins`. Voice and stated-value handling is unaffected.
6. A later explicit `correct_axis` remains allowed to move a locked axis; questionnaire and machine inference are not.

The production outcome should expose ignored evidence IDs/reasons, mirroring `IntakeReport`, so the skip is not silent.

Replay equality is an acceptance test, not an informal claim. Add an integration sequence:

```text
initial intake -> correct one axis -> intake again with answers for the
locked axis and another axis
```

Mirror the same ordered rows into `soul_algo_trait::apply_intake`/`a0_all_axes`, and assert equality of position, band, lock, and cited evidence for all five axes. Also assert:

- the second answer's event/evidence exists even when ignored;
- the corrected axis still cites only the correction;
- the other axis changes;
- replay of all rows equals the imperative result.

Use the frozen `LastWriteWins` mode. Do not silently adopt `NoDowngrade` or A1 while fixing the lock.

## 3. Importer garbage-in and dedup priority

### 3.1 Dedup is P0 for T4D

The proposed key is suitable provided `external_id` is canonical per source:

```text
sha256("soul.import.dedup.v1|source|external_id")
```

- Telegram already canonicalizes its external id as `chat_id:message_id`.
- `soul-import-v1` currently only requires a nonempty message `id`; its contract must state uniqueness within a source/account, or its canonical external id must include `conversation_id` to avoid false deduplication.
- Persist only the digest, never the raw platform id.

Add an atomic storage operation/index that claims this digest before sealing a body or writing event/evidence rows. A repeated claim skips the entire message and reports it as a duplicate. A pre-scan over graph evidence is not sufficient: owner-only messages may create no interaction evidence, and concurrent imports require a unique claim.

Why this is not P1: repeating a direct export doubles direct in/out counts. Three unique direct exchanges can become six, and repeated imports can reach ten; T4D then produces a different band from the same source history. Repeated group rows also change displayed counts and can move `last_contact`, which T4D deliberately uses for recency.

Acceptance: importing an identical file twice must leave event count, interaction evidence count, graph counts, `last_contact`, and band unchanged; the second receipt reports duplicates.

### 3.2 Group N:1 fan-out is still a T4D correctness issue

The current commit path treats one owner group message as an outgoing interaction with every person who ever spoke anywhere in that conversation. The export does not prove those historical speakers were present for that message.

T4D removes this multiplication from the count gates, but not from:

- group counts shown to the user;
- any-venue `last_contact`;
- the 180/360 recency decision.

Therefore the conservative v0.1 rule should be:

- incoming group message -> one group observation for its actual sender;
- owner group message -> store the event, but create no peer-specific interaction unless the source supplies an explicit contemporaneous recipient/member set;
- direct conversation -> continue producing the one actual peer observation.

Do not “compensate” with weights in the algorithm crate. Fix observation semantics at import.

## 4. Graph correction is P0 product-lock

In `build.rs`, an existing inference ID is reused but the whole inference is reconstructed with `user_verdict = Unreviewed`. This destroys `Accepted`, `Corrected`, or `Rejected` on the next import.

Minimum rebuild rule:

```text
new inference:       user_verdict = Unreviewed
existing inference:  user_verdict = existing.user_verdict
```

Preserve every existing verdict, not only `Corrected`.

That alone does not fulfill the product lock. A corrected effective band must also survive:

- store `machine_band` from the new T4D score;
- when unlocked, effective `band = machine_band`;
- when user-locked, update counts/timestamps/`machine_band` but keep effective `band = user_band`;
- keep correction evidence alongside the edge's interaction evidence;
- `release_tie` clears the lock/user band and immediately restores the current machine band.

A2 and graph UI consume the effective `band` and never reclassify it. The machine result can be shown separately as an explanation, but it cannot overwrite the user's correction.

This work may land in the same adoption series, but it cannot be downgraded to “after T4D.” The frozen scoring formula and the user-override contract answer different questions, and both are release gates.

## 5. Required integration tests

1. `lilei_12`: 12 reciprocal direct rows over six days -> Strong.
2. Group flood plus one direct row each way -> Weak, with exact direct/group split.
3. Group-only traffic -> Weak.
4. Direct reciprocal counts 2/3 and 9/10, plus direct active days 2/3.
5. Any-venue silence at 179/180/359/360 days -> unchanged/demoted/demoted/Weak using closed boundaries.
6. A dormant peer scored while another peer has newer data uses the newer store-wide `as_of`.
7. Same evidence under per-peer maximum is demonstrably different, preventing that regression.
8. Identical import twice changes nothing and reports duplicates.
9. One owner group message does not create N peer-specific outgoing observations without explicit recipients.
10. Corrected tie -> rebuild -> effective band and verdict survive while machine counts/band update.
11. Corrected axis -> intake again -> answer row exists, axis does not move, and replay equals stored state.
12. A2 displays direct/group counts and the supplied effective band; changing counts alone never changes a bullet's band.

## 6. Must not be done

- Do not pull `rusqlite`, SQLCipher, `soul-store`, Tauri, HTTP, or platform services into either algorithm crate. Conversion and persistence stay in Goal 1.
- Do not change `>= 180` or `>= 360` to open intervals. Exactly 180 demotes one band; exactly 360 is Weak.
- Do not derive `as_of` per peer. One explicit/store-maximum value is shared by the entire rebuild.
- Do not add a second 3/10/3 table in `soul-draft`, A2, UI, or graph glue.
- Do not let group traffic enter T4D's reciprocity/count/day gates; it may affect only reported group facts and the frozen any-venue recency clock after import semantics are corrected.
- Do not silently patch the known F04c behavior with a new recent-activity gate.
- Do not use deduplication or graph correction as a reason to alter T4D's frozen constants or reopen rejected algorithms.

## Final adoption order

1. Add pure-crate workspace dependencies and boundary adapters.
2. Fix import observation semantics and add atomic dedup, so the scorer receives unique observations.
3. Wire global `as_of`, T4D scoring, split persistence, and frozen-boundary tests.
4. Replace the old `soul-draft` renderer with A2 and single-source its dormancy constant.
5. Wire A0 lock-aware intake and replay-equality tests.
6. Complete graph correction/verdict preservation before release.

These can be separate commits, but all six are Goal 1 release blockers. “T4D is wired” is not a correctness claim while duplicate direct rows, synthetic group recency, leaky A0 intake, and rebuild-erased corrections remain.
