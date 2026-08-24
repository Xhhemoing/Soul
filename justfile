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
# cargo equivalent: rustup component add rustfmt clippy && pnpm install --frozen-lockfile
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
# apps/desktop, landed by WP09. `ui-lint` and `ui-test` install first, so
# `just ci` works from a clean checkout; `just` runs a shared dependency once.

# pnpm equivalent: pnpm install --frozen-lockfile
ui-install:
    pnpm install --frozen-lockfile

# pnpm equivalent: pnpm --filter @soul/desktop lint  (tsc --noEmit && eslint .)
ui-lint: ui-install
    pnpm --filter @soul/desktop lint

# pnpm equivalent: pnpm --filter @soul/desktop test  (vitest run)
ui-test: ui-install
    pnpm --filter @soul/desktop test

# The production frontend bundle, which `tauri build` embeds.
# pnpm equivalent: pnpm --filter @soul/desktop build
ui-build: ui-install
    pnpm --filter @soul/desktop build

# ----------------------------------------------------------- desktop ---
# apps/desktop/src-tauri is its own cargo workspace, so the recipes above and
# the whole-workspace recipes at the top never touch it. That is deliberate:
# building it needs a WebView, and Linux is only a CI host for this product.
#
# On the target platform (Windows 11 x64) nothing extra is required. On a
# Linux box, first:
#   sudo apt-get install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
#                        librsvg2-dev libxdo-dev build-essential
#
# Not chained into `ci`: the Linux job would then need that GUI stack for a
# platform Soul does not target. The Windows CI job runs `desktop-test`.

# cargo equivalent: cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets
desktop-check:
    cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets

# cargo equivalent: cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets
desktop-test:
    cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets

# Run the shell against the Vite dev server. Desktop session required.
# pnpm equivalent: pnpm --filter @soul/desktop tauri dev
desktop-dev: ui-install
    pnpm --filter @soul/desktop tauri dev
