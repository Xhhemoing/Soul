# Agent progress

Parent exclusive branch: `cursor/soul-integration-4a8e`.
Base: `origin/cursor/soul-goal1-7b1c` @ `5309656` (unique Goal 1 trunk, D49).
Goal: complete Soul so it delivers (1) multi-dimensional interpersonal and self analysis, (2) work and social assistance, (3) simple behavior and group prediction, (4) data collection for continuous optimization.
Process: Fable (`claude-fable-5-thinking-xhigh`) scans/reviews; Opus (`claude-opus-5-thinking-high-fast`) lands code. Commit one purpose at a time. PR when there is a real artifact. Merge when safe; otherwise record `BLOCKED` and continue.

## Current round

**ROUND 1** — map the real tree, do not rewrite finished WPs.

| Field | Value |
|---|---|
| Current task | 10 Fable scanners covering frontend / backend / API / data / auth / graph / soul-layer / prediction / tests+CI / independent review |
| Parent model | product/account setting (not a Task slug) |
| Hosted CI | `BLOCKED` — workflow auto-push only on `main` and `cursor/soul-goal1-7b1c`; account Actions runners empty (billing/spending). Local `just ci` is the gate. |

## Coverage map (pre-scan, from trunk STATUS + remotes)

Must cover every row. "Done on trunk" is not "Goal closed" and not "user four requirements complete".

| Area | Trunk evidence @ `5309656` | Gap vs this Goal |
|---|---|---|
| Frontend | `apps/desktop` routes: wizard, profile, graph, memory, draft, import, collect, files, research, audit, settings | Honesty copy largely landed; T4D bands not on this tree |
| Backend | `soulcore` + Session IPC; WP01–WP11, WP13, DPAPI | Still T0 graph on this tree |
| API | Tauri IPC, 36 commands | No prediction commands |
| Database | Encrypted SQLCipher `%LOCALAPPDATA%\Soul\soul.db` | Schema freeze vs `main` (AC-28+) is a contract-tree conflict |
| Login & permissions | Local store keys via DPAPI; HITL tokens; no cloud login | v0.1 has no accounts — expected |
| Storage | SQLCipher + content keys + forget | No self-export-to-disk (v0.2, PRODUCT_LOCK cut) |
| Cache | None as a product surface | Confirm no accidental cache of third-party plaintext |
| Third-party | Telegram `result.json` + `soul-import-v1`; E1 user endpoint | No OAuth (cut). E1 has no Authorization header |
| Core business | Soul layer + agent read-only path | Graph still legacy T0; no behavior/group prediction product path |
| Tests | Workspace + desktop shell tests; AC-01–AC-26 product-side largely wired | AC-28…AC-34 not on this tree; hosted AC-26 empty runner |
| Build | Tauri 2 + pnpm + cargo + just | Full `tauri build` / NSIS is author-manual (D56) |
| CI/CD | `.github/workflows/ci.yml` | This branch not in auto-push list; hosted minutes `BLOCKED` |
| Performance | Not a close gate | Unscanned |
| Security | E0 none; redactor; injection; audit no body | Session-seam KnownIdentifiers residual (BLOCKERS S2) |
| Reliability | Crash harness, forget atomic, single-instance mutex | Author Win11 checklist open |

## Already known (do not re-open as new work)

- WP01–WP11, WP13, DPAPI: landed on unique trunk.
- `PLAN_FROZEN` + `ALGO_FROZEN` on `main`.
- Do not start Goal 2 empty polish loops before Goal 1 close (FORMAL). This Goal may still land the user's four requirements; that is not "Goal 2 twenty-round rename theater".
- Do not wholesale-merge `agent/dev-sota` (PR #4) — D49.
- Unique trunk STATUS forbids merging PR #7 as the merge path for *that* branch. This parent branch may still **port** T4D/A0 from `cursor/goal1-unblock-a073` @ `6133307` after Fable names a non-destructive plan.
- Hosted empty runner is billing, not a code task. Do not empty-commit.

## Open PRs this parent will not silently merge

| PR | Head | Notes | Merge now? |
|---|---|---|---|
| #2 | `cursor/soul-goal1-7b1c` | Unique trunk vs `main`: `CONFLICTING` (docs/schema lock) | No — contract tree |
| #7 | `cursor/goal1-unblock-a073` | T4D/A0/G1/G2/G3 landed there; behind unique trunk UI honesty | No — port, don't replace trunk |
| #6 | `cursor/blockers-analysis-a073` | `BLOCKERS.md`; D32 number clash vs DECISIONS | Later, with D32 rewrite |
| #4 | `agent/dev-sota` | Parallel implementation line | No — D49 stop |
| #9 | `cursor/predict-algo-survey-a073` | v0.2 survey, draft | Read-only input for prediction work |

## Findings this round

*(filled after the 10 Fable reports)*

## Fixes this round

None yet. Round 1 is scan/split.

## Tests this round

Not run yet (no code change).

## Commits / PRs

| When | What | SHA / PR | Merge |
|---|---|---|---|
| start | Create exclusive branch + this tracker | (this commit) | PR of this branch; merge to `main` `BLOCKED` same as #2 |

## Blocked

- Hosted Actions runners (billing/spending).
- Merge to `main` until unique trunk absorbs `main` plan freeze (M2) without destroying AC-28+ or product honesty.
- Author Win11 manual checklist (needs a human on Windows).

## Next round focus

After Fable round 1: pick one Opus owner for T4D port onto this branch (G1/G1+/G2/G3/A2), then other Opus owners for remaining product-path holes. Prediction stays v0.1-compatible: schema + local preview, no E0, no third-party export.
