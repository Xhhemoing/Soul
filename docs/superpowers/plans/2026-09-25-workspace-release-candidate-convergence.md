# Workspace Release Candidate Convergence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce one isolated Soul v0.1 release-candidate branch that contains the reviewed Q2 preparation code and the latest authoritative roadmap/status corrections without modifying or deleting existing worktrees.

**Architecture:** Start from the existing Q2 integration commit `b40a577`, because it contains the four reviewed Q2 preparation packages. Apply the later documentation commit `825b0f1` on top, resolve overlapping evidence files in favor of its corrected `NOT RUN` conclusions, and keep all product code from the Q2 integration branch. Validate the resulting exact commit with targeted Rust and frontend checks before considering broader gates.

**Tech Stack:** Git worktrees, PowerShell 7, Rust 1.83/Cargo, React/TypeScript, pnpm 10, Vitest.

## Global Constraints

- Run every command with `pwsh` and set `$ErrorActionPreference = 'Stop'` first.
- Read and write text explicitly as UTF-8.
- Do not delete, reset, clean, stash, or repoint any existing worktree.
- Do not add a new release gate, hash system, contract freeze, or coverage threshold.
- Preserve the existing G-L, G-W, G-M and AC acceptance structure.
- Do not mark installation, Linux validation, or human observation as passed unless actually executed.
- Keep Q2 preparation work classified as preparation until Goal 1 closure conditions are satisfied.

---

### Task 1: Establish the isolated release candidate

**Files:**
- Create: `docs/superpowers/plans/2026-09-25-workspace-release-candidate-convergence.md`

**Interfaces:**
- Consumes: Q2 integration commit `b40a5770fe18c3366771db8ee83b53ef306d13e5`
- Produces: Branch `codex/release-candidate-20260925` in `E:\Project\Soul-release-candidate-20260925`

- [x] **Step 1: Verify the candidate worktree starts clean**

  Run: `git status --short --branch`

  Expected: branch `codex/release-candidate-20260925` with no tracked or untracked changes.

- [x] **Step 2: Record the exact baseline**

  Run: `git rev-parse HEAD`

  Expected: `b40a5770fe18c3366771db8ee83b53ef306d13e5`.

- [x] **Step 3: Commit this execution plan**

  Run: `git add docs/superpowers/plans/2026-09-25-workspace-release-candidate-convergence.md; git commit -m "docs: plan release candidate convergence"`

### Task 2: Apply authoritative documentation corrections

**Files:**
- Modify: `README.md`
- Modify: `docs/DECISIONS.md`
- Modify: `docs/GOAL2_PLAN.md`
- Modify: `docs/PLAN_INDEX.md`
- Modify: `docs/PRODUCT_LOCK.md`
- Create: `docs/ROADMAP.md`
- Modify: `docs/STATUS.md`
- Modify: `docs/gates/20260925-95ff7d6-win.md`
- Modify: `docs/gates/20260925-ebff0c9-win.md`
- Modify: `docs/gates/README.md`
- Modify: `scripts/author-manual-checklist.md`

**Interfaces:**
- Consumes: Documentation commit `825b0f110b6d79dec088f6a21aa529c9efc80c1f`
- Produces: Q2 code plus current roadmap/status/evidence conclusions on one branch

- [x] **Step 1: Cherry-pick the documentation commit**

  Run: `git cherry-pick 825b0f110b6d79dec088f6a21aa529c9efc80c1f`

  Expected: either a clean cherry-pick or conflicts limited to evidence/checklist files already changed by `b40a577`.

- [x] **Step 2: Resolve evidence conflicts toward current facts**

  Required result:
  - Preserve Q2 product and test files from `b40a577`.
  - Preserve the existing Windows G-W facts for `ebff0c9`.
  - State that the external same-source NSIS evidence directory is missing.
  - Keep G-M 0–7, real install/uninstall, and current G-L as `NOT RUN` or incomplete.
  - Do not retain a package PASS that cannot be reproduced from existing evidence.

- [x] **Step 3: Verify no product code was removed by the documentation integration**

  Run: `git diff --name-status b40a577..HEAD -- apps crates fixtures Cargo.toml Cargo.lock package.json pnpm-lock.yaml`

  Expected: no product-code differences from `b40a577`.

- [x] **Step 4: Verify the four Q2 preparation files remain present**

  Run: `Test-Path apps/desktop/src/routes/Import.race.test.tsx; Test-Path crates/soulcore/tests/q2_continuity.rs; Test-Path crates/soulcore/tests/q2_recovery.rs; Test-Path crates/soulcore/tests/q2_scale.rs`

  Expected: four `True` values.

### Task 3: Run targeted candidate verification

**Files:**
- Test: `apps/desktop/src/routes/Import.race.test.tsx`
- Test: `crates/soulcore/tests/q2_continuity.rs`
- Test: `crates/soulcore/tests/q2_recovery.rs`
- Test: `crates/soulcore/tests/q2_scale.rs`

**Interfaces:**
- Consumes: integrated candidate source
- Produces: fresh command outputs for the exact candidate commit

- [x] **Step 1: Check Rust formatting**

  Run: `cargo fmt --all -- --check`

  Expected: exit code 0.

- [x] **Step 2: Run Q2 continuity and recovery tests**

  Run: `cargo test -p soulcore --test q2_continuity --test q2_recovery`

  Expected: all non-ignored tests pass.

- [x] **Step 3: Compile the Q2 scale test without running the ignored benchmark**

  Run: `cargo test -p soulcore --test q2_scale`

  Expected: ordinary scale assertions pass and the explicit measurement test remains ignored.

- [x] **Step 4: Run the Import race tests**

  Run: `pnpm --filter @soul/desktop test -- src/routes/Import.race.test.tsx`

  Expected: all target tests pass.

- [x] **Step 5: Run frontend lint and build**

  Run: `pnpm --filter @soul/desktop lint`

  Expected: exit code 0.

  Run: `pnpm --filter @soul/desktop build`

  Expected: exit code 0.

### Task 4: Review and hand off the optimized workspace

**Files:**
- Modify only if evidence changed: `docs/STATUS.md`
- Modify only if evidence changed: `docs/gates/20260925-ebff0c9-win.md`

**Interfaces:**
- Consumes: candidate diff and fresh targeted verification
- Produces: a clean, reviewable release-candidate branch and an explicit remaining-work list

- [x] **Step 1: Review the full candidate diff**

  Run: `git diff --stat b40a577..HEAD; git diff --check`

  Expected: only planned documentation changes plus the plan file; no whitespace errors.

- [x] **Step 2: Confirm the candidate remains isolated and inspect the current worktree registry**

  Run: `git worktree list --porcelain`

  Result: the candidate and root worktrees remain isolated. Concurrent workspace cleanup removed several old worktree directories and branch refs outside this candidate diff; their required Q2 commits remain reachable from the candidate history, and the performance work remains reachable from the VAL branch. No reset, clean, stash, or source deletion was performed by this convergence change.

- [x] **Step 3: Report remaining release blockers without changing their status**

  Required blockers:
  - current-source G-L;
  - reproducible same-source NSIS artifact evidence;
  - real install, smoke, and uninstall;
  - G-M 0–7 and affected native observations;
  - final broader G-W after the candidate commit is fixed.

## Execution Evidence

- Verified candidate before this receipt: `0bb27510af9247478884d23c777966645ee04377`; this receipt changes documentation only.
- Product diff from `b40a577` across `apps`, `crates`, `fixtures`, manifests and lockfiles: empty.
- `cargo fmt --all -- --check`: exit 0.
- `cargo test -p soulcore --test q2_continuity --test q2_recovery --test q2_scale`: continuity 4/4, recovery 3/3 and scale correctness 1/1 passed; the explicit scale measurement remained ignored by design.
- `vitest run src/routes/Import.race.test.tsx --maxWorkers 2`: 1 file, 11/11 passed.
- `pnpm --filter @soul/desktop lint`: exit 0.
- `pnpm --filter @soul/desktop build`: exit 0; 37 modules transformed.
- The first Rust attempt on the candidate default target was blocked by E-drive exhaustion (`os error 112`) before tests ran. The successful rerun used the isolated `D:\Soul-rc-cargo-target-9469d63` target; the failure is not recorded as a product test failure or pass.
- Full G-L, full G-W on the final candidate, reproducible NSIS packaging and G-M remain not run.
