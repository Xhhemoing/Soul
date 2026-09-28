# Blockers Round X (extra poll)

MODEL_SLUG: cursor-grok-4.6-high (parent)
Date: 2026-08-24
Live tip still `2e72ddf`.

| Slot | Result |
|---|---|
| fable-a | Blocked by provider policy; no file. Other five slots cover the poll. |
| fable-b | FREEZE_OK. G1+ restores §4.3 (peer-authored recency). Rejects clock-filter-while-keeping-fake-rows. |
| opus-a | FREEZE_OK. No stale P0s. S3 P2 landed. |
| opus-b | FREEZE_OK on rejecting `>= 25`. Probe now also needed for APFS (`cfg(unix)` ≠ case-sensitive). |
| gpt-sol-a | FREEZE_OK. G1+ sufficient for band-changing fan-out. |
| gpt-sol-b | DELTA: keep fake rows, hide from clock only. |

**Parent:** Keep G1+ as deleting owner→historical Outgoing (fable-b/gpt-sol-a). gpt-sol-b's filter would mutate the frozen clock and leave Reciprocal on forged rows. Take opus-b APFS wording and opus-a S3 landed. Freeze stands.

No Round X+1.
