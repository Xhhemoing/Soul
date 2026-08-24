# T0 attack surface

The current T0 formula measures observed traffic, not emotional closeness:

`Strong = reciprocal AND interactions >= 10 AND UTC active days >= 3`

`graph_build.rs` records whether any event is direct, but venue does not affect
`Tally::band()`. That makes the following inputs gameable.

| Route | How it changes T0 | Product consequence |
|---|---|---|
| Telegram group spam | A user and peer alternate posts in one group over three or more UTC days. Group-only events satisfy every Strong threshold. | A group acquaintance can appear as a strong tie without any private exchange. |
| Reciprocal bot pings | A bot sends notices and the owner replies or triggers commands. Counts and reciprocity look human to T0. | Service traffic is promoted as a relationship. |
| Importing the same chat twice | If the importer gives duplicate source events new evidence IDs, graph rebuild idempotency updates the same edge but still counts every observation. | Two weak reciprocal events become four and cross Weak → Moderate; larger imports can cross Strong. |
| Timestamp manipulation | Imported timestamps can be spread over three UTC days even if the events were generated together. | The active-day gate can be forged unless source timestamps are validated. |
| Contact mis-linking | Importer identity resolution can merge two people into one peer ID. | Their directions, days, and counts combine into an artificial Strong edge. |
| High-volume direct automation | Direct-message bots or scripted reciprocal pings pass the same gates as people. | Requiring private traffic alone does not establish closeness. |

There is also an importer boundary risk around time zones. Production
`utc_date()` takes the first ten characters of a timestamp and is correct only
because writers promise normalized `Z` timestamps. An offset-bearing timestamp
that bypasses normalization would count a source-local date instead.

## Explainable v0.1 mitigation

Use a small T3-style rule instead of adding opaque weights:

1. Deduplicate at import with a stable source-event key. Prefer the platform's
   source message ID scoped by source account and conversation. Store a local
   hash of that key if raw identifiers should not persist.
2. Compute relationship bands from unique **direct** events only. Strong still
   requires both directions, at least 10 events, and at least 3 UTC days;
   Moderate still requires both directions and at least 3 events. Group-only
   traffic remains Weak and is shown separately as context.
3. Exclude bots/channels only when the official export supplies an explicit
   account type. Do not infer bot status from message prose.
4. Label the result “observed interaction band,” not closeness, and display the
   auditable counts: unique direct events, directions, UTC days, and ignored
   duplicate/group events.

This remains deterministic, O(n log n) with sets (or expected O(n) with hash
sets), local-only, prose-free, and explainable in one sentence. It closes the
demonstrated group and duplicate-import paths. Direct-message automation
remains a disclosed residual risk unless trustworthy source metadata identifies
the account as automated.
