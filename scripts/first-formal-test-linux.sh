#!/usr/bin/env bash
# Segment A of the first formal test (Linux). Not hosted green, not Win11.
# Equivalents of `just ci-full` from the justfile comments. `just` is optional.
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

echo "first-formal-test-linux: $(git rev-parse --short HEAD) on $(git branch --show-current)"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo run -q -p xtask -- schema-freeze --check
cargo run -q -p xtask -- e0-audit
cargo run -q -p xtask -- denylist-audit
cargo test -q -p soul-testkit --test fixture_corpus
cargo test --workspace --all-targets
cargo test -q -p soulcore --test install_smoke_script
cargo run -q -p xtask -- sbom
pnpm install --frozen-lockfile
pnpm --filter @soul/desktop lint
pnpm --filter @soul/desktop test
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets

if command -v cargo-deny >/dev/null; then
  GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null cargo deny check
else
  echo "first-formal-test-linux: cargo-deny not on PATH (CI installs 0.18.6); skipping deny"
fi

echo "first-formal-test-linux: segment A green on this machine (not hosted AC-26)"
