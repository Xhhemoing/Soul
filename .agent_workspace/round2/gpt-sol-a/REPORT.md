MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 2 independent ablation report

`ablate/` is a zero-dependency Rust 1.83 implementation written independently
from the supplied T3/T3R/T4 specification. It imports no Round 1 candidate.
Every scorer receives `as_of`; UTC dates are `unix.div_euclid(86400)`.

The fixture civil time `2026-08-24 14:00:00 UTC` is computed in source by a
Gregorian civil-to-Unix function as `1,787,580,000`. The value was independently
checked with GNU `date`.

## Required fixture matrix

| Fixture | T3 | T3R | T4 | T3R event milli | T3R day milli |
|---|---|---|---|---:|---:|
| F_LILEI | Strong | Strong | Strong | 48 | 24 |
| F_BURST | Moderate | Moderate | Moderate | 80 | 4 |
| F_OLD | Strong | Weak | Weak | 0 | 0 |
| F_GROUP | Moderate | Moderate | Moderate | 200 | 40 |
| F_ONESIDE | Weak | Weak | Weak | 400 | 80 |
| F_EMPTY | Weak | Weak | Weak | 0 | 0 |
| F_DORMANT_200d | Strong | Moderate | Moderate | 12 | 6 |

The one-sided fixture remains Weak despite a high weighted count because
reciprocity requires at least one incoming and one outgoing event. Group-only
traffic is capped at Moderate. The 200-day fixture is T3-Strong, while T3R's
quarter weights and T4's one-band demotion both make it Moderate.

## Net wins and decision

Under the stated stale-contact objective, T3R and T4 each net-win over T3 on
`F_OLD` and `F_DORMANT_200d`, with no regression among the other five required
fixtures. T3R has zero net-win fixtures over T4, and T4 has zero over T3R:
their bands tie on all seven fixtures.

No winner is selected. This independent matrix does not reproduce the
`R1-SYNTHESIS.md` comment that calls T3R the provisional winner. The required
fixtures provide no band-level evidence to break the T3R/T4 tie.

Two prose conflicts were resolved in favor of the direct Round 2 specification:

- R1 says to derive `as_of` from the maximum data timestamp, while this task
  requires caller-supplied `as_of` with the fixed fixture default.
- R1 describes T4's first cutoff as `>180d`, while this task requires
  `age >= 180`; tests pin ages 179, 180, 359, and 360.

## F_SCALE timing

Release build, 10,000 interactions over 200 peers, 200 scoring passes:

| Scorer | ns / full pass | ns / interaction |
|---|---:|---:|
| T0 | 574,658 | 57 |
| T3R | 628,526 | 62 |

T3R measured `1.094x` T0, or about 9.4% slower. The microbenchmark includes
fresh aggregation and allocation on every pass and excludes fixture creation
and I/O; the absolute timing is machine-specific.

## Verification

Rust 1.83.0 ran 12 tests successfully. Coverage includes the civil timestamp,
negative-Unix Euclidean day division, every weight and T4 boundary, the full
fixture matrix, reciprocity, group-only caps, all-zero dormancy, distinct-day
weighting, peer isolation, and all 10,000 F_SCALE inputs. Formatting and strict
Clippy checks also pass. Full command output is in `TEST_LOG.txt`; the
machine-readable matrix and net wins are in `ABLATION.json`.
