#!/usr/bin/env bash
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
LOG="$SCRIPT_DIR/TEST_LOG.txt"
CARGO_TARGET_DIR="$SCRIPT_DIR/cargo-target"

cleanup() {
  rm -rf "$CARGO_TARGET_DIR"
}
trap cleanup EXIT

exec >"$LOG" 2>&1

echo "Round 2 gpt-sol-a probe log"
echo "MODEL_SLUG: gpt-5.6-sol-xhigh-fast"
echo "root=$ROOT"
echo

python3 "$SCRIPT_DIR/probe_docs.py"
probe_status=$?
echo "probe_docs.py exit_code=$probe_status"
echo

echo "== 6. cargo test --workspace --offline =="
(
  cd "$ROOT"
  CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo test --workspace --offline
)
cargo_status=$?
echo "cargo test --workspace --offline exit_code=$cargo_status"
echo

if [[ "$probe_status" -eq 0 && "$cargo_status" -eq 0 ]]; then
  echo "RUN_RESULT PASS"
  exit 0
fi

echo "RUN_RESULT FAIL"
exit 1
