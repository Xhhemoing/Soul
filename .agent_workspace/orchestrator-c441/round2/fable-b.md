# fable-b — BUILD/AUDIT R2 defect list (READONLY)

- **round**: BUILD/AUDIT R2
- **role**: review (independent auditor; did not read fable-a's round-2 findings)
- **scope**: races/concurrency, memory safety (soul-win-dpapi unsafe), extreme edges, auth/data path, copy-vs-behavior. Trunk = `cursor/soul-goal1-7b1c` @ `5309656`; branch = `cursor/goal1-build-audit-c441`. Fixes landed on the branch mid-round are noted per defect.

## done

- Read PLAN_INDEX, STATUS, PRODUCT_LOCK, DECISIONS, SECURITY, FORMAL redlines 1–12, algorithms/DECISION.md.
- Full read of `soul-win-dpapi` (`sys.rs`, `lib.rs`), `soul-collect` (`windows.rs`, `collector.rs`, `runner.rs`), `soul-store` (`keys.rs`, `forget.rs`), `soulcore/commands` (`session.rs`, `policy.rs`, `draft.rs`, `fileplan.rs`), `soul-policy` (`hitl.rs`, `net_guard.rs`), `soul-egress`, `soul-fileplan` (`screen.rs`, `authorize.rs`, `scan.rs`), `soul-import/commit.rs`, `soul-graph/build.rs`, desktop shell (`instance.rs`, `commands.rs`, `lib.rs`), UI routes (`Files.tsx`, `Import.tsx`, `Draft.tsx`).
- **Clean findings (no defect)**: `soul-win-dpapi/src/sys.rs` unsafe blocks are sound — `DATA_BLOB` in/out handling, `GetLastError` captured immediately after each failed call, `LocalFree` on both success and error paths, output copied then zeroed, non-Windows answers `Unsupported` instead of lying. `soul-collect/src/windows.rs` FFI likewise sound (bounded buffer, handle closed on every path). `screen.rs` path screening is thorough (`\\?\` prefixes, ADS, device names, Explorer quotes). HITL token single-use/TTL/scope/plan-hash checks in `hitl.rs` are correct. Collector re-checks consent both before sampling and before each store write (AC-09/AC-10 honored).

## defects

### P0

**P0-1 — Files empty state claims Soul reads no file at all before directory auth.**
- Path: `apps/desktop/src/routes/Files.tsx` (trunk `5309656` line 123): 「授权之前，Soul 读不到你机器上的任何文件」.
- Repro: fresh install, open Files with no authorized roots. The claim is false: the Import page reads a user-picked file with no directory auth, and Soul always reads its own config and database. Copy-vs-behavior lie on the trust-critical screen.
- Status: **confirmed at trunk; already closed on this branch** by `1f52ca5` (HEAD line 129 now scopes the promise to the directory scanner and names the exceptions). Owner: opus-a. Not touched by me.

### P1

**P1-1 — E1 plan hash does not bind the target endpoint; a pending approval survives an endpoint change.**
- Path (trunk `5309656`): `crates/soulcore/src/commands/policy.rs:197,236` — `e1_plan(model, &body)` has no origin field; `set_user_endpoint` (`policy.rs:82`) swaps the `NetGuard` without invalidating the pending draft held in `draft.rs`.
- Repro: prepare a draft (token issued against plan P), then change the E1 endpoint in Settings, then call generate with the earlier confirmation. Token verifies (plan hash unchanged), body — including redaction-exempted original text — is POSTed to the *new* endpoint under an approval that described the old one. Violates SECURITY.md "config changes invalidate the plan hash". The desktop UI loses the Confirm panel on navigation, which accidentally masks this, but the core contract is broken (headless, or Settings opened without leaving Draft).
- Status: **open at trunk; closed on this branch mid-round** by `d1b6457` (plan now carries the origin it was described against) + `24ea578`. Verify at close-out that a pending plan is also *cleared or invalidated* on `set_user_endpoint`/`clear_user_endpoint`, not merely hash-mismatched at consume time.

**P1-2 — Forgetting a label-less contact is a silent no-op; sealed message bodies stay decryptable.**
- Path: `crates/soul-store/src/forget.rs:76-88` — `ForgetUnit::Contact` collects only `contacts.display_label_key_id`; empty result short-circuits to `ForgetPlan::default()`. `crates/soul-import/src/commit.rs:301-316` — a contact without a `display_label` never gets `seal_label` called, so its fresh `content_key_id` (minted per import, `commit.rs:301-308`) is recorded nowhere in `contacts`; bodies are sealed under it (`commit.rs:158-172`).
- Repro: import a `soul-import-v1` file whose participant has no `display_label`; forget that contact. Preview impact shows zeros, receipt reads as completed, audit logs a forget — but every `sealed_blobs` row for their messages survives and the DEK still decrypts them. Breaks the forget-by-key-destruction contract in SECURITY.md/PRODUCT_LOCK; the receipt is the copy-vs-behavior lie.
- Status: **open at HEAD**. Compounded by P1-3: each re-import mints another orphan key for the same label-less contact.

**P1-3 — Re-import silently duplicates events and inflates tie strength; nothing warns the user.**
- Path: `crates/soul-import/src/commit.rs:22-23` (documented: same file committed twice writes events twice, no external-id index); `crates/soul-graph/src/build.rs` tallies every duplicate as a real interaction; `apps/desktop/src/routes/Import.tsx` offers no dedup warning or "already imported" signal.
- Repro: import the same export file 3–5 times, rebuild the graph. `interaction_count` multiplies; a contact legitimately below the MODERATE/STRONG thresholds crosses them on duplicate evidence alone. The graph then misrepresents relationship strength — a data-integrity failure in the product's core claim ("what the graph shows is what your history supports"). Not asking for a dedup index rewrite (v0.1 scoping decision stands); minimum honest fix is an import-receipt warning + Files/Import copy that re-import double-counts, or a same-file-hash guard.
- Status: **open at HEAD**.

**P1-4 — `DirectorySnapshot::of` ignores `max_entries`; huge directory freezes the whole IPC surface.**
- Path: `crates/soul-fileplan/src/scan.rs:165-210` — snapshot walk bounds only on `max_depth` (line 207); the plan walk enforces `max_entries` (line 365) but the before/after snapshots do not.
- Repro: authorize a root with a very large flat directory (hundreds of thousands of entries within depth limit, e.g. a node_modules or photo dump); request a file plan. Both snapshots accumulate an unbounded `Vec<String>` while the single `Session` mutex is held — GUI-wide freeze, unbounded memory, and the disk-unchanged comparison doubles the cost. Contradicts the scan module's own stated invariant that walks are bounded by limits.
- Status: **open at HEAD**.

### P2

**P2-1 — One `Mutex<Session>` plus 120 s egress timeout can block consent revocation for ~2 minutes.**
- Path: `apps/desktop/src-tauri/src/commands.rs:40` (single `SessionState(Mutex<Session>)`, every IPC command serializes on it); `crates/soul-egress/src/lib.rs:39,42` (`REQUEST_TIMEOUT` 120 s, `CONNECT_TIMEOUT` 10 s, blocking client).
- Repro: point the E1 endpoint at a server that accepts the connection and stalls; click generate; then try `revoke_collect_consent` (or anything else). The revoke queues behind the stalled request for up to ~130 s while the collector keeps sampling, because the consent flag only flips when the queued command finally runs. Bounded, so P2 not P1 — but it is the user's *stop collecting* button that hangs.
- Status: open at HEAD.

**P2-2 — TOCTOU: a directory can be swapped for a symlink between the queue-time check and `read_dir`.**
- Path: `crates/soul-fileplan/src/scan.rs:171,207-209` (snapshot) and `:347,433-436` (plan walk) — dir-ness/symlink-ness is decided from `symlink_metadata` when the child is queued; `std::fs::read_dir` on it happens a later loop iteration.
- Repro: attacker with write access inside an authorized root replaces a queued subdirectory with a symlink to an outside directory in the race window; `read_dir` follows it and outside entries appear in the snapshot/plan. Requires local write access inside the root, hence P2; but the module doc claims categorical no-follow, so either close the race (Windows: open handle with reparse-point rejection, then enumerate by handle) or soften the claim.
- Status: open at HEAD.

**P2-3 — NTFS case folding is not Unicode `to_lowercase`; `CaseFolded` can mis-scope authorization on edge names.**
- Path: `crates/soul-fileplan/src/authorize.rs:66` — `left.to_lowercase() == right.to_lowercase()` (full Unicode, Rust-version-dependent) vs NTFS's per-volume `$UpCase` table (UTF-16 code-unit uppercasing, frozen at format time).
- Repro (extreme edge): a directory containing both `K.txt` (U+212A KELVIN SIGN) and `k.txt` — distinct files to NTFS, but the same segment to Soul, so an authorization/plan for one can be attributed to the other; the reverse class (pairs NTFS folds but Rust does not) makes a genuinely-inside path look outside. Affects AC-18-adjacent identity of "which on-disk file did the user authorize".
- Status: open at HEAD. Windows-only, exotic names only.

**P2-4 — First-run key-blob race: two processes can mint different DEKs; the loser's database becomes permanently unopenable.**
- Path: `crates/soul-store/src/keys.rs:327-349` — `create_blob` writes `.partial` then renames over `keys.dpapi` unconditionally (no `create_new` guard, no existence re-check at rename). `crates/soulcore/src/commands/session.rs` `Session::open` takes no data-dir lock, and `apps/desktop/src-tauri/src/instance.rs:106-111` documents that off Windows every launch is "the first one".
- Repro: two processes (GUI + `soul-headless`, or any non-Windows pair) start on a virgin data dir concurrently. Both mint distinct KEK/DEK; last rename wins. If the losing process already created the SQLCipher database with its DEK, the surviving blob cannot open it — permanent lockout of encrypted data with no error until next launch. Windows GUI-only is protected by the named mutex; every other pairing is not.
- Status: open at HEAD. Cheap fix shape: `OpenOptions::create_new` on the final name (fall back to re-read on `AlreadyExists`), or a data-dir lockfile in `Session::open`.

**P2-5 — A mismatched forget confirmation destroys the pending preview, contrary to its own contract.**
- Path: `crates/soulcore/src/commands/session.rs:1132-1135` — `self.held_forget.take().filter(...)`: the held preview is consumed *before* the match check, so a stale or typo'd `preview_id`/`memory_id` drops a legitimate pending confirmation. The field's doc (lines 388-390) says "taken by value **when the forget runs**" — it is also taken when the forget is refused.
- Repro: preview a forget; send a confirmation with a wrong preview_id (refused); resend the correct one — also refused, user must re-preview. Defensible as anti-guessing, but then the docstring and the refusal copy should say so; today code and contract disagree.
- Status: open at HEAD. Borderline P3; kept P2 because forget is the product's most trust-sensitive flow.

## open

- P1-2, P1-3, P1-4 and all P2s are unaddressed at branch HEAD as of this writing (`d1b6457`).
- P1-1 fix landed mid-audit; needs a close-out check that endpoint *changes* clear/invalidate the pending draft rather than only failing at consume.
- Not verified (out of readonly reach): actual NTFS `$UpCase` behavior for P2-3 (reasoned from spec, not tested on a Windows volume); P2-2 race window timing under real Windows filesystem semantics.

## assumptions

- The audit target is trunk `5309656`; the branch is a moving target (fixes `1f52ca5`, `24ea578`, `d1b6457` landed during this round) and each defect carries its status at both points.
- Independence: I did not open `round2/fable-a.md`, `gpt-sol-a.md`, or `gpt-sol-b.md`; overlap with their findings is uncoordinated.
- "No second threshold literals" honored: P1-3 references the existing MODERATE/STRONG constants in `soul-graph/build.rs` without restating their values.

## do_not_touch

- `apps/desktop/src/routes/Files.tsx`, `Files.test.tsx` (opus-a owns; P0-1 already fixed there).
- Goal 2, AC-27, PR #7, completed WPs, threshold literals — none touched; no product file edited; this file is my only write.

## next

1. Fix P1-2 first (label-less contact forget no-op): record the body `content_key_id` on the contact row (or a contact→key table) at import, and make `ForgetUnit::Contact` collect it; add a test that a label-less contact's sealed bodies die with the forget.
2. P1-3: same-file re-import guard (file hash on the receipt) or honest double-count warning in Import copy + receipt.
3. P1-4: enforce `max_entries` in `DirectorySnapshot::of` and mark the snapshot truncated, mirroring the plan walk.
4. Batch the P2s: `create_new` on the key blob + data-dir lock (P2-4), snapshot/plan TOCTOU doc-soften or handle-based enumeration (P2-2), match-before-take in `forget_memory` (P2-5), revoke-path responsiveness note or lock split (P2-1), NTFS fold caveat comment (P2-3).
5. Close-out check on P1-1: endpoint change should clear the pending draft, not only mismatch at consume.
