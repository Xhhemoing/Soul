#!/usr/bin/env bash
# Fail-closed listing of origin (or --repo) branches vs the unique Goal 1 trunk.
# Default is dry-run. --merge NAME only fast-forwards a non-forbidden, conflict-free
# branch that is not already contained.
set -euo pipefail

TRUNK_REF="${SOUL_TRUNK_REF:-origin/cursor/soul-goal1-7b1c}"
REMOTE="${SOUL_REMOTE:-origin}"
DO_FETCH=0
MERGE_NAME=""
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"

usage() {
  cat <<'EOF'
Usage: scripts/branch-disposition.sh [--fetch] [--merge <branch>]
  --fetch         git fetch <remote> before classifying
  --merge NAME    fast-forward NAME onto HEAD if allowed (else exit 2)
Environment:
  SOUL_TRUNK_REF  default origin/cursor/soul-goal1-7b1c
  SOUL_REMOTE     default origin
EOF
}

forbidden_name() {
  local n="${1#origin/}"
  n="${n#"$REMOTE"/}"
  case "$n" in
    main|HEAD|agent/dev-sota|cursor/goal1-unblock-a073)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --fetch) DO_FETCH=1; shift ;;
    --merge)
      MERGE_NAME="${2:-}"
      if [[ -z "$MERGE_NAME" ]]; then
        echo "branch-disposition: --merge needs a name" >&2
        exit 2
      fi
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "branch-disposition: unknown arg: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$REPO_ROOT" ]]; then
  echo "branch-disposition: not a git repository" >&2
  exit 2
fi
cd "$REPO_ROOT"

if [[ "$DO_FETCH" -eq 1 ]]; then
  git fetch "$REMOTE"
fi

if ! git rev-parse -q --verify "$TRUNK_REF" >/dev/null; then
  echo "branch-disposition: trunk $TRUNK_REF is missing" >&2
  exit 2
fi

classify() {
  local ref="$1"
  local short="${ref#refs/remotes/}"
  short="${short#refs/heads/}"
  local tip
  tip="$(git rev-parse --short "$ref")"
  if forbidden_name "$short"; then
    printf '%s\t%s\tFORBIDDEN\t\n' "$short" "$tip"
    return
  fi
  if [[ "$ref" == "$(git rev-parse -q --verify "$TRUNK_REF")" ]] || \
     [[ "$(git rev-parse "$ref")" == "$(git rev-parse "$TRUNK_REF")" ]]; then
    printf '%s\t%s\tUNIQUE-TRUNK\t\n' "$short" "$tip"
    return
  fi
  local mb
  if ! mb="$(git merge-base "$TRUNK_REF" "$ref" 2>/dev/null)"; then
    printf '%s\t%s\tNO-MERGE-BASE\t\n' "$short" "$tip"
    return
  fi
  local ahead behind
  ahead="$(git rev-list --count "$TRUNK_REF..$ref")"
  behind="$(git rev-list --count "$ref..$TRUNK_REF")"
  if [[ "$ahead" -eq 0 && "$behind" -eq 0 ]]; then
    printf '%s\t%s\tEQUALS-TRUNK\t\n' "$short" "$tip"
    return
  fi
  if [[ "$ahead" -eq 0 ]]; then
    printf '%s\t%s\tALREADY-IN-TRUNK\t\n' "$short" "$tip"
    return
  fi
  local tree conf
  tree="$(git merge-tree "$mb" "$TRUNK_REF" "$ref")"
  conf="$(printf '%s\n' "$tree" | grep -c '^changed in both' || true)"
  if [[ "$behind" -eq 0 && "$conf" -eq 0 ]]; then
    printf '%s\t%s\tFF-SAFE\t+%s/-%s\n' "$short" "$tip" "$ahead" "$behind"
    return
  fi
  printf '%s\t%s\tUNSAFE\t+%s/-%s conflicts=%s\n' "$short" "$tip" "$ahead" "$behind" "$conf"
}

echo -e "branch\ttip\tclass\tnote"
if git show-ref --verify --quiet refs/remotes/"$REMOTE"/HEAD || true; then
  :
fi

listed=0
while read -r ref; do
  [[ -z "$ref" ]] && continue
  classify "$ref"
  listed=1
done < <(git for-each-ref --format='%(refname)' refs/remotes/"$REMOTE" | sed '/refs\/remotes\/'"$REMOTE"'$/d')

if [[ "$listed" -eq 0 ]]; then
  echo "branch-disposition: no $REMOTE remotes" >&2
  exit 2
fi

if [[ -z "$MERGE_NAME" ]]; then
  exit 0
fi

name="${MERGE_NAME#origin/}"
name="${name#"$REMOTE"/}"
if forbidden_name "$name"; then
  echo "branch-disposition: refused to merge $name" >&2
  exit 2
fi

target=""
if git rev-parse -q --verify "$REMOTE/$name" >/dev/null; then
  target="$REMOTE/$name"
elif git rev-parse -q --verify "$name" >/dev/null; then
  target="$name"
else
  echo "branch-disposition: unknown branch $MERGE_NAME" >&2
  exit 2
fi

class_line="$(classify "$target")"
class="$(printf '%s\n' "$class_line" | awk -F'\t' '{print $3}')"
if [[ "$class" != "FF-SAFE" ]]; then
  echo "branch-disposition: $name is $class, not FF-SAFE" >&2
  exit 2
fi

git merge --ff-only "$target"
echo "branch-disposition: fast-forwarded $(git rev-parse --short HEAD) from $target"
