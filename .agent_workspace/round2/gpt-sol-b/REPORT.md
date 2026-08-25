MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 2 T3/T3R/T4 ablation report

## Outcome

The independent Rust 1.83 probe passes all 12 tests with zero dependencies and
no `SPEC_BUG`. It reimplements T3, T3R, and T4 without importing another
agent's implementation.

The Round 1 group-only failure is closed in all three arms:

| Fixture | T3 | T3R | T4 |
|---|---|---|---|
| Group-only 50 events / 10 UTC days | Moderate | Moderate | Moderate |
| Dormant 2019 direct tie, `as_of` 2026 | **Strong control** | Weak | Weak |
| Reciprocal 20-event afternoon burst | Moderate | Moderate | Moderate |
| Lilei 12 direct events / 6 UTC days | Strong | Strong | Strong |

The dormant T3 result is asserted explicitly. It is the intended no-recency
ablation contrast, not a failed probe.

## Reimplemented rules

- **T3:** Moderate requires reciprocity and at least 3 events. Strong requires
  reciprocity, at least 10 events, at least 3 distinct UTC days, and at least
  one direct event. Consequently, group-only evidence cannot exceed Moderate.
- **T3R:** Uses the same latches with integer milli-event weights:
  `1000` for age `<90d`, `500` for `<180d`, `250` for `<360d`, and `0`
  afterward. Expired events do not supply direction, direct-contact, or
  active-day latches. There is no floating-point path.
- **T4:** Classifies with T3, lowers one band when latest contact is strictly
  more than 180 days old, and caps at Weak from 360 days. Exactly 180 days
  remains unchanged; the boundary has a dedicated probe.

All ages are computed from caller-supplied `as_of_unix`. Normal fixtures use
their maximum event timestamp; the dormancy fixture deliberately supplies a
2026 reference point. Future-dated events clamp to age zero. UTC days use
Euclidean division, including negative Unix timestamps.

## Boundary and policy probes

- Reversing and rotating the input leaves every complete result unchanged.
- Pacific-local midnight around DST does not create an extra UTC active day.
- `i64::MIN` event times and `as_of < event` complete without overflow or
  panic.
- `TieScore` includes `any_direct`.
- A source inspection test rejects `body`, `text`, `content`, `message`,
  `payload`, or `prose` fields on `Interaction`.
- Explanations use observable counts, UTC days, reciprocity, venue, and age.
  Tests reject `诊断`, `抑郁`, `score`, `percentile`, and the broader Round 1
  denylist.
- T3R's bucket boundary probe type-checks the aggregate as `u64`, verifies
  `1000 + 500 + 250 + 250`, and rejects floating-point types/constants in the
  implementation source.

## Recommendation

T3 is dominated on dormant ties. T3R and T4 both remove that regression while
preserving every required positive and anti-burst case. Prefer **T4** for v0.1:
its one-step downgrade and 360-day cap are easier to audit and explain than
per-event bucket arithmetic. Retain T3R only if gradual event-level aging is a
product requirement.

Full command output is in `TEST_LOG.txt`: `12 passed; 0 failed`.
