MODEL_SLUG: gpt-5.6-sol-xhigh-fast
# Round 3 gpt-sol-b: T4/T4D adversarial probes

## Independent implementation

`probes/` is a dependency-free Rust 1.83 package. It implements:

- T4: the frozen 3/10/3 rule over all events, with a direct-event latch.
- T4D: the same thresholds over direct events only; direct reciprocity, direct
  active days, and direct recency are used for its band.
- Frozen recency: age `>= 180` demotes one band and age `>= 360` forces Weak.
- Caller-controlled `as_of`, integer UTC-day arithmetic, and no wall-clock read.

The input record has exactly three metadata fields: timestamp, direction, and
venue. It has no message-content field. The only output words are
`weak`, `moderate`, and `strong`.

## Probe results

| Probe | T4 | T4D | Result |
|---|---:|---:|---|
| Latest age 179 | Strong | Strong | closed lower side preserved |
| Latest age 180 | Moderate | Moderate | `>= 180`, not `> 180` |
| Latest age 359 | Moderate | Moderate | one-band demotion only |
| Latest age 360 | Weak | Weak | forced Weak |
| F_HEAVY: 100 group + 2 reciprocal direct | Strong | Weak | T4D blocks fanout unlock |
| lilei: 12 direct / 6 days | Strong | Strong | no anchor regression |
| group-only 50 | Moderate | Weak | neither can be Strong |
| dormant 2019, explicit 2026 `as_of` | Weak | Weak | 2,632-day dormancy honored |

Additional properties passed:

- Exact score equality under 64 deterministic shuffles.
- T4D cannot return Strong with `direct_count < 10`; the probe crosses direct
  counts 0–9 with group counts 0, 1, 12, and 100.
- The metadata-only input shape is compile-time exhaustive.
- The public result vocabulary is closed to the three neutral band labels.

## `as_of` trap

`trap_peer_local_fixture_max_wrongly_revives_f_old` deliberately calls T4 on
F_OLD without the 2026 cutoff. The fallback selects that peer fixture's latest
2019 event, computes age 0, and returns Strong. Passing the rebuild-wide
2026-08-24 cutoff computes age 2,632 and returns Weak. Therefore a caller must
select one `as_of` for the whole rebuild; it must not derive one per peer.

## Recommendation

**Recommend T4D as the single graph algorithm.**

T4 fails the new adversarial case: group fanout supplies 100 of its 102 events,
while only two direct greetings unlock Strong. T4D removes that failure, keeps
the accepted lilei relationship Strong, preserves the frozen recency
boundaries, and makes `Strong => direct_count >= 10` structural rather than a
convention.

The deliberate policy consequence is that group-only evidence is Weak under
T4D rather than T4's Moderate. Group totals remain available in the returned
audit counters; they simply do not determine the relationship band.

Verification: Rust 1.83.0, formatting clean, clippy clean with warnings denied,
13/13 integration tests passed.
