# Repository Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the highest-value, bounded correctness and performance defects found by the full repository audit without redesigning product contracts.

**Architecture:** Preserve existing crate and UI boundaries. Each task adds a regression test first, applies the smallest local fix, and runs the narrow test plus the relevant lint/build command. Ordinary session edits use the existing transaction primitive; irreversible-operation result redesigns remain follow-up work because they require an explicit product-level result contract.

**Tech Stack:** Rust 1.83, Cargo, React 19, TypeScript, Vite, Vitest, PowerShell 7.

## Global Constraints

- All shell commands use `pwsh`; every script starts with `$ErrorActionPreference = 'Stop'`.
- File reads and writes use UTF-8 explicitly.
- Do not add hashes, frozen contracts, baselines, or new gates.
- Preserve existing safety checks and security boundaries.
- Use targeted edits; do not reformat unrelated files.
- Scope: repository-wide module review, with deeper review and executable regressions for the defects below. The September 27 inventory contains 470 tracked files, including 275 Rust files, 39 TypeScript/TSX files, and 7 PowerShell scripts; inventory and static review are not proof that every execution path is correct.

---

### Task 1: Reject non-success E1 HTTP responses

**Files:**
- Modify: `crates/soul-egress/src/lib.rs:105-120`
- Modify: `crates/soul-egress/tests/e1_origin.rs`

**Interfaces:**
- Consumes: `send(&E1RequestPlan) -> Result<E1Response, EgressError>`
- Produces: `EgressError::HttpStatus { status, body_len }` for every non-2xx response after bounded body reading.

- [x] **Step 1: Write a failing wire-level test**

Add a loopback response test that serves a `429` body containing hostile text, calls `send`, and asserts:

```rust
assert!(matches!(
    error,
    EgressError::HttpStatus { status: 429, body_len } if body_len > 0
));
assert!(!error.to_string().contains("hostile response body"));
```

- [x] **Step 2: Run the test and verify current behavior fails**

Run: `cargo test -p soul-egress --test e1_origin non_success_statuses_are_errors_without_exposing_their_bodies -- --exact`

Expected before fix: the call returns `Ok(E1Response { status: 429, .. })`.

- [x] **Step 3: Return the existing typed error**

After `read_capped`, add:

```rust
if !(200..=299).contains(&status) {
    return Err(EgressError::HttpStatus {
        status,
        body_len: body.len(),
    });
}
```

- [x] **Step 4: Verify the crate**

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

- [x] **Step 1: Write a deferred-read regression test**

Create a `File` whose `text()` returns a controlled promise. Upload it under `soul-import-v1`, switch the radio to Telegram before resolving the read, then resolve it. Assert the stale preview does not become confirmable and no commit command can use a format different from the previewed format.

- [x] **Step 2: Run the focused test and verify it fails**

Run: `pnpm --filter @soul/desktop test -- Import.test.tsx --maxWorkers=2`

Expected before fix: the stale request publishes a preview or commit uses the newly selected radio format.

- [x] **Step 3: Stage format, text, and preview together**

Use a request generation ref. Increment it when forgetting/changing format and before every new file read. Only publish results for the current generation. Store:

```ts
type StagedImport = {
  format: ImportFormat;
  text: string;
  preview: ImportPreview;
};
```

Commit from `staged.format` and `staged.text`. Disable both format radios while `busy`.

- [x] **Step 4: Verify the route and lint**

Run: `pnpm --filter @soul/desktop test -- Import.test.tsx --maxWorkers=2`

Run: `pnpm --filter @soul/desktop lint`

---

### Task 3: Stabilize frontend tests and remove production source maps

**Files:**
- Modify: `apps/desktop/vite.config.ts:17-31`
- Modify: `scripts/gate-win.ps1:166-173`

**Interfaces:**
- Produces: all Vitest entry points use two workers; production `dist` contains no JavaScript source map; the existing Windows gate runs existing lint and test commands before bundling.

- [x] **Step 1: Centralize Vitest worker count**

Add `maxWorkers: 2` to the existing `test` config so root, package, and gate invocations have the measured stable setting.

- [x] **Step 2: Disable production source maps**

Change `build.sourcemap` from `true` to `false`.

- [x] **Step 3: Extend the existing Windows gate**

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

- [x] **Step 4: Verify all frontend paths**

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
- Add: `crates/soul-store/tests/malformed_nonce.rs`.

**Interfaces:**
- Produces: `StoreError::Backend` for malformed wrapped-key nonce length and `StoreError::SealBroken` or `StoreError::Backend` for malformed sealed-blob nonce length; neither path panics.

- [x] **Step 1: Write corruption regression tests**

Open a test store, create valid encrypted rows, mutate `wrap_nonce` and `sealed_blobs.nonce` to a short BLOB through SQL, then invoke the public read/open paths inside normal test execution. Assert an error is returned and the test process does not panic.

- [x] **Step 2: Run the focused tests and verify the panic**

Run: `cargo test -p soul-store --test malformed_nonce --locked --offline -j 2`.

Expected before fix: `XNonce::from_slice` panics on the malformed length.

- [x] **Step 3: Validate lengths before fixed-size conversion**

For both database nonce reads, require `nonce.len() == 24` before calling `XNonce::from_slice`. The wrapping-key error includes the key identifier and observed length without exposing ciphertext or key material. The blob path retains the existing `SealBroken(blob_id)` error shape.

- [x] **Step 4: Verify the store crate**

Run: `cargo test -p soul-store --tests`

Run: `cargo clippy -p soul-store --all-targets -- -D warnings`

---

### Task 5: Report unreadable source during URL audits

**Files:** `crates/xtask/src/egress.rs`, `crates/xtask/tests/self_test.rs`.

- [x] Reproduce the silently skipped UTF-16 source file with a real temporary script.
- [x] Return a contextual UTF-8 read error naming the file; preserve the existing audit and allowlist.
- [x] Pass all 34 xtask self tests and targeted Clippy.

### Task 6: Reject stale collection and people-summary responses

**Files:** `Collect.tsx`, `Collect.test.tsx`, `Graph.tsx`, `Graph.test.tsx` under `apps/desktop/src/routes/`.

- [x] Reproduce old collection success/error overwriting a completed stop and duplicate pending status requests (3 failing tests).
- [x] Add collection request generations, an in-flight read guard, and a synchronous write guard.
- [x] Reproduce summary response reordering and a pre-correction summary reappearing (3 failing tests).
- [x] Invalidate summaries on newer selection, correction, and unmount; disable summary requests during a relationship write.
- [x] Pass independent review, 39 focused tests, and the full frontend suite (205 tests), lint, and build.

### Task 7: Roll back ordinary session edits when their audit fails

**Files:** `crates/soulcore/src/commands/session.rs`, `crates/soulcore/tests/session_atomic_edits.rs`.

- [x] Write four real-SQLCipher regressions using an audit INSERT failure and exact affected-table row comparisons, followed by successful retries.
- [x] Observe the four tests fail before changing production code.
- [x] Wrap `correct_axis`, `set_voice`, `write_memory`, and `edit_memory` in the existing `SqlCipherStore::transact`, including response readback.
- [x] Pass focused tests, relevant crate tests, Clippy, and independent review.

## Completion evidence — September 27, 2026

- `cargo test --workspace --locked --offline -j 2`: 1053 passed, 0 failed, 0 ignored, including the two nonce and four session regressions.
- `cargo clippy --workspace --all-targets --profile test --locked --offline -j 2 -- -D warnings`: passed. The test profile reuses the native dependency build; it is not a release build check.
- Frontend: 205 tests passed; lint and the explicit `NODE_ENV=production` build passed; no source maps in `dist`.
- Independent code review found no blocking issue in the import, collection, summary, scanner, nonce, and transaction changes.
- See the [results and remaining risks](../../2026-09-27_code-review-soul-report.md). The desktop Rust workspace, full Windows release gate, and installation smoke workflow were not run in this task.

## Deferred Findings and Deliberate Product Limits

- Collection consent revocation can race with the final store write.
- Re-importing the same source duplicates events/evidence by current documented v0.1 design; preserve that behavior rather than introducing a new hash or uniqueness contract.
- Post-forget audit/checkpoint failures need explicit committed-with-warning result types. This does not apply to the ordinary, reversible session edits in Task 7.
- WAL checkpoint `busy` status is currently discarded after irreversible forget commits.
- Graph rebuild performs O(P²) existing-row matching and should be pre-indexed with a performance regression fixture.

These are not silently dismissed. Revocation and forgetting require concurrency/result semantics; graph indexing needs measurement and behavior-preserving fixtures. They are not claimed fixed by this patch.
