# Round 1 / slot fable-a — Process, Git and CI blockers (Soul)

MODEL_SLUG: claude-fable-5-thinking-xhigh
Scope: process/git/CI only. No Goal 2. Product lock and frozen algorithms untouched.
All facts below re-verified against the repo and GitHub on 2026-08-24 ~16:20–16:35 UTC. Two facts in the brief were **wrong** and are corrected in §0.

## 0. Corrections to the brief (verify-first findings)

1. **PR #2's branch is NOT unrelated to main.** The "merge-base empty / root fbad85b" claim is an artifact of a **shallow clone**: this workspace's `.git/shallow` grafted history exactly at `fbad85b`. After `git fetch --unshallow`, `cursor/soul-goal1-7b1c` has 97 commits rooted at main's `ea6f62f`, and `git merge-base origin/main origin/cursor/soul-goal1-7b1c` = `ea6f62f`. Any prior analysis (including parts of this brief) that concluded "unrelated histories" was measuring the shallow graft, not the repo.
2. **PR #4's body claim "PR #1/#2 are already ancestors" is false for #2.** `agent/dev-sota` and `cursor/soul-goal1-7b1c` **diverged** at `862e858` ("STATUS: record WP09's first段, the shell"). Neither is an ancestor of the other (`git merge-base --is-ancestor` fails both ways). After the fork, **both branches independently implemented WP10 (soul-draft) and WP11 (soul-fileplan)** — goal1 with 36 commits (`862e858..3e88b48`), dev-sota with 11+ commits (`862e858..be1abee`, still growing).
3. PR #3 is no longer merely OPEN; it is **CONFLICTING** with main (add/add on `.agent_workspace/PROGRESS.md` and `.agent_workspace/round1/README.md`).

Verified-true facts from the brief: main = `ea6f62f` + `7b35bde` (#5 squash, algo crates only, no app code); PR #2 DRAFT + CONFLICTING; PR #4 CONFLICTING with add/add on `Cargo.toml`/`Cargo.lock`/`.gitignore`/`rust-toolchain.toml`/`.agent_workspace/*`; PR #1 DRAFT MERGEABLE; PR #5 MERGED; Windows CI on goal1 fails in `soul-fileplan tests/authorized_scan.rs` on exactly the two named tests, still red in run 32748412486 after commit `9ba160d`; branch policy is `cursor/` prefix (`docs/FORMAL_WORK_PROMPT.md:33`, §11.5, same text on main at `.agent_workspace/context/plan/FORMAL_WORK_PROMPT.md:33`).

---

## 1. Ranked blockers

### P0-1 — Two diverged Goal 1 trunks under simultaneous active development

**Evidence.**
- Fork point: `git merge-base origin/cursor/soul-goal1-7b1c origin/agent/dev-sota` = `862e858` (WP09 shell era).
- goal1 after fork: 36 commits through `3e88b48` (WP10 draft, WP11 fileplan, WP13 both halves: install smoke, SBOM, one-store session, merged questionnaire, `/files` + `/graph` wired — per `docs/STATUS.md` on that branch, lines 5–29 and "下一步" section).
- dev-sota after fork: `a3ec3f4`…`be1abee` — a **second, different** implementation of WP10/WP11 (`2920d2f` "register draft/fileplan crates", `eb74904` "draft replies that never leave, and a read-only file plan"), plus SOTA-review docs and hardening.
- dev-sota is being pushed **right now**: pushes at 16:09, 16:10, 16:19, 16:21 UTC (runs 32749201665, 32749277561, 32750178874, 32750406603), head moved `215a310` → `be1abee` during this analysis.
- PR #4 body says "本分支基于 Goal 1 壳 `cursor/soul-goal1-7b1c` … **不要再合并 PR #1 / #2**, 它们已经是祖先" — the ancestry half of that is stale/false (see §0.2).

**Impact.** This is the single biggest reason the project cannot progress. `crates/soul-draft` and `crates/soul-fileplan` exist on both lines with different content; merging one PR makes the other unmergeable wholesale. Every hour of continued work on dev-sota deepens the divergence and will be partially thrown away. Goal 1 cannot close (AC-26 "repo CI green" is a property of *one* trunk) while two candidate trunks exist.

**Fix.**
1. Declare `cursor/soul-goal1-7b1c` the trunk (rationale in §3) and record that decision in `docs/STATUS.md` on that branch.
2. **Stop the agent pushing to `agent/dev-sota`** (parent-agent action; nothing in git can do this).
3. Extract dev-sota's unique value as small cherry-picks onto `cursor/`-prefixed branches based on goal1 (candidates: the e0-audit message-send-crate bans with the synthetic `lettre` control from `9705fb4`; SOTA review documents `8676dbd`, `7f634c0`, `d908fe8`; the D32 QQ/WeChat deferral doc `215a310`). Evaluate `92eb9db`/`be1abee` (view surface) against goal1's `a77b7ec`/`a3a39a5` — likely redundant, drop if so.
   ```bash
   git checkout -b cursor/sota-hardening-cherry-83a6 origin/cursor/soul-goal1-7b1c
   git cherry-pick -x 215a310            # docs-only, applies cleanly
   git cherry-pick -x 8676dbd 7f634c0 d908fe8   # review docs
   # 9705fb4 touches code both lines changed — expect conflicts; port by hand, keep goal1's implementations
   ```
4. Close PR #4 with a comment: superseded by #2; unique parts cherry-picked (link the new PRs).

**What NOT to do.** Do not merge PR #4 into main. Do not attempt `git merge agent/dev-sota` into goal1 wholesale — the duplicated WP10/WP11 crates conflict across whole file trees and a mixed merge would produce a chimera nobody reviewed. Do not rebase either branch onto the other.

### P0-2 — PR #2 (the trunk) is unmergeable into main: add/add conflicts created by PR #5's squash

**Evidence.** `git merge-tree --write-tree origin/main origin/cursor/soul-goal1-7b1c` reports exactly four conflicts, all add/add: `.gitignore`, `Cargo.lock`, `Cargo.toml`, `rust-toolchain.toml`. Cause: PR #5 (`7b35bde`) squash-merged a **standalone** two-crate workspace (members = `crates/soul-algo-tie`, `crates/soul-algo-trait`; toolchain `1.83.0`; 2-line `.gitignore`) onto a previously near-empty main, while goal1 had long since created its own 15-member workspace versions of the same four files. GitHub: PR #2 `mergeable=false`, `mergeable_state=dirty`.

**Impact.** Nothing substantial can merge to main until this is resolved; every other open PR (#3, #4) has the same or a superset of these conflicts.

**Fix.** Merge main **into** goal1 once, resolving in goal1's favor plus the two algo crates:
```bash
git checkout -b cursor/goal1-merge-main-83a6 origin/cursor/soul-goal1-7b1c
git merge origin/main
# Resolve:
#  Cargo.toml         -> goal1's file + add "crates/soul-algo-tie", "crates/soul-algo-trait" to members
#  .gitignore         -> goal1's file (it already covers /target; add **/*.rs.bk if desired)
#  rust-toolchain.toml-> goal1's file (channel "1.83" == main's "1.83.0", keep goal1's components list)
#  Cargo.lock         -> take goal1's, then regenerate: cargo update -w  (algo crates have zero deps)
git rm -r --cached crates/soul-algo-tie/Cargo.lock crates/soul-algo-trait/Cargo.lock  # members must not carry their own locks
cargo test -p soul-algo-tie -p soul-algo-trait   # frozen tests must still pass, unchanged
git commit && git push -u origin cursor/goal1-merge-main-83a6
```
Notes: the algo crates have **no dependencies** (`crates/soul-algo-tie/Cargo.toml`, `crates/soul-algo-trait/Cargo.toml` on main), so workspace absorption is low-risk. They lack a `license` field, and goal1's `cargo deny check` / `xtask sbom` police licensing — if lint goes red, add `license = "LicenseRef-Soul-Proprietary"` (or `license.workspace = true`) to the two manifests. That is packaging metadata, not an algorithm change. Land this either directly on `cursor/soul-goal1-7b1c` or as a PR targeting it, then mark PR #2 ready-for-review.

**What NOT to do.** Do not rebase goal1's 96 commits onto main (rewrites a shared, open PR branch). Do not resolve by taking main's four files (that deletes the entire app workspace definition). Do not edit anything under `crates/soul-algo-*/src` or `tests` — algorithms are frozen; only manifest metadata may change.

### P0-3 — Windows CI red on the trunk (AC-26 gate): case-folding decoy lands inside the authorized root

**Evidence.**
- Run https://github.com/Xhhemoing/Soul/actions/runs/32748412486 (head `3e88b48`, 16:01 UTC): `test (windows-latest)` **failure**; lint, ubuntu tests, sbom all green. Job 97499320614 log: `authorized_scan.rs:65` and `:165` assert-eq failures; the actual lists contain `decoy.txt` / `("decoy.txt", "文档/decoy.txt")`, the expected lists do not. 7 of 9 tests in the file pass.
- Root cause is in the fixture, not in canonicalize: `crates/soul-fileplan/tests/common/mod.rs:104-108` writes `decoy.txt` into `base/alpha` (lowercase) as a deliberate case-collision decoy. On NTFS (case-insensitive) `alpha` **is** `Alpha`, so the decoy is physically inside the authorized root; the scan correctly reports it and the plan correctly proposes moving it. The module's own doc comment (lines 5–9) acknowledges the two readings; the *expected lists* in `authorized_scan.rs` were only ever written for the case-sensitive reading.
- Commit `9ba160d` ("fileplan: Windows canonicalize is a local drive, not a UNC share") fixed a **different** Windows bug (`\\?\C:\` verbatim-prefix handling, see `docs/STATUS.md` line 7 on goal1) — it could never have fixed this one, which is why the failure survived it.
- AC-26 (`docs/FORMAL_WORK_PROMPT.md:103`): "仓库 | CI | lint/test/schema/红线/打包绿" — Goal 1 cannot close with this job red.

**Fix.** Make the two expected lists platform-aware in `crates/soul-fileplan/tests/authorized_scan.rs`; on Windows the honest expectation *includes* the decoy:
- test at line 65: expected moves on `cfg(windows)` = `[("budget.csv", …), ("decoy.txt", "文档/decoy.txt"), ("photo.jpg", …), ("report.pdf", …), ("笔记.txt", …)]` (sorted order, matching the observed `left` list);
- test at line 165: expected relatives on `cfg(windows)` include `"decoy.txt"` after `"budget.csv"`.
Implement with two `cfg`-selected `vec!`s or a small `if cfg!(windows)` insertion, plus a one-line comment citing NTFS case folding. This is a test-expectation fix; no product code changes.

**What NOT to do.** Do not delete or `cfg(unix)`-gate the lowercase-`alpha` fixture — it powers the refusal corpus (`unauthorized_paths.rs:50-54`: "a sibling that was never named") and the explicit two-readings test (`case_is_decided_by_the_filesystem_rather_than_assumed`, which asserts that under `PathMatching::CaseFolded` the decoy **does** resolve — including the decoy on Windows is correct behavior, not a bug to suppress). Do not `#[ignore]` or skip the two tests on Windows — this file is "the half that has to work" for AC-18 and Windows is the target platform. Do not "fix" `soul-fileplan` to hide same-directory files.

### P1-1 — `agent/dev-sota` violates the frozen branch-naming policy

**Evidence.** `docs/FORMAL_WORK_PROMPT.md:33` (§11.5): "Git：`cursor/` 前缀分支" — identical text on main (`.agent_workspace/context/plan/FORMAL_WORK_PROMPT.md:33`) and on both feature branches. `agent/dev-sota` exists and is PR #4's head.

**Impact.** Not merge-blocking by itself, but it is the visible symptom of P0-1: a parallel line created outside the agreed namespace, whose PR body then made a false ancestry claim other agents trusted (this brief did). Policy exists precisely to prevent this class of confusion.

**Fix.** After P0-1 extraction is complete: close PR #4, then delete the branch (`git push origin --delete agent/dev-sota`). If the history should be preserved for archaeology, first `git push origin agent/dev-sota:refs/heads/cursor/archive-dev-sota-83a6`.
**What NOT to do.** Do not delete before the cherry-pick extraction; do not rename the branch under the open PR (GitHub would close #4 anyway — close it deliberately with an explanation instead).

### P1-2 — PR hygiene: #1, #2-draft, #3 in states that stall the queue

**Evidence & fixes, per PR.**
- **PR #2** is a **DRAFT**, so even with conflicts fixed it cannot merge. Fix: after P0-2 and P0-3 land and CI is green, mark ready for review (`gh pr ready 2` — write op, for the parent/owner to run) and merge with a **merge commit**, not squash.
- **PR #1** (`cursor/soul-product-lock-7b1c`, head `a785317`) is MERGEABLE but redundant: its head is an ancestor of goal1's head (verified `git merge-base --is-ancestor`). If PR #2 merges via merge commit, GitHub auto-marks #1 merged (its head becomes reachable from main) — zero-effort resolution. If #2 is squashed instead, #1 must be closed manually and its 96-commit provenance is lost; another reason to prefer a true merge.
- **PR #3** (`cursor/soul-status-round1-665b`): archival analysis docs; conflicts with main on 2 files (`.agent_workspace/PROGRESS.md`, `.agent_workspace/round1/README.md`) because PR #5 already landed a different `.agent_workspace/round1` archive. Fix: close it as a superseded snapshot (its content described "main is almost empty," which is no longer true), or if the author wants the docs kept, rebase resolving the two files by concatenation. Do not spend implementation effort here.

### P1-3 — dev-sota's Windows CI runs an un-runnable test (`ipc_roundtrip`): confirms it cannot be the trunk as-is

**Evidence.** Run https://github.com/Xhhemoing/Soul/actions/runs/32746339032 (dev-sota, 15:41 UTC): `test (windows-latest)` fails with `exit code: 0xc0000139, STATUS_ENTRYPOINT_NOT_FOUND` launching `ipc_roundtrip-*.exe` (job 97492798259 log, line ~2196) — a WebView2Loader.dll symbol-resolution failure in the Tauri mock-runtime test binary; zero assertions execute. goal1 already diagnosed and worked around exactly this on its line: the windows job compiles `ipc_roundtrip` with `--no-run` (`.github/workflows/ci.yml` ~line 140; rationale recorded in `docs/STATUS.md` "WP13 的取舍与遗留" item 7).

**Impact.** dev-sota cannot satisfy AC-26 without re-doing work goal1 already did. Only matters if anyone proposes dev-sota as trunk (they shouldn't, per P0-1) or blindly cherry-picks its CI workflow.
**Fix.** None needed on the goal1 trunk. When cherry-picking from dev-sota, never import its `.github/workflows/ci.yml`.

### P2-1 — Shallow clones in agent workspaces produce false git conclusions

**Evidence.** This workspace's `.git/shallow` contained `fbad85b`, which manufactured the "PR #2 root is unrelated to main" narrative that reached this brief.
**Impact.** Wrong integration decisions (e.g., "goal1 must be recreated") could have been taken on bad data.
**Fix.** Any agent doing history analysis runs `git rev-parse --is-shallow-repository` first and `git fetch --unshallow origin` if true. Worth one line in the shared brief / parent prompt.

### P2-2 — CI signal quality: push+PR double-triggers and cancel-in-progress on a hot branch

**Evidence.** `ci.yml` triggers on `push: branches: ["**"]` **and** `pull_request`, with `concurrency: cancel-in-progress: true`. On dev-sota, 14 of 18 recent runs are "cancelled" (list above) — rapid pushes kept killing runs, so no complete Windows signal existed for hours; each surviving run also runs twice (push + PR event), doubling minutes on a workflow with a heavy windows `package` job.
**Fix (optional).** Restrict the `push` trigger to `main` and let `pull_request` cover feature branches, or add `paths-ignore` for docs. Low priority; do not touch while P0s are open.

---

## 2. Recommended single integration path

**Trunk: `cursor/soul-goal1-7b1c` (PR #2).** It is the strict superset of Goal 1: WP01–WP13 (both halves) done per its `docs/STATUS.md` (single source of truth per §11.5), one process-wide store session, merged single questionnaire, `/files` + `/graph` + endpoint confirmation wired, install-smoke + SBOM + honest package CI, full connected history with main, policy-compliant name. dev-sota forked at WP09, re-implemented a subset (WP10/WP11), and its Windows CI has a failure mode goal1 already solved.

Order of operations (each step = one PR or one push to the trunk branch, CI green before the next):
1. **Stop pushes to `agent/dev-sota`** (parent-agent coordination, immediately — it moved twice during this analysis).
2. **Fix P0-3** on goal1 (the two `cfg(windows)` expected lists) → `test (windows-latest)` green → AC-26 achievable.
3. **Fix P0-2**: merge `origin/main` into goal1, resolving the four add/add files as specified; algo crates become workspace members with metadata-only manifest additions; frozen algo tests run unchanged in workspace CI.
4. **Merge PR #2 into main with a merge commit** (not squash): preserves the 96-commit history the STATUS narrative references, and auto-marks PR #1 merged since `a785317` becomes reachable from main.
5. **Extract dev-sota's unique deltas** (e0-audit send-crate bans + `lettre` control, SOTA review docs, D32 doc) as small `cursor/*-83a6` PRs onto main; **close PR #4** as superseded, then delete/archive `agent/dev-sota`.
6. **Close PR #3** (superseded snapshot) — or rebase-and-merge only its two doc files if the author insists.
7. Goal 1 then has exactly one trunk and three known remaining work items (wizard UI + 4 views, real DPAPI, Windows manual checklist) — all engineering, no process blockers.

PR disposition summary: **#2 merge** (after steps 2–3, via merge commit) · **#1 auto-merges** with it (else close) · **#4 close** after cherry-pick extraction · **#3 close** · **#5 already merged, nothing to do**.

---

## 3. NOT blockers (do not spend cycles here)

- **Frozen plan / PLAN_FROZEN (PR #1 content).** Present and identical in spirit on main (`.agent_workspace/context/plan/`), goal1 and dev-sota (`docs/FORMAL_WORK_PROMPT.md` is byte-identical between the two feature branches — verified by diff). Nothing needs to change; §11.5 and the AC matrix are consistent everywhere.
- **Frozen algorithms (T4D + A0, `crates/soul-algo-tie` / `soul-algo-trait`).** Zero-dependency, self-contained, tests included. Integration is a members-list + license-metadata operation (P0-2); no algorithm file needs editing, and none should be.
- **WP12.** Deleted by the frozen plan (`docs/FORMAL_WORK_PROMPT.md:59`: "WP12 已删除"). Any reference to it is noise.
- **AC-27 / file-write execution.** Explicitly v0.1.1, not Goal 1 (`docs/FORMAL_WORK_PROMPT.md:105`). goal1's `/files` view having no execute path is by design, asserted in tests (`Files.test.tsx`, `one_store.rs` per STATUS).
- **Goal 2.** Explicitly out of scope until Goal 1 closes (`docs/FORMAL_WORK_PROMPT.md:12`).
- **DPAPI skeleton and the 7-item Windows manual checklist.** These are the *remaining Goal 1 work* (goal1 STATUS "下一步" items 2–3), honestly reported in-product (`store_notice`), not process/git/CI blockers — they block the last AC sign-offs, not merging.
- **`ipc_roundtrip` not executing on windows-latest (on the goal1 trunk).** Deliberate, documented trade-off (compile-only via `--no-run`; runs fully on Linux, 11 assertions). It only becomes a blocker in dev-sota's CI shape (P1-3).
