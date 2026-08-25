MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Foreground collection probe for later next-app fixtures

## Stored shape

- **Event fields:** the frozen `SoulEvent` has **9 possible top-level fields**, of which **7 are required**: `schema_version`, `event_id`, `ts`, `source`, `kind`, `actor_subject`, `consent_id`, `privacy`, and `body_ref`. See `docs/schemas/event.schema.json:6-47` and the matching Rust struct at `crates/soul-schema/src/event.rs:42-56`.
  - A collector-produced foreground event serializes **8 top-level fields** in the current implementation: all of the above except `consent_id`, because the collector sets it to `None` and serde omits it; `body_ref` is present. The fixed values are `source = collector.foreground_app`, `kind = app.foreground`, and owner subjects. See `crates/soul-collect/src/collector.rs:241-255`.
  - The SQL table has **8 physical columns** (`event_id`, `ts`, `source`, `kind`, `actor_subject`, `privacy_subject`, `body_blob_id`, `doc`); `doc` retains the complete schema object. This is storage indexing, not eight independent model features. See `crates/soul-store/src/sql.rs:46-56` and `crates/soul-store/src/store.rs:361-377`.
- **Session seal fields:** the decrypted `SealedSessionBody` has exactly **2 fields**, `app` and `duration_ms`; `deny_unknown_fields` rejects additions. See `crates/soul-collect/src/session.rs:56-79`. The in-memory completed session temporarily has `app`, `started_at_unix_millis`, and `ended_at_unix_millis`, but only app and computed duration enter the sealed body (`crates/soul-collect/src/session.rs:23-53`).
  - Separately, the `body_ref` seal pointer supports 7 metadata fields (`content_key_id`, `blob_id`, `alg`, optional `aad`, `subject`, `char_count`, optional `placeholder`), not behavioral features (`crates/soul-schema/src/common.rs:240-252`). Current collection emits six because `aad` is set and `placeholder` is absent (`crates/soul-store/src/store.rs:909-917`).

## Time and existing aggregates

- There is **no dedicated hour-of-day field**. The event `ts` is the session's start instant, converted to an RFC 3339 UTC string at whole-second precision (`crates/soul-collect/src/collector.rs:241-247`, `crates/soul-policy/src/clock.rs:31-40`). Thus UTC hour is derivable from `ts`; local/civil hour is not recoverable because no timezone or UTC-offset-at-event is stored. The body has duration but no explicit end timestamp; an end instant can be reconstructed as start plus duration.
- There is **no existing per-app histogram or transition matrix**. The nearest aggregate is the research preview's SQL hour-bucket rollup: `count(*)` grouped by event `kind`, UTC date-hour, and privacy subject (`crates/soul-store/src/research_preview.rs:56-72`). It does not open `body_ref`, so all foreground executables in an hour collapse into one `app.foreground` count and durations are unavailable. Its output fields are `event_kind`, `time_bucket_utc`, and `aggregate_count` (`crates/soul-store/src/research_preview.rs:88-99`); the test demonstrates two same-hour events becoming count 2 at `crates/soul-store/tests/research_preview.rs:156-164`. This is a dated hourly rollup, not a recurring 24-bin hour-of-day histogram.

## What a later Markov fixture needs

1. One or more **separate ordered traces** of valid executable names, such as `["code.exe", "chrome.exe", "code.exe"]`, plus deterministic millisecond timestamps. Keep traces separate so a transition is never invented across a fixture/session boundary.
2. Granted collection consent, a cloned `FakeForegroundSource`, a collector, and a readable test store. Drive each step with `switch_to(app)` followed by `poll_once(timestamp)`, then call `finish(final_timestamp)` so the last app is emitted. The established pattern is at `crates/soul-collect/tests/consent_gate.rs:145-158`.
3. Read only `collector.foreground_app` events in insertion order. `SqlCipherStore::list_events` currently uses `ORDER BY rowid` (`crates/soul-store/src/store.rs:405-442`), and the existing acceptance test asserts that this is session-end order (`crates/soul-collect/tests/consent_gate.rs:160-183`).
4. Open each `body_ref` and parse `SealedSessionBody` to obtain the executable sequence; `app` is encrypted and cannot be learned from the event columns or research rollup. Existing fixture code does this at `crates/soul-collect/tests/consent_gate.rs:167-175`.
5. Define sequence semantics explicitly:
   - repeated polls of the same app are coalesced into one session, so they do not naturally create `A -> A`;
   - `clear()` ends a session and creates a gap; decide that transitions do not bridge such gaps unless that behavior is intentionally under test;
   - source errors and consent revocation should terminate/drop training spans rather than silently join their neighboring apps;
   - decide whether transition counts are session-weighted (one edge per switch) or duration-weighted. Ordinary first-order next-app Markov should start with one edge per adjacent completed session.
   These behaviors follow the collector state machine at `crates/soul-collect/src/collector.rs:127-181` and fake controls at `crates/soul-collect/src/fake.rs:47-66`.
6. Store expected **directed edge counts** and normalized probabilities, including repeated edges, cycles, ties, an unseen current app, and an app with no outgoing edge. Assert count totals before probabilities so normalization cannot conceal dropped transitions.
7. If the future model is conditioned on time, specify whether it means UTC hour (available from `ts`) or local hour. A local-hour fixture needs additional timezone/offset input not present in today's collection record; it must not relabel UTC hour as local hour.
8. Keep the fixture local/offline and treat any persisted transition table as `derived`/`aggregate`, consistent with `.agent_workspace/predict/CONSTRAINTS.md:7-21`; the raw executable sequence remains sealed owner data.

Minimal tally logic for the expected oracle:

```python
from collections import Counter

def next_exe_counts(fake_sequence: list[str]) -> Counter[tuple[str, str]]:
    # Input is the emitted session sequence, after same-app poll coalescing.
    return Counter(zip(fake_sequence, fake_sequence[1:]))

assert next_exe_counts(
    ["code.exe", "chrome.exe", "code.exe", "chrome.exe"]
) == {
    ("code.exe", "chrome.exe"): 2,
    ("chrome.exe", "code.exe"): 1,
}
```
