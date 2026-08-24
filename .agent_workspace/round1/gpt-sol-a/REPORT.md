MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 1 T0 harness report

The independent Rust 1.83 harness reproduces Goal 1's T0 rule exactly:

- `Strong`: reciprocal, interaction count at least 10, and at least 3 distinct
  UTC active days.
- `Moderate`: reciprocal and interaction count at least 3, unless already
  strong.
- `Weak`: every other case.

The scorer deliberately does not use `now_utc`, age, or direct/group venue to
choose a band because Goal 1 does not use them.

## Deterministic fixture results

| Fixture | Interactions | Expected T0 band | Why |
|---|---:|---|---|
| F_LILEI | 12 | Strong | Reciprocal direct contact over 6 active days |
| F_BURST | 20 | Moderate | Reciprocal and frequent, but only 1 active day |
| F_OLD | 12 | Strong | Meets count/day gates; Jan 2019 age is ignored |
| F_GROUP | 50 | Strong | Meets count/day gates; group-only venue is ignored |
| F_ONESIDE | 100 | Weak | No incoming interaction, so reciprocity fails |
| F_EMPTY | 0 | Weak | No interaction or reciprocity |
| F_SCALE | 10,000 | Strong | Each of 200 peers has 50 reciprocal interactions over 10 days; all 200 score Strong |

`fixtures/cases.jsonl` contains one machine-readable metadata record per row.

## Checks and timing

All 6 tests pass on Rust 1.83.0. They cover determinism, both strong-band safety
properties, monotonic interaction count, peer isolation, and fixture
expectations. The captured output is in `TEST_LOG.txt`.

The release microbenchmark scored all 10,000 F_SCALE interactions into 200 peer
summaries 200 times:

- 259.538 ms total
- 1,297,688 ns per complete F_SCALE scoring operation
- 129 ns per input interaction (amortized)

This machine-specific timing includes fresh aggregation and allocation per
pass, but excludes fixture construction and file I/O.

## Brittleness assessment

Goal 1's constants look brittle as relationship semantics, though they are
simple and testable as a baseline. The values 3, 10, and 3 create sharp,
uncalibrated cliffs: otherwise identical evidence can change band on one
message or one UTC-date boundary. More importantly, F_OLD remains Strong after
more than seven years and F_GROUP becomes Strong from group-only traffic.
Those outcomes expose omitted recency and venue dimensions rather than a
coding defect. F_BURST confirms that the 3-day strong gate usefully blocks a
single-session burst from becoming Strong, while still labeling that burst
Moderate. Reciprocity correctly keeps F_ONESIDE Weak regardless of volume.

No T1, T2, or T3 winner is claimed in this report.
