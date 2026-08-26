MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 1 boundary probe report

Scope: current checkout `cursor/polish-project-plan-5280`. The v0.1 core plan
surface used for the keyword/negative probes is `README.md` plus
`docs/{PRODUCT_LOCK,DECISIONS,FORMAL_WORK_PROMPT,SECURITY,STATUS}.md`.
`docs/algorithms/*.md` is reported separately as the algorithm authority.
The non-authoritative `.agent_workspace/context/plan/*` snapshot was also
scanned for the requested terms.

## 1. AC ↔ 13-item PRODUCT_LOCK coverage

Extraction found AC-01 through AC-26 in the Goal 1 matrix, plus AC-27 in the
deferred sentence, and exactly 13 numbered v0.1 slice items.

| Slice | Backing ACs | Assessment |
|---|---|---|
| 1 | AC-01, AC-02 | Full |
| 2 | AC-03, AC-04, AC-05, AC-25 | Full |
| 3 | AC-03, AC-06, AC-08 | Partial |
| 4 | AC-07 | Partial |
| 5 | AC-14, AC-15 | Full |
| 6 | AC-16, AC-17 | Partial |
| 7 | AC-09, AC-10 | Full |
| 8 | AC-07, AC-11, AC-12, AC-13, AC-17, AC-25 | Full |
| 9 | AC-18, AC-19, AC-25 | Full |
| 10 | AC-20 | Full |
| 11 | AC-11, AC-17, AC-21, AC-22 | Full |
| 12 | AC-14, AC-15, AC-23, AC-24 | Partial |
| 13 | AC-01, AC-26 | Full |

No v0.1 slice item has zero ACs, and no AC-01–AC-26 matrix row has zero slice
mapping. AC-27 is outside the 13-item slice by design: FORMAL labels it
v0.1.1, so it is not a v0.1 orphan defect.

The four partial assessments are semantic gaps inside compound slice items:

- Slice 3: the ACs prove a profile, evidence-linked inference, and a graph, but
  do not explicitly test editing axes/preferences/boundaries, the evidence-band
  field, or T4D direct/group/as_of behavior.
- Slice 4: AC-07 tests a changed tone and non-overwrite, but not A0 axis
  correction followed by questionnaire intake/replay. No AC names graph
  correction or `correct_tie`.
- Slice 6: AC-16/17 test evidence, no diagnosis, and no-key fallback, but do not
  pin A2 as a pure T4D-band renderer or reject a second threshold set.
- Slice 12: AC-23 broadly says “after matrix actions”; it does not enumerate the
  ten PRODUCT_LOCK audit action kinds as ten expected audit records.

The complete bidirectional matrix and extracted source text are in
`COVERAGE.json`.

## 2. Algorithm-freeze keyword scan

None of the requested terms appears in any core v0.1 plan file, and none
appears in `.agent_workspace/context/plan/*`.

| Term | Current `docs/` matches |
|---|---|
| `T4D` | `algorithms/DECISION.md`, `REJECTED.md`, `R3-SYNTHESIS.md`, `README.md`, `COPY_ZH.md` |
| `T0` | `algorithms/DECISION.md`, `REJECTED.md`, `R1-SYNTHESIS.md` |
| `as_of` | `algorithms/DECISION.md`, `REJECTED.md`, `COPY_ZH.md` |
| `DEMOTE_ONE_BAND_DAYS` | `algorithms/DECISION.md`, `COPY_ZH.md` |
| `soul-algo-tie` | `algorithms/README.md` only |
| `correct_tie` | None |
| literal `intake lock` | None |
| semantic intake+lock | `algorithms/DECISION.md` line 40 (`intake 不绕锁`) |

Boundary result: PRODUCT_LOCK slice 3 and AC-08 cover a generic evidence-backed
graph, but the core plan and Goal 1 AC matrix do not absorb the frozen T4D
choice, direct-vs-group gates, one-global-`as_of` discipline, 180-day closed
boundary, or the T0 replacement obligation. A0's intake-lock behavior likewise
exists only in the algorithm authority, not in a core-plan AC. This is coverage
drift between `ALGO_FROZEN` and `PLAN_FROZEN`, not a contradiction with the 13
product items.

There is also a naming boundary: the checked-in crate is
`crates/soul-algo-tie`; only `docs/algorithms/README.md` uses that exact name,
while `docs/algorithms/DECISION.md` describes the post-merge dependency as
`crates/soul-algo`.

## 3. `relationship.schema.json`

`tie_strength` is a free object:

- The root requires only `schema_version`, `relationship_id`,
  `from_contact_id`, `to_contact_id`, and `evidence_ids`.
- `tie_strength` is optional.
- Its entire schema is `{ "type": "object" }`: no properties, required list,
  enum, or `additionalProperties: false`.
- With `jsonschema` 4.26.0, a relationship without `tie_strength`, with
  `tie_strength: {}`, and with arbitrary score/percentile/algorithm fields all
  validate.

DECISION requires the scoring output to carry band, raw counts, direct/group
split, last contact, silent days, and `as_of`; none is guaranteed by this
schema. DECISION's 3/10/3, 180/360, and dormant-note constants belong in the
algorithm's single constant definition rather than serialized relationship
fields. The schema gap is therefore the unconstrained result shape, not the
absence of duplicated numeric constants.

## 4. `profile.schema.json` versus A0

Aligned:

- No `score` or `percentile` property is declared.
- A trait-axis item has `additionalProperties: false`, so those keys are
  rejected directly on an axis.
- `position` is exactly
  `leans_low | mixed | leans_high | unknown`, matching A0's wire enum.
- Numeric position values are rejected.

Loose:

- Axis `locked_by_user` and `evidence_ids` exist but are optional; the focused
  validator probe accepts an axis with neither.
- `voice` is a free object, while `values` and `boundaries` are arrays without
  item schemas. Consequently the schema does not globally prohibit nested
  score/percentile keys; `voice: {"score": 88}` validates.

Thus the trait-axis direction vocabulary is aligned with A0, but the schema
does not fully encode A0 lock/evidence state.

## 5. Negative scope

Confirmed across the core plan surface:

- No v0.1 file-write execution requirement. It is denied in PRODUCT_LOCK and
  FORMAL, not consumed by SECURITY, and deferred to v0.1.1 in DECISIONS.
- No v0.1 OAuth requirement. It is explicitly absent/deferred to v0.2.
- No v0.1 E0 HTTP requirement. E0 has no code path/client; the cloud HTTP
  adapter is deferred to v0.4. User-directed E1 remains a separate allowed
  class and is already tested by AC-11/12/13/17.

Mentions of these features are prohibitions, deferred roadmap entries, or
denial tests, not positive v0.1 implementation obligations.

## 6. Validation

All 11 JSON files under `docs/schemas/` parse. Python `jsonschema` 4.26.0 was
available; Draft 2020-12 validator selection, `check_schema`, and validator
construction succeeded for all 11. Focused looseness probes produced the
results above. Exact output is in `SCHEMA_RESULTS.json`; commands and outcomes
are in `TEST_LOG.txt`.
