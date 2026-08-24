MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 1 boundary/adversarial report

## Outcome

The dependency-free Rust 1.83 probe independently reproduces T0 and the A0
correction lock. Fourteen of fifteen tests pass. The sole failure is
intentional and confirms a product defect: 10 days × 20 reciprocal group
events produce `Strong`, even though the normative probe rejects calling a
group-only tie strong.

This is a **PRODUCT RISK**, not a Rust correctness failure. T0 uses venue only
as an edge label; venue is absent from its band decision. A Telegram group
acquaintance, reciprocal bot traffic, or a duplicate import can inflate a band.

## Inputs checked

- `.agent_workspace/SHARED_BRIEF.md`
- `.agent_workspace/context/plan/PRODUCT_LOCK.md`
- `.agent_workspace/context/impl/graph_build.rs`
- `.agent_workspace/context/impl/profile_service.rs`
- `.agent_workspace/context/impl/redactor.rs`

The probe has no external dependencies and does not import production crates.
Its `Interaction` is metadata-only. A compile-time-included source inspection
greps that struct's field names for `body`, `text`, and `content`.

## Probe results

| Probe | Expected against independent rules | Observed | Finding |
|---|---:|---:|---|
| P_NO_TEXT | PASS | PASS | T0 accepts IDs, direction, venue, and Unix time only. |
| P_NO_NAME | PASS | PASS | Explanation accepts a peer ID, not a display name. |
| P_NO_CLINICAL | PASS | PASS | All three band explanations avoid `抑郁 焦虑 障碍 诊断 人格 量表 百分位 score percentile`. |
| P_DST | PASS | PASS | Two instants split across Pacific dates around the 2024 DST transition remain one UTC day. |
| P_LEAP | PASS | PASS | Two events on 2024-02-29 count as one active day. |
| P_ORDER | PASS | PASS | Reversed/rotated input yields the identical complete result and Strong band. |
| P_SELF | PASS | PASS | Peer ID `0` tallies without panic. Self-loop rejection remains a graph-layer policy. |
| P_U64_MAX | PASS | PASS | `conversation_id` and `peer_id` at `u64::MAX` are preserved. |
| P_NEGATIVE_TIME | PASS | PASS | Euclidean UTC floors support pre-1970 time; `-86399` and `-1` share UTC day `-1`. |
| P_NOW_BEFORE_EVENT | PASS | PASS | Results are identical before/after the event because T0 ignores `now`. |
| P_ALL_GROUP_STRONG_ATTEMPT | **FAIL expected** | **FAIL** | Faithful T0 returns Strong for 200 reciprocal group-only events over 10 days. |
| P_LOCK | PASS | PASS | A later conflicting Strong proposal returns `RefusedAxisLocked` and changes no axis field. |
| P_FORGET | PASS | PASS | Removing evidence 4–10 downgrades Strong to Moderate; forgotten IDs are absent from support. |
| Duplicate-import attack | PASS (demonstrates risk) | PASS | Reimporting two reciprocal events under new evidence IDs upgrades Weak to Moderate. |
| Forgotten A0 proposal | PASS | PASS | A proposal with no live evidence is refused and leaves the axis unchanged. |

Full output is in `TEST_LOG.txt`: `14 passed; 1 failed`, exit status 101 due
only to the intentional P_ALL_GROUP_STRONG_ATTEMPT assertion.

## Boundary observations

1. `graph_build.rs::Tally::band()` ignores `any_direct`; `any_direct` affects
   only `TieType`. The group-only Strong result is therefore faithful to T0.
2. Production day counting takes the first 10 timestamp characters. It is UTC
   correct only if every importer enforces the documented normalized `Z`
   invariant. The independent probe instead floors Unix seconds with
   `div_euclid(86400)`, including negative values.
3. Graph rebuild is edge-idempotent, but the shown code does not deduplicate
   observations before incrementing counts. Evidence IDs are deduplicated for
   citations only. Import-level stable event identity is required.
4. The shown rebuild writes peers that still have tallies but does not visibly
   prune an old edge when all supporting evidence disappears. The broader
   forgetting/orphan pipeline must prove that stale edge and inference bands
   cannot remain user-visible.
5. A0 correctly stores a later inference but refuses to apply it when
   `locked_by_user` is true. The probe focuses on the required profile-state
   invariant and also rejects forgotten-only support.
6. No T0 explanation needs names or prose. This is compatible with the
   third-party placeholder boundary in `redactor.rs`, but relationship
   explanations should remain local-only because IDs and contact timing are
   still third-party metadata.

## Recommendation

Do not ship T0 unchanged as a semantic “strong relationship” rule. For v0.1,
deduplicate source events and calculate bands from unique direct interactions;
keep group-only traffic Weak and display it separately. Preserve the existing
reciprocity/count/UTC-day thresholds so the result remains deterministic and
auditable. See `ATTACK_SURFACE.md` for the exact rule and residual bot risk.
