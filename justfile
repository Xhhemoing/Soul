# Soul task runner.
#
# `just` is the documented entry point, but nothing here is more than a thin
# wrapper: every recipe below prints as a plain cargo or pnpm command, and the
# cargo equivalents are listed in the comment on each recipe so the repository
# stays usable, and CI stays reproducible, without installing `just`.
#
#   just              list the recipes
#   just ci-full      the whole Linux gate (G-L); see docs/gates/README.md
#
# Installing just:  cargo install just --locked --version 1.46.0
# (1.46 is the last release that builds on the pinned Rust 1.83 toolchain.)
#
# There is no hosted CI for this repository (DECISIONS D61). Every gate runs on
# a machine the author controls and leaves its record under docs/gates/.

set shell := ["bash", "-uc"]

export CARGO_TERM_COLOR := "always"
# `react-dom` does not export `act` under NODE_ENV=production, and a shell that
# sets it globally would take every vitest file down with it. The gate must not
# depend on the environment of whoever runs it.
export NODE_ENV := "test"

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
# No `--all-features`: no crate in this workspace declares a `[features]`
# table, so the flag changes nothing except the fingerprint, and a fingerprint
# that differs from `test` rebuilds vendored OpenSSL/SQLCipher from source
# (measured: +18 min on the 4-core gate machine).
lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

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
# database. Install a prebuilt binary:
#   cargo install cargo-deny --locked   (or a release tarball >= 0.18)
deny:
    cargo deny check

# The licence graph, in cargo-deny's own words, next to what `xtask sbom`
# says ships. A disagreement between the two files is the thing to look at.
# cargo equivalent: cargo deny list --layout crate --format json
deny-list:
    mkdir -p target/sbom
    cargo deny list --layout crate --format json > target/sbom/licences.json
    cargo deny list --layout license --format tsv > target/sbom/licences.tsv
    @echo "deny-list: target/sbom/licences.json, target/sbom/licences.tsv"

# The whole product, once, with this process's sockets watched. AC-21.
# cargo equivalent: cargo run -p soulcore --bin soul-headless -- smoke
# The JSON goes to stdout and the readable summary to stderr, so
# `just headless > report.json` leaves the summary on the terminal.
headless:
    cargo run -q -p soulcore --bin soul-headless -- smoke

# The one path that opens a socket, with this process's sockets watched.
# cargo equivalent: cargo run -p soulcore --bin soul-headless -- e1-watch
# The complement of `headless`: that one watches a flow with no endpoint
# configured, which is AC-21, and therefore never wakes the HTTP client. This
# one saves an endpoint the process is itself serving on 127.0.0.1, approves a
# draft, asks for a people summary and replays the approval, and reports every
# peer the kernel says this process had. Not chained into `ci`: it is a
# developer entry point onto what `soulcore/tests/e1_watch.rs` already runs.
e1-watch:
    cargo run -q -p soulcore --bin soul-headless -- e1-watch

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

# The Linux gate without cargo-deny, in the order the gate record lists it.
# `deny` is not chained here so a checkout without the cargo-deny binary can
# still run the documented entry point; `ci-full` adds it.
ci: lint schema e0 denylist fixtures-verify test smoke-lint sbom ui-lint ui-test

# G-L, the whole Linux gate (docs/gates/README.md). Record the result as
# docs/gates/<yyyymmdd>-<sha7>-linux.md before merging.
ci-full: ci deny deny-list

# The desktop shell's mock-runtime tests, which need the webview dev libraries
# (see the desktop section below). Not part of `ci-full`: the Linux gate
# machine may not have them. Run this wherever they are installed and record
# it in the gate file; on Windows `desktop-test` covers the same files.
desktop-shell-test:
    cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test ipc_roundtrip --test command_surface --test no_egress_path

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
# `--maxWorkers 2`: on a 4-core gate machine the jsdom suites contend for
# CPU and the longest wizard flow occasionally trips the 5 s default; two
# workers is the measured sweet spot, and the same file passes alone in 3 s.
ui-test: ui-install
    pnpm --filter @soul/desktop test -- --maxWorkers 2

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
# able to run the documented entry point. `desktop-shell-test` above runs
# ipc_roundtrip, command_surface and no_egress_path against Tauri's mock
# runtime, which needs no display, so the argument conversion the WebView
# depends on is exercised wherever the libraries exist. The Windows gate
# (G-W) runs `desktop-test` for the rest.

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
