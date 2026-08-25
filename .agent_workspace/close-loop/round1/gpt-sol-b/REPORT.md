gpt-5.6-sol-xhigh-fast
# Round 1 probe report

Probe only. No product crate or workflow implementation was made.

## 1. CI branch inventory versus the unique trunk

The unique Goal 1 merge trunk is `cursor/goal1-unblock-a073`.

At the requested trunk snapshot (`6133307`), `.github/workflows/ci.yml` named:

- `main`: push filter plus all five job gates.
- `cursor/soul-goal1-7b1c`: push filter plus all five job gates.
- `cursor/goal1-unblock-a073`: absent.

That confirms the seeded CI-TRUNK mismatch. While this probe was running, the
workflow owner landed `f8fe32f`. The current stacked branch now names every
active branch in each relevant location:

- `main`
- `cursor/goal1-unblock-a073` — the unique trunk
- `cursor/soul-goal1-7b1c` — explicitly retained as a historical trigger, not
  a merge path

Each appears once under `push.branches` and once in each of the five job
conditions. The unique trunk is therefore present in all six active branch
checks on the current close-loop branch.

## 2. R-1 reproduced

The isolated Rust probe is under `probe/`. It deserializes the legacy
eight-field JSON through the real `soul_graph::TieStrength` type, confirms
Serde defaulted `algorithm_id` to `""`, serializes the model through
`serde_json::to_value`, and validates a containing relationship with the
repository's own `SchemaSet`.

Serialized `TieStrength`:

```json
{
  "active_day_count": 2,
  "algorithm_id": "",
  "band": "moderate",
  "conversation_count": 1,
  "direct_active_day_count": 0,
  "direct_in_count": 0,
  "direct_out_count": 0,
  "first_contact_utc": "2026-08-01T00:00:00Z",
  "group_in_count": 0,
  "group_out_count": 0,
  "incoming_count": 2,
  "interaction_count": 4,
  "last_contact_utc": "2026-08-02T00:00:00Z",
  "outgoing_count": 2,
  "silent_days": 0
}
```

Schema result:

```text
relationship schema valid: false
relationship.schema.json rejected the instance:
  - "" is not one of ["T4D","T4"] at /tie_strength/algorithm_id
  - "as_of_utc" is a required property at /tie_strength
```

R-1 is confirmed. The empty string is serialized because `algorithm_id` has
`#[serde(default)]` but is not skipped when empty. It violates the frozen
`["T4D", "T4"]` enum. Its presence also activates the schema's complete
algorithm surface and exposes the legacy row's absent `as_of_utc`.

## 3. Second-threshold literal audit

A focused search for the five frozen constant names or comparisons against
`3`, `10`, `180`, or `360` found no matches in non-test
`soul-graph/src` or `soul-draft/src`. No raw `180` or `360` token exists in
either scope.

The broader raw-token search found only these non-threshold hits:

- `soul-graph/src/correct.rs:4`: documentation says “constraint 10”.
- `soul-graph/src/t4d_adapt.rs:97,135,163,169,182`: RFC 3339 byte positions,
  decimal radix, calendar months, and civil-date arithmetic.
- `soul-draft/src/a2_adapt.rs:155,192,219,225,238`: the same timestamp/calendar
  parser mechanics.

`soul-graph/src/t4d_adapt.rs:277` is inside `#[cfg(test)]` and was excluded as
requested. There are no second copies of the tie thresholds in the audited
product code.

## 4. GitHub merge capability

Confirmed unavailable from this VM:

- `gh` resolves to the Cloud Agent wrapper at `/exec-daemon/gh`; the runtime
  grants it read-only GitHub access.
- The available tool catalog has no `ManagePullRequest`, pull-request merge,
  or merge action.
- `gh pr merge --help` only exposes upstream static help; invoking a write was
  neither authorized nor attempted.

This agent cannot merge a GitHub pull request.
