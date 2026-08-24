#!/usr/bin/env bash
# Regenerates TEST_LOG.txt from real command output. Nothing in the log is
# typed by hand except the four header lines below.
set -u
cd "$(dirname "$0")/soul-algo-tie" || exit 1
OUT="../TEST_LOG.txt"

{
  echo "MODEL_SLUG: claude-opus-5-thinking-high-fast"
  echo "Round 3 / opus-a — soul-algo-tie 测试日志"
  echo "包路径：.agent_workspace/round3/opus-a/soul-algo-tie"
  echo "默认规则 = T4D（TieAlgo::DEFAULT，公开 score()）；T4 保留为回退；T0 仅测试用 oracle"
  echo "as_of 常量 = 2026-08-24T14:00:00Z = 1787580000（调用方传入；全程不读墙钟）"
  echo
} > "$OUT"

run() {
  echo "=== \$ $* ===" >> "$OUT"
  "$@" >> "$OUT" 2>&1
  echo >> "$OUT"
}

run rustc --version
run cargo --version
run uname -srmo
run cargo tree

echo "=== \$ cargo fmt --check ===" >> "$OUT"
if cargo fmt --check >> "$OUT" 2>&1; then echo "(no diff)" >> "$OUT"; fi
echo >> "$OUT"

run cargo clippy --all-targets -- -D warnings
run cargo test
run cargo test --release
run cargo run --example matrix
