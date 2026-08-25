MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round X independent T4D cross-check

The zero-dependency implementation is in `shadow/`. It implements the frozen
rule directly from `DECISION.md`: direct-only reciprocity/count/day gates,
followed by the any-venue recency clock and closed 180/360-day thresholds.

`compare/` is a separate package. From its actual location, the canonical path
dependency is `../../../../crates/soul-algo-tie` (one level more than the
suggested path); it compares bands and the numeric direct/group/day/silence
diagnostics.

| Fixture | Expected | Shadow | `soul_algo_tie` | Direct | Group | Direct days | Silent days | Result |
|---|---|---|---|---:|---:|---:|---:|---|
| F_LILEI | Strong | Strong | Strong | 12 | 0 | 6 | 3 | AGREE |
| F_HEAVY | Weak | Weak | Weak | 2 | 100 | 1 | 0 | AGREE |
| F_GROUP | Weak | Weak | Weak | 0 | 50 | 0 | 0 | AGREE |
| F_OLD | Weak | Weak | Weak | 12 | 0 | 6 | 2800 | AGREE |
| F_BURST | Moderate | Moderate | Moderate | 20 | 0 | 1 | 0 | AGREE |
| 179 | Strong | Strong | Strong | 12 | 0 | 6 | 179 | AGREE |
| 180 | Moderate | Moderate | Moderate | 12 | 0 | 6 | 180 | AGREE |
| 359 | Moderate | Moderate | Moderate | 12 | 0 | 6 | 359 | AGREE |
| 360 | Weak | Weak | Weak | 12 | 0 | 6 | 360 | AGREE |

## DISAGREE rows

None.

Verification:

- `cargo test --manifest-path shadow/Cargo.toml`: 3 passed.
- `cargo clippy --manifest-path shadow/Cargo.toml --all-targets -- -D warnings`: clean.
- `cargo run --manifest-path compare/Cargo.toml`: all nine rows agree.

VERDICT CONFIRM_FREEZE
