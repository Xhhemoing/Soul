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

# The whole product, once, with this process's sockets watched. AC-21.
# cargo equivalent: cargo run -p soulcore --bin soul-headless -- smoke
# The JSON goes to stdout and the readable summary to stderr, so
# `just headless > report.json` leaves the summary on the terminal.
headless:
    cargo run -q -p soulcore --bin soul-headless -- smoke

# A CycloneDX bill of materials per shipped workspace, into target/sbom.
# cargo equivalent: cargo run -p xtask -- sbom
# Offline and deterministic: it reads `cargo metadata` and Cargo.lock, and
# fails on a dependency that states no licence.
sbom:
    cargo run -q -p xtask -- sbom

# What can be checked about scripts/install-smoke.ps1 without a Windows box:
# the report fields it reads exist, it installs and uninstalls silently, and
# it downloads nothing. `pwsh -File ... -DryRun` on top of that if pwsh is
# installed here, which on a Linux CI host it usually is not.
# cargo equivalent: cargo test -p soulcore --test install_smoke_script
smoke-lint:
    cargo test -q -p soulcore --test install_smoke_script
    @if command -v pwsh > /dev/null; then \
        pwsh -NoProfile -Command '$e=$null; [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path scripts/install-smoke.ps1).Path, [ref]$null, [ref]$e) > $null; if ($e.Count) { $e; exit 1 }; "install-smoke.ps1: parses clean"'; \
        pwsh -NoProfile -File scripts/install-smoke.ps1 -DryRun -SkipInstall; \
    else \
        echo "smoke-lint: no pwsh here, so only the Rust checks ran (see crates/soulcore/tests/install_smoke_script.rs)"; \
    fi

# Everything the Linux test job runs, in the order it runs it.
# `deny` is deliberately not chained here: it needs a cargo-deny binary that
# the workflow installs in the separate lint job. Run `just deny` alongside
# this locally to reproduce CI in full.
ci: lint schema e0 denylist fixtures-verify test smoke-lint sbom ui-lint ui-test

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
# Not chained into `ci`: a Linux author without that GUI stack still has to be
# able to run the documented entry point. CI installs it as a separate step —
# the ubuntu job runs ipc_roundtrip, command_surface and no_egress_path against
# Tauri's mock runtime, which needs no display, so the argument conversion the
# WebView depends on is exercised there. The Windows job runs the rest of the
# desktop tests and compiles ipc_roundtrip with `--no-run`, because that
# runner's WebView2Loader cannot start the harness.

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
