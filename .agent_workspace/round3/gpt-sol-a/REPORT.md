MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 3 T4 versus T4D report

## Result

On the requested criterion, T4D dominates T4: it removes the heavy-group
false Strong without regressing F_LILEI.

- `F_HEAVY_GROUP_PLUS_TWO_DIRECTS`: T4 = Strong; T4D = Weak.
- `F_LILEI`: T4 = Strong; T4D = Strong.

The heavy fixture has 36 group events over three days and only two direct
events, one in each direction. T4's `any_direct` latch lets group volume and
days satisfy its Strong gates. T4D applies reciprocity, count, and day gates
only to direct events, so two direct events do not even satisfy its Moderate
count of three.

This is a scoped dominance result, not a claim that every band is preserved.
In particular, group-only `F_GROUP` changes from T4 Moderate to T4D Weak
because T4D uses direct counts for its Moderate gate too. All other required
fixtures retain their T4 band.

## Implemented rules

T4 uses all-venue reciprocity, counts, and active days, with `any_direct`
required for Strong. T4D requires direct reciprocity and:

- Strong: `direct_count >= 10` and `direct_days >= 3`.
- Moderate: `direct_count >= 3`.

Both use the latest interaction from any venue for dormancy: age `>= 180`
demotes one band and age `>= 360` forces Weak. Group-only traffic cannot be T4
Strong. One caller-supplied `as_of` is used for the full rebuild.

## Fixture matrix

| Fixture | T4 | T4D |
|---|---|---|
| F_LILEI | Strong | Strong |
| F_BURST | Moderate | Moderate |
| F_GROUP | Moderate | Weak |
| F_OLD | Weak | Weak |
| F_ONESIDE | Weak | Weak |
| F_DORMANT_200d | Moderate | Moderate |
| F_HEAVY_GROUP_PLUS_TWO_DIRECTS | Strong | Weak |

Full counts are in `MATRIX.md` and `ABLATION.json`.

## Performance

Rust 1.83 release build, 10,000 interactions over 200 peers, 30 warmups per
policy and 500 timed passes in alternating-order chunks:

| Policy | ns/full pass | ns/interaction |
|---|---:|---:|
| T0 | 691,442 | 69 |
| T4D | 691,865 | 69 |

T4D measured `1.001x` T0. This allocation-inclusive microbenchmark is
machine-specific.

## Verification

All 11 tests pass. They pin the complete matrix, the closed 180/360 dormancy
boundaries, direct-only T4D reciprocity/count/day gates, any-venue last
contact, group-only behavior, one-sided traffic, caller-supplied `as_of`, and
the exact 10,000-event scale fixture. Formatting and strict Clippy checks also
pass. The implementation has no dependencies.
