MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 2 fable-a — git topology / integration path / PR disposition (deltas only)

Re-verified 2026-08-24 ~16:45 UTC on a full (non-shallow) clone, live `gh` against Xhhemoing/Soul.

## §1 仓库拓扑 — AGREE, all hashes re-measured

- merge-bases confirmed: main∩goal1 = `ea6f62f`, goal1∩dev-sota = `862e858`, main∩dev-sota = `ea6f62f`.
- Counts confirmed: goal1 has 96 commits not on main, 36 past the fork; dev-sota 12 past the fork.
- `git merge-tree --write-tree origin/main origin/cursor/soul-goal1-7b1c`: exactly 4 add/add
  conflicts (`.gitignore`, `Cargo.lock`, `Cargo.toml`, `rust-toolchain.toml`). No fifth file.
- main is 2 commits total; `7b35bde` is single-parent ("(#5)" squash). "近空 + squash" is exact.
- PR states confirmed: #1 MERGEABLE+draft, #2 CONFLICTING+draft, #3 CONFLICTING, #4 CONFLICTING,
  #5 MERGED. `a785317` (PR #1) is an ancestor of BOTH goal1 and dev-sota.
- PR #4 body claim "「#1/#2 已是祖先」对 #2 为假" verified: true for #1, false for #2
  (`3e88b48` is not reachable from `fc9836e`).
- **Delta (info, update before freeze):** dev-sota pushed again after the draft's 16:35 check:
  `be1abee..fc9836e` ("wire draft and file-plan views through the desktop shell", CI in flight).
  Pin the tip as `fc9836e` in the frozen doc. It adds `Draft.tsx`/`Files.tsx` routes goal1
  ALREADY has — with tests (`Draft.test.tsx`, `Files.test.tsx`, plus `Graph.tsx` dev-sota lacks).
  No new cherry-pick item; the trunk verdict is *reinforced*, and the stop-push order gets more
  urgent with every push.
- Trunk verdict AGREE. Both branches name the same 15 crates (implementations diverged);
  goal1 is ahead on the desktop surface. Not wrong trunk.

## §2 M1 — AGREE (P0 stands)

- Cherry-pick list verified as dev-sota-only: `lettre`/`teloxide`/`matrix-sdk` ban lives in
  dev-sota `crates/xtask/src/egress.rs` + self_test, zero matches on goal1; D32 only in
  dev-sota docs. List is still current after `fc9836e`.
- ci.yml warning verified: dev-sota Windows job runs desktop `cargo test --all-targets` with no
  ipc_roundtrip carve-out; goal1 has the dedicated `--no-run` step. Do not adopt dev-sota's ci.yml.

## §2 M2 — AGREE with two small process gaps (both P2, doc-only)

- Conflict table is correct: goal1 toolchain (`1.83` + components) ⊇ main (`1.83.0` bare);
  algo crates are zero-dependency so the lock regen is trivial; merge-commit-not-squash is
  correct GitHub semantics (merge makes `a785317` reachable from main → PR #1 auto-flips to
  merged; squash breaks that ancestry).
- **Gap 1:** the recipe builds `cursor/goal1-absorb-main-a073` but never says how it becomes
  PR #2's tip. Either push the merge commit to `cursor/soul-goal1-7b1c` (PR #2 turns
  MERGEABLE), or open a new PR from the absorb branch and merge THAT with a merge commit
  (#2 and #1 then both auto-flip to merged). Pick one in the frozen text.
- **Gap 2:** PR #2 (and #1) are **draft** PRs; GitHub will not merge a draft. Step 11 needs
  "mark PR #2 ready for review" first.
- `main` has no branch protection; default branch is `main`. Nothing hidden blocks step 11.
- Nit: main's workspace license is `LicenseRef-All-Rights-Reserved` vs goal1's
  `LicenseRef-Soul-Proprietary`; the hedged `license.workspace = true` sentence silently picks
  goal1's string. Fine, but worth naming in the table so nobody "fixes" it later.

## §2 M3 — AGREE, confirmed from the live failing run (not from reading source alone)

- Run 32748412486 (tip `3e88b48`): only `test (windows-latest)` failed; lint/ubuntu/sbom/package
  green. Exactly 2 failures, panics at `authorized_scan.rs:65` and `:165` — the cited lines.
- Fixture writes `base/alpha/decoy.txt` unconditionally (`common/mod.rs`); symlink escapes are
  `#[cfg(unix)]`-gated, so the decoy is the only Windows-divergent input. Root cause stands.
- Collateral verified: `soul_graph`, `sqlcipher_smoke`, `soulcore` appear in the Windows log
  only as Compiling/warning lines; their test binaries never ran. No `--no-fail-fast` anywhere
  in goal1's ci.yml. The "SECURITY.md claim false on this branch" point is real.

## §5 工序 / §0 — AGREE

- Order is safe; steps 0–3 verified feasible against live state. Step 11 needs the two P2
  additions above (ready-for-review; absorb-branch landing path). §0 row "单独合 PR #1" is
  correct — merge-commit of #2 closes it as merged automatically.

## Attack summary

- Wrong trunk: **no** (reinforced by dev-sota's own latest push duplicating goal1 work).
- Dangerous merge recipe: **no**; only the two P2 process gaps above.
- Missed process P0: **none found** (protection off, no required checks, no hidden PR state).
- False P0: **none** in M1/M2/M3 — all three re-confirmed with live git/gh evidence.
