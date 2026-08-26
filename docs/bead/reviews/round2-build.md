# ROUND 2 · WP-B10 build / CI / performance probe

Reviewer: `gpt-5.6-sol-xhigh-fast`. Probe only; this brief does not implement a workflow or change product code.

## Verdict

WP-B10 is ready to implement as one new, isolated workflow. The repository already has the two required package-level command surfaces, `bead-core` remains outside the root Cargo workspace, and the root pnpm scripts still target Soul only. No change to Soul `.github/workflows/ci.yml` is needed or permitted.

## Verified baseline

- `.github/workflows/` contains only Soul `ci.yml`. Its automatic trigger is still `push` on `main` and `cursor/soul-goal1-7b1c`; it has no `pull_request` trigger.
- Root `package.json` remains BD4-compliant:
  - `lint`: `pnpm --filter @soul/desktop lint`
  - `test`: `pnpm --filter @soul/desktop test`
  - `build`: `pnpm --filter @soul/desktop build`
  - `dev`: `pnpm --filter @soul/desktop tauri dev`
  These scripts must stay Soul-only. `apps/*` already discovers `apps/bead`; B10 must not repoint the root scripts or change `pnpm-workspace.yaml`.
- `crates/bead-core/Cargo.toml` has its own `[workspace]`, its own lockfile, Rust 1.83, and no dependencies. Root `Cargo.toml` does not list `crates/bead-core`; it must remain excluded (I-2/BD3).
- The app exposes `lint`, `test`, and `build`; the core is addressable safely with `--manifest-path crates/bead-core/Cargo.toml`.
- The current branch passes the exact proposed command sequence:
  - frozen pnpm install: lockfile already current, pnpm 10.15.0;
  - app lint: pass;
  - app test: 22 files / 258 tests pass;
  - app production build: pass, 307.23 kB JavaScript / 97.96 kB gzip;
  - core fmt and clippy with warnings denied: pass;
  - core test: 153 tests including doctests pass.

The observed local wall times are diagnostics, not budgets: app test about 7 seconds, app build about 2.4 seconds including TypeScript checking, and a cold local core test build about 6.6 seconds. Hosted-runner timings will include checkout, setup, and cache variance.

## Implementable B10 workflow

Create `.github/workflows/bead.yml` in the implementation change. Do not edit Soul `ci.yml`.

```yaml
name: Bead

on:
  push:
    branches:
      - "cursor/bead*"
    paths:
      - "apps/bead/**"
      - "crates/bead-core/**"
      - "pnpm-lock.yaml"
      - ".github/workflows/bead.yml"
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: ${{ github.workflow }}-${{ github.ref_name }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always
  CARGO_NET_RETRY: 3

jobs:
  bead-app:
    name: bead app (lint, test, build)
    if: startsWith(github.ref, 'refs/heads/cursor/bead')
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
      - run: pnpm install --frozen-lockfile
      - run: pnpm --filter @bead/app lint
      - run: pnpm --filter @bead/app test
      - run: pnpm --filter @bead/app build

  bead-core:
    name: bead-core (fmt, clippy, test)
    if: startsWith(github.ref, 'refs/heads/cursor/bead')
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.83
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
        with:
          key: bead-core
          workspaces: crates/bead-core -> target
      - run: cargo fmt --manifest-path crates/bead-core/Cargo.toml --check
      - run: cargo clippy --manifest-path crates/bead-core/Cargo.toml --all-targets -- -D warnings
      - run: cargo test --manifest-path crates/bead-core/Cargo.toml
```

### Trigger and isolation rules

1. The only automatic branch selector is `cursor/bead*`. The paths deliberately omit `docs/bead/**`; review-only pushes should not buy runners.
2. There is no `pull_request` trigger. Push checks already attach to a PR, while PR path evaluation considers the whole PR and causes repeat runs on later documentation commits.
3. Manual dispatch is for `bead.yml` only. The job guards make a manual selection of a non-bead ref skip both jobs. BD17 remains absolute: never dispatch Soul `CI` from a bead ref.
4. Workflow name `Bead` plus `github.workflow` in the concurrency key prevents this workflow from cancelling Soul `CI`.
5. The two jobs run package-specific commands only. They do not call root pnpm scripts, root Cargo workspace tests, or `just ci`.
6. I-7 remains the merge acceptance record: package test, root `just ci`, `cargo run -p xtask -- e0-audit`, and `cargo run -p xtask -- denylist-audit`. Those coexistence checks must stay green, but they do not justify widening Soul CI or making it a bead trigger. If they are automated in B10 later, use the separate `coexist` job from the round-1 brief rather than folding Soul tests into either package job.
7. Do not use empty commits to probe hosted-runner billing.

## Performance disposition

### Worth measuring later

- **End-to-end photo conversion in a real browser after the Worker path exists.** Use representative 512×512 RGBA inputs, both dithering states, the 48-colour palette, and a 56×56 output. Measure p50/p95 elapsed time, Worker transfer cost, and main-thread long tasks. Keep the round-1 soft target of under 1 second on a stated desktop reference machine; do not turn it into a hosted-CI wall-clock gate.
- **Assembly interaction after WP-B04 lands.** In a browser trace, measure initial 56×56 render and one next-step update. Verify the timer does not rerender all 3,136 cells. Only evidence of missed frames or long tasks should trigger canvas/virtualization work.
- **Bundle growth after B04/B05/B07 dependencies settle.** Record Vite raw/gzip output and investigate step changes. The current 97.96 kB gzip result is a baseline observation, not a frozen threshold.

### `NO_HIGH_VALUE` now

- Criterion/Vitest microbenchmark suites and performance assertions on hosted runners: current tests complete quickly, runner noise would dominate, and a timing assertion would be flaky.
- Optimizing or rewriting the Rust/TypeScript grid algorithms for the v0 ceiling of 56×56. A full grid has only 3,136 cells; even a 48-entry nearest-colour scan is 150,528 candidates before fixed-cost dithering work.
- DOM virtualization, canvas migration, SIMD, wasm, memoization layers, or stress gates above the supported v0 grid size. They add code and test surface without a demonstrated bottleneck.
- A CI duration SLA or hard bundle-size failure. Collect trends first and set a gate only after a user-visible regression and representative benchmark can define the boundary.

## SH-2 port recommendation

Keep `127.0.0.1:1520` with `strictPort: true`; the Vite config and README already agree, so SH-2 is closed and 1430 should not be restored.

## Implementation acceptance checklist

- Only `.github/workflows/bead.yml` is added by WP-B10; Soul `ci.yml`, root pnpm scripts, root Cargo members, and product code are unchanged.
- `push.branches` is exactly `cursor/bead*`; paths are the two bead trees, root pnpm lockfile, and the workflow itself.
- No `pull_request` trigger; no Soul workflow dispatch from a bead ref.
- Both isolated jobs pass, and the I-7 coexistence commands are recorded.
- No empty commit is used as a runner probe.
