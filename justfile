# Soul task runner.
#
# `just` is the documented entry point, but nothing here is more than a thin
# wrapper: every recipe below prints as a plain cargo or pnpm command, and the
# cargo equivalents are listed in the comment on each recipe so the repository
# stays usable, and CI stays reproducible, without installing `just`.
#
#   just              list the recipes
#   just ci           everything CI runs on Linux
#
# Installing just:  cargo install just --locked --version 1.46.0
# (1.46 is the last release that builds on the pinned Rust 1.83 toolchain.)

set shell := ["bash", "-uc"]

export CARGO_TERM_COLOR := "always"

# Show the available recipes.
default:
    @just --list

# One-time developer setup. Safe to re-run.
# cargo equivalent: rustup component add rustfmt clippy && pnpm install
setup:
    rustup component add rustfmt clippy
    @echo "setup: rust toolchain ready ({{ if path_exists('rust-toolchain.toml') == 'true' { 'pinned by rust-toolchain.toml' } else { 'unpinned' } }})"
    just ui-install

# cargo equivalent: cargo fmt --all
fmt:
    cargo fmt --all

# cargo equivalent: cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings
lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# cargo equivalent: cargo test --workspace --all-targets
test:
    cargo test --workspace --all-targets

# Frozen contract digests must match docs/schemas/schemas.lock.json.
# cargo equivalent: cargo run -p xtask -- schema-freeze --check
schema:
    cargo run -q -p xtask -- schema-freeze --check

# Re-pin the contracts. Only in a change that has approval to touch them.
# cargo equivalent: cargo run -p xtask -- schema-freeze --write
schema-write:
    cargo run -q -p xtask -- schema-freeze --write

# No HTTP client in the shipped graph, no URL literal outside the allowlist.
# cargo equivalent: cargo run -p xtask -- e0-audit
e0:
    cargo run -q -p xtask -- e0-audit

# No clinical vocabulary, no numeric ratings.
# cargo equivalent: cargo run -p xtask -- denylist-audit
denylist:
    cargo run -q -p xtask -- denylist-audit

# The fixture corpora parse and still carry what the acceptance tests need.
# cargo equivalent: cargo test -p soul-testkit --test fixture_corpus
fixtures-verify:
    cargo test -q -p soul-testkit --test fixture_corpus

# Third-party licences, advisories, and the HTTP client ban.
# cargo equivalent: cargo deny check
# Needs cargo-deny >= 0.18; older releases cannot parse the current advisory
# database. See .github/workflows/ci.yml for the pinned install.
deny:
    cargo deny check

# Everything the Linux test job runs, in the order it runs it.
# `deny` is deliberately not chained here: it needs a cargo-deny binary that
# the workflow installs in the separate lint job. Run `just deny` alongside
# this locally to reproduce CI in full.
ci: lint schema e0 denylist fixtures-verify test ui-lint ui-test

# Everything CI checks anywhere, including cargo-deny.
ci-full: ci deny

# ---------------------------------------------------------------- UI ---
# Placeholders until WP09 brings up apps/desktop. They exit 0 on purpose:
# `just ci` should be runnable today, and these recipes are where the real
# pnpm commands will go rather than being invented at that point.

ui-install:
    @echo "ui-install: no UI workspace yet (WP09 adds apps/desktop); skipping pnpm install"

ui-lint:
    @echo "ui-lint: no UI workspace yet (WP09 adds apps/desktop); nothing to lint"

ui-test:
    @echo "ui-test: no UI workspace yet (WP09 adds apps/desktop); nothing to test"
