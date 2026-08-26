MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 3 replay, negative-scope, keyword, and coverage report

## Verdict

All six requested probes pass at Git `c3d59602a4e718e1468fa321275a7432e69dc8b5`.

- `PRODUCT_LOCK` SHA-256 values still diverge: authoritative `docs/` is
  `50bbe63a…5ad1`; the historical context snapshot is `9fa335f0…5e5c7`.
  This is expected because `.agent_workspace/context/plan/README.md` marks that
  directory non-authoritative.
- The core plan surface does not require OAuth, E0 HTTP, or file-write
  execution in v0.1. Every occurrence is an exclusion, denial test, or later
  roadmap item. User-directed E1 is separate from E0.
- `T4D` occurs in both required files: twice in `PRODUCT_LOCK` and six times in
  `FORMAL`.
- FORMAL now has 33 v0.1 matrix rows: AC-01–26 and AC-28–34. AC-27 remains
  v0.1.1. Every v0.1 AC maps to at least one of the 13 slices, and every slice
  has at least one AC.
- AC-34 maps to slice 3, not slice 2: import is its stimulus, while the asserted
  behavior is graph edge and `last_contact` attribution.
- Root Cargo workspace membership remains exactly the two algorithm crates.
- Python parsed all 12 authoritative `docs/schemas/*.json` files and all 18
  schema JSON artifacts in the repository; no parse failures occurred.

## Coverage refresh

Coverage remains **11 full / 2 partial**:

- Slice 3 remains partial despite adding AC-34 because no matrix row explicitly
  edits and verifies both preferences and boundaries.
- Slice 12 remains partial because AC-23 does not enumerate all ten named audit
  action kinds as separate expected records.

These are pre-existing matrix-to-slice specificity gaps, not orphan mappings.
`COVERAGE.json` contains the complete bidirectional mapping and probe evidence.
