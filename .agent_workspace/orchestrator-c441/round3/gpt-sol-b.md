[Model: gpt-5.6-sol-xhigh-fast]
round: BUILD R3
role: probe
scope:
- Audited `cursor/goal1-build-audit-c441` at `58cc133` (Goal 1 ancestor `5309656`) against AC-01–AC-26 only: crate → `Session` → Tauri IPC → React UI, with emphasis on HITL replay, redirect, injection, forget, and research preview.
- `crates/soul-graph/src/build.rs` still owns the legacy `MODERATE_MIN_INTERACTIONS` / `STRONG_MIN_INTERACTIONS` / active-day thresholds, and this workspace has no `soul-algo-tie` member. That is the already-recorded N6 integration split owned by open [PR #7](https://github.com/Xhhemoing/Soul/pull/7), not a new work package here.
- AC-28…AC-34 are on PR #7's line and are out of scope for this trunk. No T4D/A0 work is proposed here.
done:
- HITL replay is genuinely covered below UI: `session_e1::the_same_approval_twice...` and `ipc_roundtrip::the_same_approval_replayed...` both drive a real loopback endpoint, require one request, reject the replay, and inspect `hitl.deny`.
- Cross-origin redirect is genuinely covered below UI: Session and IPC tests use two real loopback servers and require the redirect target's request count to stay zero.
- Injection reaches Session and IPC in all three channels (paste, both import formats, filename), with chain leakage checks. The caveat is P1-1 below: several IPC cases may early-return green before making those assertions.
- Forget key destruction/reopen and research exclusion are strong in crate/Session tests. The remaining gaps are specifically at the product boundary, not requests to rewrite those crates.
- No current code P0 found.
open:
- P1-1: the IPC acceptance file has 14 `return;` escape hatches after any coded store/import refusal. A command stubbed to always return `{reason_code, explanation}` can therefore leave product-path tests green without exercising the claimed AC.
- P1-2: Session now preserves a pending forget preview after a mismatched confirmation, but IPC and UI do not pin or expose that contract.
- P1-3: AC-20's store test checks file name and byte-length stability, while Session/IPC only compare directory path lists. A product wrapper that writes to the existing SQLCipher DB or WAL remains green.
- P2-1: E1 UI tests do not pin exactly one `generate_draft` call and do not exercise a rejection returned by `generate_draft` (the stage where redirect denial arrives).
- P2-2: Research UI tests never assert that `research_preview` was invoked; a hard-coded safe preview in `core.ts` or the route keeps the UI suite green while disconnecting the product from the store.
p0/p1/p2:
- P0: none.
- P1-1 — false-green IPC success paths.
  - AC: AC-03/04/05/06/08/12/14/15/16/20/23/25.
  - Layer: IPC test harness.
  - Evidence: `apps/desktop/src-tauri/tests/ipc_roundtrip.rs` contains 14 early returns. Examples include the real import→graph path, both hostile import previews, memory CRUD/forget, three-memory CRUD, imported-name redaction, and research preview setup. Each accepts any refusal with a string `reason_code` as a passing alternative.
  - Green mutant: make `commit_soul_import_v1`, `commit_telegram`, or `create_memory` always return a coded refusal. The corresponding acceptance test exits before graph, redaction, forget/reopen, audit, or research assertions.
  - Proposed test command: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test ipc_roundtrip`.
  - Opus scope: test-only edit to `apps/desktop/src-tauri/tests/ipc_roundtrip.rs`; make acceptance preconditions `expect` success. Keep store-unavailable/refusal-shape behavior in separate explicit tests rather than as an alternate pass branch.
- P1-2 — forget retry contract stops at Session.
  - AC: AC-15 plus AC-19's stale/changed-plan refusal behavior.
  - Layer: Session green; IPC stale; UI red to the Session contract.
  - Evidence: `Session::forget_memory` matches before `take` and documents that refusal leaves the preview standing (`session.rs:1141-1158`); `session_screens::a_forget_refused_for_the_wrong_id...` proves wrong preview id, wrong memory id, correct confirmation, then replay. The IPC test still says “The refusal spent the held preview” and calls `preview_forget` again (`ipc_roundtrip.rs:2883-2887`). `Memory.tsx:125-140` clears the displayed preview before the result and does not restore it on refusal. `fakeCore.ts:1129-1145` neither binds both ids nor consumes a successful preview, so UI tests cannot detect the drift.
  - Green mutant: consume the held preview on an IPC mismatch, or invoke `forget_memory` twice from the UI; existing IPC/UI tests still pass.
  - Proposed test commands: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test ipc_roundtrip a_memory_is_written_read_edited_and_forgotten_over_the_ipc -- --exact`; `pnpm --filter @soul/desktop exec vitest run src/routes/Memory.test.tsx`.
  - Opus scope: `ipc_roundtrip.rs`, `Memory.tsx`, `Memory.test.tsx`, and the forget state in `fakeCore.ts`. Pin wrong confirmation → same held preview can still be correctly confirmed; pin one click → exactly one forget call; pin successful confirmation → replay rejected.
- P1-3 — product-level “research writes nothing” only compares filenames.
  - AC: AC-20.
  - Layer: crate strong; Session/IPC weak.
  - Evidence: `soul-store/tests/research_preview.rs:229-250` compares names plus byte lengths after three previews. `session_screens.rs:883-903` and `ipc_roundtrip.rs:3144-3188` collect only `PathBuf`s. A new audit append, DB row, WAL growth, or same-path overwrite in `Session::research` is invisible at both product-facing layers.
  - Green mutant: append a `research.preview`-like audit row in `Session::research`; crate tests remain green and the current Session/IPC “no file” checks still see the same path set.
  - Proposed test commands: `cargo test -p soulcore --test session_screens the_research_preview_writes_nothing_and_the_chain_holds_no_prose -- --exact`; `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test ipc_roundtrip the_research_preview_crosses_the_ipc_as_counts_and_no_third_party_row -- --exact`.
  - Opus scope: test-only hardening in those two test files. Snapshot sorted relative path + length + SHA-256 before and after the product call; do not weaken the store-level exclusion/leakage assertions.
- P2-1 — UI does not pin E1 approval cardinality or generate-stage refusal.
  - AC: AC-11 and AC-19.
  - Layer: UI.
  - Evidence: `Draft.test.tsx` checks `callsTo("generate_draft")[0]` but never requires length 1. `fakeCore.ts:1027-1030` accepts the same approval indefinitely. The only endpoint-refusal UI case throws during `prepare_draft`; no case throws `E1_CROSS_ORIGIN_REDIRECT` from `generate_draft`.
  - Green mutant: call `generateDraft` twice in `approve`; the fake returns two drafts and current UI assertions remain green. The real backend sends once and rejects once, which can leave the user seeing a denial after one request actually ran.
  - Proposed test command: `pnpm --filter @soul/desktop exec vitest run src/routes/Draft.test.tsx`.
  - Opus scope: UI test/fake only unless the test exposes a race. Make prepared approval single-use in the fake, assert exactly one invocation, and assert a generate-stage redirect refusal renders the core code with no draft.
- P2-2 — research screen can be disconnected from IPC without a red test.
  - AC: AC-20.
  - Layer: UI adapter.
  - Evidence: `Research.test.tsx` installs and returns a fake core but has no `callsTo("research_preview")` assertion; `command_surface.rs` checks only that the command name exists, not that the TypeScript wrapper invokes it.
  - Green mutant: return `aResearchPreview()` directly from the wrapper/route. All current research rendering tests and the Rust command inventory remain green.
  - Proposed test command: `pnpm --filter @soul/desktop exec vitest run src/routes/Research.test.tsx`.
  - Opus scope: test-only `Research.test.tsx`; assert one `research_preview` call and no write/export command. Keep the real exclusion/no-disk proof in Rust.
tests:
- PASS: `cargo test -p soulcore --test session_screens a_forget_refused_for_the_wrong_id_leaves_the_preview_the_user_read_standing -- --exact` — 1/1.
- PASS: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test ipc_roundtrip` — 51/51.
- PASS: `pnpm --filter @soul/desktop test -- Memory.test.tsx Draft.test.tsx Research.test.tsx` — Vitest ran the current full 14-file suite, 168/168.
- No mutation was applied because this probe is read-only; the green-mutant statements above are static reachability findings.
assumptions:
- The Linux developer-key path is a valid deterministic acceptance environment: all 51 IPC tests reached their happy paths in this run. A platform-key failure should have its own refusal test, not silently satisfy an AC success test.
- Current one-line Tauri commands are correct delegates; findings target regressions that existing layered tests would not detect.
- Hosted AC-26 and author Win11 AC-01/21/22 evidence remain external blockers and were not reclassified as code defects.
do_not_touch:
- No product file, schema, frozen plan, threshold constant, PR branch, or AC-28+ implementation was changed.
- Do not add `soul-algo-tie` or rewire T4D/A0 on this trunk; N6 stays with PR #7.
- Do not weaken crate tests, add AC-27 execution, merge PR #7/#10, or claim local green is hosted green.
next:
- First dispatch one opus-fast for P1-1 (remove false-green early exits), then one for P1-2 (forget contract across IPC/UI).
- A test-only opus-fast can combine P1-3, P2-1, and P2-2 because paths are disjoint except the IPC test file; serialize that edit after P1-1.
- Re-run full `ipc_roundtrip` and UI suites; keep N4/N5/N6 open and Goal 1 unclosed.
