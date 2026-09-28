# Blockers Round 1 synthesis (BINDING draft for Round 2)

MODEL_SLUG: cursor-grok-4.6-high (parent)
Date: 2026-08-24

Slots: fable-a, fable-b, opus-a, opus-b, gpt-sol-a, gpt-sol-b.
User-facing freeze candidate: `docs/BLOCKERS.md` (`BLOCKERS_DRAFT`).

## Disagreements and how they were called

| Dispute | Winner | Why |
|---|---|---|
| PR #2 unrelated to main? | **Related.** merge-base = `ea6f62f` | opus-a measured a shallow graft (`fbad85b`). After unshallow, fable-a is correct. |
| PR #4 ancestor of current #2? | **No.** fork at `862e858` | fable-a / gpt-sol-b. PR #4 body is stale. |
| Canonical trunk | **goal1 (`cursor/soul-goal1-7b1c`)** | WP13, merged questionnaire, `/files` `/graph`, ipc `--no-run`. dev-sota re-implemented a subset. |
| AC-18 remaining red | **Fixture case-fold, not `\\?\` strip** | All four technical slots agree. opus-b's filesystem probe > fable-a's `cfg!(windows)` expected lists. |
| fail-fast hiding crates | **P0 companion to AC-18** | opus-a, opus-b. |
| DPAPI | **P0-ship / P1-merge** | gpt-sol-b DEFER is right for *merging honest fail-closed code*. opus-b is right that AC-04/AC-08 on a real Win11 box are unreachable until DPAPI exists. Collect crate already allows `unsafe` on Windows, so SECURITY.md's reason is stale. |
| Import fan-out | **P1 after T4D, not a banding P0** | T4D count gates ignore group rows (fable-b). gpt-sol-a is still right that fan-out pollutes `last_contact`; that is a clock-quality P1, not a reason to delay T4D wiring. |
| Dedup | **P0 with T4D** | gpt-sol-a. Re-import crosses 3/10. |
| Graph correction | **P0 product-lock** | fable-b, gpt-sol-a. gpt-sol-b omitted it from the hard set; PRODUCT_LOCK text is explicit. |
| Questionnaire UI | **P0-close, not P0-merge** | opus-a, fable-b. |
| AC-21 vacuous | **P1** | Honest `Unsupported` is already documented. Fail-loud is good, not merge-blocking. |
| Junction/reparse as the AC-18 failure | **Not the current red** | gpt-sol-b's security concern stands as author-manual / future Windows corpus; the two failing tests are the decoy collision. |

## Must not leak into Round 2 as new scope

F04c third gate, T0 revival, Goal 2, file-write execute, OAuth, E0, Linux desktop, WeChat scrape, merging PR #1 separately, rebasing 96 Goal 1 commits onto main, taking main's `Cargo.toml` (would delete the app workspace).

## Round 2 charge

Attack `docs/BLOCKERS.md`: missed P0, false P0, wrong fix that weakens AC-18, wrong integration order, STATUS over-claims still uncorrected. Do not implement product code. First line: your model slug.
