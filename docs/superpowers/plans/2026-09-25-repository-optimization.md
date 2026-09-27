# Repository Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the highest-value, bounded correctness and performance defects found by the full repository audit without redesigning product contracts.

**Architecture:** Preserve existing crate and UI boundaries. Each task adds a regression test first, applies the smallest local fix, and runs the narrow test plus the relevant lint/build command. Transactional and irreversible-operation redesigns remain documented follow-up work because they require an explicit product-level result contract.

**Tech Stack:** Rust 1.83, Cargo, React 18, TypeScript, Vite, Vitest, PowerShell 7.

## Global Constraints

- All shell commands use `pwsh`; every script starts with `$ErrorActionPreference = 'Stop'`.
- File reads and writes use UTF-8 explicitly.
- Do not add hashes, frozen contracts, baselines, or new gates.
- Preserve existing safety checks and security boundaries.
- Use targeted edits; do not reformat unrelated files.
- Scope coverage: all 323 tracked code files were inventoried; 268 Rust files and 38 TypeScript/TSX files received static review, with deeper L3 review on the defects below.

---

### Task 1: Reject non-success E1 HTTP responses

**Files:**
- Modify: `crates/soul-egress/src/lib.rs:105-120`
- Modify: `crates/soul-egress/tests/e1_origin.rs`

**Interfaces:**
- Consumes: `send(&E1RequestPlan) -> Result<E1Response, EgressError>`
- Produces: `EgressError::HttpStatus { status, body_len }` for every non-2xx response after bounded body reading.

- [ ] **Step 1: Write a failing wire-level test**

Add a loopback response test that serves a `429` body containing hostile text, calls `send`, and asserts:

```rust
assert!(matches!(
    error,
    EgressError::HttpStatus { status: 429, body_len } if body_len > 0
));
assert!(!error.to_string().contains("hostile response body"));
```

- [ ] **Step 2: Run the test and verify current behavior fails**

Run: `cargo test -p soul-egress --test e1_origin non_success_status_is_an_error -- --exact`

Expected before fix: the call returns `Ok(E1Response { status: 429, .. })`.

- [ ] **Step 3: Return the existing typed error**

After `read_capped`, add:

```rust
if !(200..=299).contains(&status) {
    return Err(EgressError::HttpStatus {
        status,
        body_len: body.len(),
    });
}
```

- [ ] **Step 4: Verify the crate**

Run: `cargo test -p soul-egress --test e1_origin`

Run: `cargo clippy -p soul-egress --all-targets -- -D warnings`

---

### Task 2: Make import preview and commit an atomic staged choice

**Files:**
- Modify: `apps/desktop/src/routes/Import.tsx:82-168`
- Modify: `apps/desktop/src/routes/Import.test.tsx`

**Interfaces:**
- Consumes: preview and commit commands selected by `ImportFormat`.
- Produces: one staged object containing the exact `format`, `text`, and `preview` confirmed by the user.

- [ ] **Step 1: Write a deferred-read regression test**

Create a `File` whose `text()` returns a controlled promise. Upload it under `soul-import-v1`, switch the radio to Telegram before resolving the read, then resolve it. Assert the stale preview does not become confirmable and no commit command can use a format different from the previewed format.

- [ ] **Step 2: Run the focused test and verify it fails**

Run: `pnpm --filter @soul/desktop test -- Import.test.tsx --maxWorkers=2`

Expected before fix: the stale request publishes a preview or commit uses the newly selected radio format.

- [ ] **Step 3: Stage format, text, and preview together**

Use a request generation ref. Increment it when forgetting/changing format and before every new file read. Only publish results for the current generation. Store:

```ts
type StagedImport = {
  format: ImportFormat;
  text: string;
  preview: ImportPreview;
};
```

Commit from `staged.format` and `staged.text`. Disable both format radios while `busy`.

- [ ] **Step 4: Verify the route and lint**

Run: `pnpm --filter @soul/desktop test -- Import.test.tsx --maxWorkers=2`

Run: `pnpm --filter @soul/desktop lint`

---

### Task 3: Stabilize frontend tests and remove production source maps

**Files:**
- Modify: `apps/desktop/vite.config.ts:17-31`
- Modify: `scripts/gate-win.ps1:166-173`

**Interfaces:**
- Produces: all Vitest entry points use two workers; production `dist` contains no JavaScript source map; the existing Windows gate runs existing lint and test commands before bundling.

- [ ] **Step 1: Centralize Vitest worker count**

Add `maxWorkers: 2` to the existing `test` config so root, package, and gate invocations have the measured stable setting.

- [ ] **Step 2: Disable production source maps**

Change `build.sourcemap` from `true` to `false`.

- [ ] **Step 3: Extend the existing Windows gate**

After install and before bundle, add existing checks:

```powershell
Step 'frontend lint' {
    pnpm --filter '@soul/desktop' lint
}

Step 'frontend tests' {
    pnpm --filter '@soul/desktop' test
}
```

This extends G-W; it does not create a new gate.

- [ ] **Step 4: Verify all frontend paths**

Run: `pnpm --filter @soul/desktop test`

Run: `pnpm --filter @soul/desktop lint`

Run: `pnpm --filter @soul/desktop build`

Confirm: `Get-ChildItem -LiteralPath apps/desktop/dist -Recurse -Filter '*.map'` returns no files.

Parse the PowerShell script with `System.Management.Automation.Language.Parser::ParseFile` and require zero parse errors.

---

### Task 4: Turn malformed database nonces into typed store errors

**Files:**
- Modify: `crates/soul-store/src/store.rs:326-365`
- Modify: `crates/soul-store/src/store.rs:1167-1195`
- Modify: the closest existing `crates/soul-store/tests/*.rs` store corruption test file.

**Interfaces:**
- Produces: `StoreError::Backend` for malformed wrapped-key nonce length and `StoreError::SealBroken` or `StoreError::Backend` for malformed sealed-blob nonce length; neither path panics.

- [ ] **Step 1: Write corruption regression tests**

Open a test store, create valid encrypted rows, mutate `wrap_nonce` and `sealed_blobs.nonce` to a short BLOB through SQL, then invoke the public read/open paths inside normal test execution. Assert an error is returned and the test process does not panic.

- [ ] **Step 2: Run the focused tests and verify the panic**

Run the exact new tests with `cargo test -p soul-store <test-name> -- --exact`.

Expected before fix: `XNonce::from_slice` panics on the malformed length.

- [ ] **Step 3: Validate lengths before fixed-size conversion**

For both database nonce reads, require `nonce.len() == 24` before calling `XNonce::from_slice`. Return an error that names the affected key/blob identifier and observed length without exposing ciphertext or key material.

- [ ] **Step 4: Verify the store crate**

Run: `cargo test -p soul-store --tests`

Run: `cargo clippy -p soul-store --all-targets -- -D warnings`

---

## Deferred Findings Requiring Contract Design

- Collection consent revocation can race with the final store write.
- Re-importing the same source duplicates events/evidence by current documented design.
- Memory/profile multi-write operations and post-commit audit/checkpoint failures need explicit committed-with-warning result types.
- WAL checkpoint `busy` status is currently discarded after irreversible forget commits.
- Graph rebuild performs O(P²) existing-row matching and should be pre-indexed with a performance regression fixture.
- `xtask e0-audit` should explicitly handle UTF-16 scripts instead of silently skipping decode failures.

These are not silently dismissed; they are excluded from this patch because changing them without a reviewed state/result contract could make irreversible behavior less clear.
