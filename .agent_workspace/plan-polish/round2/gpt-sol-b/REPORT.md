MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 2 relationship-schema and AC boundary report

## Verdict

The checked-in `docs/schemas/relationship.schema.json` still has the Round 1
blocking gap: `tie_strength` is only `{ "type": "object" }`. It accepts an
empty object and an arbitrary `score`. The Round 1 opus-b candidate closes
both negative boundaries, still permits the property to be omitted for legacy
compatibility, and accepts a complete T4D-shaped object.

Tested checked-in schema bytes:

- Git HEAD: `b380f675636eb30aede1c0513ba91c6fef01d66c`
- `git hash-object docs/schemas/relationship.schema.json`:
  `96003f4470b8e6c0797e6bb00ecb493b6972b3be`
- SHA-256:
  `df34747e28ef2e01ff02f094b26a2b7722c096256563474835f2c61f1ec122ff`

## Schema boundary matrix

Python `jsonschema` 4.26.0 selected Draft 2020-12; both schema definitions
passed `check_schema`, and `_defs.schema.json` refs resolved.

| Probe | Current docs schema | Round 1 candidate |
|---|---:|---:|
| Missing `tie_strength` | valid | valid |
| `tie_strength: {}` | **valid (gap)** | invalid |
| `tie_strength: {"score": 0.9}` | **valid (gap)** | invalid |
| Complete T4D-shaped object | valid | valid |

The candidate behavior matches the requested compatibility boundary: omission
remains legal, but once `tie_strength` is present it cannot be empty or carry
an undeclared score. Its T4D object requires the split counts, direct active
days, silent days, and `as_of_utc` whenever `algorithm_id` is present.

## AC ↔ 13-slice refresh

FORMAL now contains 32 Goal 1 matrix rows: AC-01–AC-26 and AC-28–AC-33.
AC-27 remains explicitly v0.1.1. All 32 Goal 1 rows map to at least one of the
13 PRODUCT_LOCK slices, and every slice has at least one AC.

Coverage improves from Round 1's 9 full / 4 partial to **11 full / 2
partial**:

- Slice 4 is now full: AC-31 covers A0 correction-lock intake/replay and AC-32
  covers locked relationship correction, complementing AC-07's immediate
  drafting change.
- Slice 6 is now full: AC-32 makes A2 consume the effective corrected band;
  AC-33 enforces split-field rendering, omission when absent, and no second
  threshold/recomputation.
- Slice 3 remains partial. AC-28–30 now cover T4D, direct/group fields, and
  one-global-`as_of`; AC-32 covers corrected effective band. No matrix row
  explicitly edits and verifies both preferences and boundaries.
- Slice 12 remains partial. AC-32 explicitly covers correction audit, but
  AC-23 still does not enumerate the slice's ten named action kinds as
  separate expected records.

`COVERAGE.json` contains the complete bidirectional mapping.

## Required negative and keyword probes

All negative scope checks pass:

- v0.1 requires read-only file planning, not file-write execution; AC-27 and
  execution/undo remain v0.1.1.
- OAuth is absent from v0.1 and deferred to v0.2.
- E0 has no v0.1 implementation, domain, HTTP client, or code path. E1 remains
  a separate user-directed path.

The Round 1 keyword obligation also passes:

- `T4D` appears twice in `docs/PRODUCT_LOCK.md`.
- `T4D` appears six times in `docs/FORMAL_WORK_PROMPT.md`.

Exact probe output and line evidence are in `TEST_LOG.txt`.

