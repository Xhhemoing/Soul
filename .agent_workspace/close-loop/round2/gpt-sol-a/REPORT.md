gpt-5.6-sol-xhigh-fast
# Round 2 probe report

Tested HEAD: `ee4337b2c94b57712bc1db50bab3bf16f7ebb842` (`docs(close-loop): Round 1 synthesis`).

The Round 2 Opus CODE-2/CODE-3 tests were not present at this HEAD. The existing product-boundary tests still have the strict gaps recorded in Round 1:

- CODE-2: no `soul-graph` product test imports the named `group_heavy_plus_three_directs` fixture, checks its `direct_active_day_count`, pairs its Moderate result with the named anchor, and proves exact shared `as_of_utc` on both edges.
- CODE-3: `an_owner_group_message_does_not_write_one_outgoing_row_per_speaker` proves zero outgoing observations/counts, but does not assert that the owner's later group message leaves each historical speaker's `last_contact_utc` unchanged.

Both requested checks pass:

- `cargo test -p soul-graph -p soul-import -p soul-schema --offline`: 109 tests passed, 0 failed; all three doc-test targets passed with 0 tests.
- `cargo run -p xtask -- schema-freeze --check`: `docs/schemas` matches the lock.

No product code, Goal 2 work, or workspace-root `Cargo.lock` was changed.
