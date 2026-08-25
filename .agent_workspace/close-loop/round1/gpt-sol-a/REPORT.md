gpt-5.6-sol-xhigh-fast
# Round 1 probe report

Probe result: the requested package suite and schema-freeze check both pass. Strict AC mapping found one fully covered row (AC-31) and six rows with related passing tests but incomplete acceptance coverage (AC-28, AC-29, AC-30, AC-32, AC-33, AC-34). See `AC_MAP.md` for file:line, test names, and the exact missing assertion in each row.

Schema confirmation:

- `docs/schemas/relationship.schema.json:31-34` makes `algorithm_id` the closed enum `T4D` / `T4`, not a uuid7.
- `crates/soul-schema/tests/schema_wiring.rs:286-303` explicitly exempts `algorithm_id` from the entity-identifier uuid7 walk.

No product feature, Goal 2 work, frozen algorithm constant, schema, or source file was changed. Only this probe's three report files were added.
