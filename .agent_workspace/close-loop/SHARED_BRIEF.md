# Goal 1 close-loop — shared brief

Parent: cursor-grok-4.6-high. Run `bc-b296c4d9-0feb-4f6e-b844-aeaca7a8a073`.
Branch: `cursor/goal1-close-loop-a073` (from `cursor/goal1-unblock-a073` @ `6133307`).
Unique merge-to-main path remains PR #7 (`cursor/goal1-unblock-a073`). This branch is a stacked close-loop, not a second implementation trunk.

## First line of every agent report

Declare the actual model slug you ran. No silent downgrade. If the requested slug was unavailable, stop and say so.

## What this loop is

v0.1 WPs (WP01–WP11, WP13, DPAPI) are already landed. T4D / A0 / A2 / G1+ / G3 / intake lock / import clocks are already landed. `origin/main` plan-doc freeze is already absorbed. PR #7 is `MERGEABLE` / `CLEAN` vs `main`.

This loop closes **remaining honest gaps** that are still code or docs on this tree. It does **not** restart Goal 1. It does **not** open Goal 2.

## Hard bans

- Do not start Goal 2. Do not read `GOAL2_POLISH_PROMPT.md` as a work order.
- Do not `CreateGoal：Goal 1`. Do not re-dispatch a planner to split WPs.
- Do not silent-patch F04c (no third demotion gate).
- Do not pull SQLCipher / store / Tauri / HTTP into `soul-algo-*`.
- Do not `cargo generate-lockfile`. Do not empty-commit to refresh hosted CI.
- Do not merge PR #1 / #2 / #4 as the Goal 1 path. Do not land work on `agent/` branches (D49, FORMAL 11.5). Cloud + repo policy: `cursor/<name>-a073` only.
- Do not treat exhausted GitHub Actions minutes or the author Win11 checklist as code tasks.
- Do not implement GC-6/7 (D34) or GC-9b 「由你本人指定」(D35).
- Do not copy `docs/BLOCKERS.md` wholesale into this tree (still PR #6).
- Do not change frozen algorithm crate constants or `COPY_ZH.md` keys.

## Known remaining work (parent-seeded; confirm or refute with evidence)

| ID | Kind | Notes |
|---|---|---|
| CI-TRUNK | code | `.github/workflows/ci.yml` auto-push still lists `main` + `cursor/soul-goal1-7b1c` only. Unique trunk is `cursor/goal1-unblock-a073`. Adding a `pull_request` trigger is forbidden (minutes). `workflow_dispatch` stays. |
| R1-LEGACY | code P2 | After typed `tie_strength`, `correct_tie`/`release_tie` on a never-rebuilt eight-field edge serializes `algorithm_id: ""` and fails schema. Rebuild-first or refuse-with-rebuild; do not loosen the T4D enum. See `.agent_workspace/plan-polish/round3/fable-b/REGRESSIONS.md` R-1. |
| DOC-D49 | docs | D49 still names `cursor/soul-goal1-7b1c` as unique trunk. Successor is PR #7. Append, do not rewrite history silently if you touch DECISIONS. |
| DOC-INDEX | docs | PLAN_INDEX “only constant names” line vs D60 (COPY_ZH/REJECTED may cite frozen numbers). |
| M1-PR4 | GitHub | PR #4 `agent/dev-sota` still open. Do not merge it. Closing it is a human/parent GitHub action; this environment’s `gh` is read-only. |
| AUTHOR | not code | `scripts/author-manual-checklist.md` |
| MINUTES | not code | Hosted empty runners |

## File ownership this round

Write reports only under your slot directory. Do not edit another slot’s files.

## Authority

`docs/PRODUCT_LOCK.md`, `docs/DECISIONS.md`, `docs/algorithms/DECISION.md`, `docs/FORMAL_WORK_PROMPT.md`, `docs/STATUS.md`, `docs/PLAN_INDEX.md`.
