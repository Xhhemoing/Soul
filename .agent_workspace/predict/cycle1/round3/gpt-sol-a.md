MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Minimal later-test harness

Each test case is independent: create a fresh temporary store, granted consent, `FakeForegroundSource`, collector, and empty counters.

1. Drive only the real collection seam: `switch_to(app)` → `poll_once(t)` for each sample, then `finish(t)`; never insert event rows directly.
2. Read completed foreground events in their returned session-end order.
3. For every event, require `body_ref`, call `store.open(body_ref)`, and decode `SealedSessionBody`; take `app` from that unsealed body, not plaintext columns.
4. Build `apps`; compute `marginal = Counter(apps)` and `transitions = Counter(zip(apps, apps[1:]))`.
5. Assert exact integer counts and that a new case starts from zero. Never join separate cases, gaps, or traces into an edge.
