#!/usr/bin/env bash
# Hermetic checks for scripts/branch-disposition.sh. Does not touch the Soul repo.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$ROOT/scripts/branch-disposition.sh"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/soul-disp.XXXXXX")"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

git init -q "$TMP/repo"
cd "$TMP/repo"
git checkout -q -b cursor/soul-goal1-7b1c
git commit --allow-empty -q -m trunk
git branch -M cursor/soul-goal1-7b1c
git checkout -q -b cursor/goal1-closeout-c441-2d70
printf 'ours\n' > STATUS.md
git add STATUS.md
git commit -q -m closeout-a
git checkout -q cursor/soul-goal1-7b1c
printf 'theirs-trunk\n' > STATUS.md
git add STATUS.md
git commit -q -m trunk-status
git checkout -q cursor/goal1-closeout-c441-2d70
printf 'theirs-closeout\n' > STATUS.md
git add STATUS.md
git commit -q -m closeout-b
git checkout -q cursor/soul-goal1-7b1c
git checkout -q -b cursor/goal1-build-audit-c441
git commit --allow-empty -q -m audit
git checkout -q --orphan main
git commit --allow-empty -q -m main-root
git checkout -q -b agent/dev-sota
git commit --allow-empty -q -m sota
git checkout -q cursor/soul-goal1-7b1c

# Pretend remotes
git remote add origin "$TMP/repo"
git update-ref refs/remotes/origin/cursor/soul-goal1-7b1c cursor/soul-goal1-7b1c
git update-ref refs/remotes/origin/cursor/goal1-build-audit-c441 cursor/goal1-build-audit-c441
git update-ref refs/remotes/origin/cursor/goal1-closeout-c441-2d70 cursor/goal1-closeout-c441-2d70
git update-ref refs/remotes/origin/main main
git update-ref refs/remotes/origin/agent/dev-sota agent/dev-sota
git update-ref refs/remotes/origin/cursor/goal1-unblock-a073 main

out="$TMP/out.txt"
SOUL_TRUNK_REF=origin/cursor/soul-goal1-7b1c "$SCRIPT" >"$out"

grep -q $'agent/dev-sota\t.*\tFORBIDDEN' "$out"
grep -q $'cursor/goal1-unblock-a073\t.*\tFORBIDDEN' "$out"
grep -E -q $'(^|/)main\t.*\tFORBIDDEN' "$out"
grep -q $'cursor/goal1-build-audit-c441\t.*\tFF-SAFE' "$out"
grep -q $'cursor/goal1-closeout-c441-2d70\t.*\tUNSAFE' "$out"

rc=0
SOUL_TRUNK_REF=origin/cursor/soul-goal1-7b1c "$SCRIPT" --merge agent/dev-sota || rc=$?
if [[ "$rc" -ne 2 ]]; then
  echo "selftest: forbidden merge must exit 2 (got $rc)" >&2
  exit 1
fi

rc=0
SOUL_TRUNK_REF=origin/cursor/soul-goal1-7b1c "$SCRIPT" --merge cursor/goal1-closeout-c441-2d70 || rc=$?
if [[ "$rc" -ne 2 ]]; then
  echo "selftest: unsafe merge must exit 2 (got $rc)" >&2
  exit 1
fi

before="$(git rev-parse HEAD)"
SOUL_TRUNK_REF=origin/cursor/soul-goal1-7b1c "$SCRIPT" --merge cursor/goal1-build-audit-c441
after="$(git rev-parse HEAD)"
if [[ "$after" == "$before" ]]; then
  echo "selftest: FF-SAFE merge did not move HEAD" >&2
  exit 1
fi
if [[ "$(git rev-parse HEAD)" != "$(git rev-parse origin/cursor/goal1-build-audit-c441)" ]]; then
  echo "selftest: HEAD is not the audit tip" >&2
  exit 1
fi

echo "branch-disposition.selftest: ok"
