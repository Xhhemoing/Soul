# Agent progress

Parent exclusive branch: `cursor/soul-integration-4a8e`.
Working base: `origin/cursor/first-test-candidate-c441` @ `85aae68` (strict FF of unique trunk `cursor/soul-goal1-7b1c` @ `5309656`, AD-8).
Goal: complete Soul so it delivers (1) multi-dimensional interpersonal and self analysis, (2) work and social assistance, (3) simple behavior and group prediction, (4) data collection for continuous optimization.
Process: Fable (`claude-fable-5-thinking-xhigh`) scans/reviews; Opus (`claude-opus-5-thinking-high-fast`) lands code. Commit one purpose at a time. PR when there is a real artifact. Merge when safe; otherwise record `BLOCKED` and continue.

## Current round

**ROUND 2** — land the T4D port and non-overlapping product-path fixes. Round 1 scan is closed.

| Field | Value |
|---|---|
| Current task | One Opus owner ports T4D/G1+/G2/G3/A2 from PR #7 onto this FF'd line; parallel Opus owners fix forget page-level CK destruction, wizard error shape, E1 plan-hash vs endpoint |
| Parent model | product/account setting (not a Task slug) |
| Hosted CI | `BLOCKED` — billing/spending; workflow auto-push still only `main` + `cursor/soul-goal1-7b1c` (not widened: would waste minutes on docs). Local `just ci` is the gate. |

## ROUND 1 record

| Field | Value |
|---|---|
| ROUND | 1 |
| 子代理任务 | 10× Fable scanners: frontend, backend, API/IPC, data, auth, graph/T4D, soul layer, prediction, tests/CI, independent review |
| 发现问题 | See findings below |
| 修复问题 | None in R1 (scan only). Parent FF'd exclusive branch onto c441 (git, not product code) |
| 测试结果 | Independent review ran vitest 14/161, `session_e1` 22/22, matrix replay, xtask e0/denylist/schema-freeze — green on the pre-FF tree; post-FF not re-run yet |
| Commit | tracker docs; rebase onto `85aae68` |
| PR | #15 |
| Merge状态 | `BLOCKED` to `main` (same contract-tree as #2). Merge of #7/#4 forbidden. Parent cannot merge PRs (no merge tool) |
| 下一轮重点 | Port T4D onto FF'd line; independent store/policy/IPC fixes; prediction thin slice waits for T4D |

## Coverage map (after Round 1 evidence)

| Area | Evidence | Gap vs this Goal |
|---|---|---|
| Frontend | 11 routes + wizard cover slices 1–12; copy honesty holds; cross-page state consistent | Graph still T0 words; no prediction surface; no tie-correction UI |
| Backend | Session is the one stateful module; WP01–WP11/WP13/DPAPI landed | T0 `Tally::band`; G1+ fanout; G2 intake moves locked axes; G3 no correct_tie |
| API | 36 commands, three-way pinned | `complete_wizard` error shape; no directory revoke; no predict command (thin slice needs none) |
| Database | SQLCipher + CK + forget atomic; no third-party plaintext cache | Forget leaves wrapped CK in free pages; schema `tie_strength` still loose vs main |
| Login & permissions | Local DPAPI key chain; no cloud login | Expected |
| Storage | As database | Same forget gap |
| Cache | None found | — |
| Third-party | Import + E1 redactor; KnownIdentifiers seam closed on this tree | E1 origin not in plan hash; name-prefix scrub nits |
| Core business | Soul + agent read-only | **T0 graph is the false-close trap**; no T4D projection sentences |
| Tests | AC-02–25 product-path on this tree | AC-28…34 absent until T4D port |
| Build | Tauri 2 + pnpm + cargo + just | NSIS author-manual |
| CI/CD | Auto-push main + unique trunk | Hosted empty runner `BLOCKED` |
| Performance | Unmeasured | NO_HIGH_VALUE_CHANGE_FOUND |
| Security | E0 type-absent; HITL for E1 | Forget bypasses capability tokens; four unused ConsentTopics |
| Reliability | Crash harnesses, one-store, single-instance | Author Win11 checklist open |

## Findings this round (R1)

1. **P0 G1:** this tree still bands with T0 3/10/3 in `crates/soul-graph/src/build.rs`. Frozen answer is T4D. Algo crates not in workspace.
2. **P0 G1+:** owner group messages still fan Outgoing to every historical speaker (`soul-import/src/commit.rs`).
3. **P0 G2:** `intake` calls `place_axis` without lock check; locked axes move while still showing the lock badge.
4. **P0 G3:** rebuild writes `UserVerdict::Unreviewed`; no product `correct_tie`.
5. **P0 topology:** PR #7 has the port complete but is behind c441 honesty/tests. Port, do not merge. Exclusive branch is now FF'd onto c441 so the port does not collide with 77 already-landed commits.
6. **Prediction:** thinnest lock-compatible slice is T4D demotion clock read forward (bands/dates, no model, no new WP, COMMANDS stays 36). Lands after T4D. Research preview already satisfies v0.1 data-collection preview (D8).
7. **Store:** forget does not `secure_delete` / checkpoint, so wrapped CK can remain in free pages while DEK still opens the file.
8. **IPC:** `complete_wizard` refusals are not `SessionRefusal` shape.
9. **Policy:** E1 plan hash ignores origin; changing endpoint after prepare still sends.

## Fixes this round

- Fast-forward exclusive branch onto `85aae68` (AD-8). Tracker docs only.

## Opus dispatch (Round 2)

| Owner | Scope | Must not touch |
|---|---|---|
| Opus T4D | Port from `origin/cursor/goal1-unblock-a073` per Fable #6 brief | Frozen algo rules; PR #7 wholesale merge; `soul-store` forget path |
| Opus store | `secure_delete` + checkpoint after forget | graph/import/profile/draft |
| Opus policy/IPC | Wizard refusal shape + E1 origin in plan hash | graph/T4D files |

## Commits / PRs

| When | What | SHA / PR | Merge |
|---|---|---|---|
| R1 start | Exclusive branch tracker | `3c6354f` (rebased) / #15 | Not to `main` |
| R1 end | FF onto c441 + this record | (this commit) / #15 | Same |

## Blocked

- Hosted Actions runners (billing/spending).
- Merge to `main` until M2 after T4D+schema.
- Merge of PR #7 / #4 / #2 (contract or D49).
- Author Win11 manual checklist.
- Parent has no merge-PR capability — record and continue.
- Prediction thin slice blocked on T4D port.

## Next round focus

Fable review of the T4D port. Then: tie-correction UI, intake `ignored` receipt, T4D projection sentences (simple behavior/group prediction), remaining policy nits. Do not start empty Goal 2 polish.
