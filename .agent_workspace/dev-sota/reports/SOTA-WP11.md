MODEL: claude-fable-5-thinking-xhigh

# SOTA review — WP11 read-only file plan (`soul-fileplan` + `soulcore::commands::fileplan`)

Scope: SOTA_BARS F-01..F-06, TASK_SPLIT ST-02, PRODUCT_LOCK slice 9, D31 (execute is v0.1.1).
Everything under `crates/soul-fileplan/{src,tests}`, `crates/soulcore/src/commands/fileplan.rs`,
`crates/soulcore/tests/fileplan_commands.rs`, `fixtures/fileplan/**` was read in full; store/shell
wiring (`store.rs`, `shell.rs::authorize_root`) was skimmed for AC-18 consistency. Test evidence was
re-run during this review: `cargo test -p soul-fileplan --all-targets` → 27 passed,
`cargo test -p soulcore --test fileplan_commands` → 6 passed, `cargo test -p soul-policy --test hitl`
→ 15 passed. Commit `eb74904` touches neither `soul-policy/src/hitl.rs` nor
`soul-policy/tests/hitl.rs` (F-05.3's "don't touch" clause holds).

## 1. Verdict

**ship-with-must-fix.**

The implementation itself is clean: authorization is canonicalize-then-`Path::starts_with`
(component-wise), roots are canonicalized once at construction behind a newtype, empty roots refuse
everything, refusals never echo the requested path, the scanner follows no links and opens no file
contents, `written_to_disk` is a literal `false` behind private fields with `plan()` as the only
constructor, the plan hash is taken over the same JSON the render derives from, entries are
explicitly sorted with the sortedness asserted directly (not just repeatability), and the audit
chain carries a scan UUIDv7 plus a count and provably nothing else. The two must-fixes are
test-coverage gaps against the letter of the bars, not code defects.

## 2. Bar table

| Bar | Grade | Proving test(s) |
|---|---|---|
| F-01 disk unchanged, reverse-anchored | **partial** | `authorized_scan.rs::scan_and_preview_leave_the_tree_byte_for_byte_unchanged` (3 repeats, recursive content-hash snapshot, no mtime, non-empty preview referencing real files) + `authorized_root_scans_real_entries`. **Gap:** the snapshotted fixture has no zero-byte file, no large file, no Chinese/emoji/over-long names, one nesting level — see MF-1 |
| F-02 unauthorized 100% refused | **pass** | `unauthorized_is_refused.rs` — all ten: `empty_roots_refuse_everything`, `a_path_outside_every_root_is_refused`, `dotdot_traversal_out_of_the_root_is_refused`, `a_symlink_escaping_the_root_is_refused` (unix; Windows junction on manual list per the bar's own allowance), `a_parent_directory_of_the_root_is_refused`, `a_sibling_sharing_the_root_prefix_is_refused` (`/data/a` vs `/data/ab`), `a_case_or_nfd_variant_outside_the_root_is_refused`, `refusal_names_no_requested_path`, `the_refused_tree_is_untouched`, plus the legal-path control `an_authorized_request_still_passes` |
| F-03 source-level no write API | **pass** | `no_write_api.rs::production_source_has_no_write_surface` (runtime `src/` enumeration, 22-needle write vocabulary) + anchor `the_scanner_saw_the_real_modules` + control `the_scanner_recognises_a_synthetic_write_call` + `no_execute_or_apply_entry_point_exists` + `no_token_machinery_appears_in_the_source`. Hardening gaps in SF-1 |
| F-04 filenames are data | **partial** | `filename_injection.rs` (portable subset created and listed verbatim, one suggestion per file max, `UntrustedText`/`ExternalChannel::FileName`, URLs refused by `NetGuard::closed()`) + `fileplan_commands.rs::external_content_cannot_request_a_scan_or_plan` + `the_audit_chain_carries_no_file_name`. **MISSING:** the `ActionKind::ALL` × `RequestOrigin::ExternalContent` loop over the filename corpus (bar F-04.2) — see MF-2; decoy zero-count and chain-corpus letter in SF-2/SF-3 |
| F-05 no FileWrite token | **pass** | `fileplan_commands.rs::a_file_write_token_buys_no_execution` (`issue_token(FileWrite)` → `ReasonCode::WriteNotImplemented`; `issued_count() == 0` after both actions run through `check_action`) + `no_write_api.rs::no_token_machinery_appears_in_the_source` + untouched `soul-policy/tests/hitl.rs::a_file_write_token_is_refused_even_when_it_is_otherwise_perfect` and `no_known_action_asks_for_the_file_write_scope`; `needs_capability_token() == false` re-pinned for both actions |
| F-06 plan hash pins the plan | **pass** | `authorized_scan.rs::the_same_tree_scanned_twice_yields_the_same_plan_hash` (sortedness asserted directly; same tree in another directory hashes differently; `scan_id` provably outside the hash) + `preview_is_derived_from_the_real_scan` (field-by-field JSON↔entries fidelity, render and JSON share one entry list) + `changing_a_real_file_changes_the_plan_hash` + `fileplan_commands.rs::an_edited_plan_fails_the_approved_hash` (retargeted and reordered edits both `PlanHashMismatch`, approved-as-is control passes, only the approved hash lands in the chain) |

## 3. Must-fix findings

### MF-1 (F-01) — the byte-for-byte snapshot never covers the fixture shapes the bar names

The bar's fixture list is "多层子目录、中文/emoji/超长文件名、零字节文件、大文件".
`fixtures/fileplan/sample_tree.json` is eight all-ASCII entries, every file small and non-empty,
deepest nesting one level (`inbox/budget.csv`). Chinese names do appear elsewhere
(`fileplan_commands.rs::build_tree`, the injection corpus's portable subset), but neither of those
trees is ever snapshot-compared before/after — the disk-unchanged proof only ever runs over the
narrow sample tree. Worse, the zero-byte case is structurally locked out:

```58:59:crates/soul-fileplan/tests/authorized_scan.rs
                assert!(entry.bytes() > 0, "the fixture writes real content");
```

Smallest patch (fixture + one assertion + one line of test setup; no production code):

1. `fixtures/fileplan/sample_tree.json`: add `{"relative_path": "inbox/空文件.txt", "kind": "file",
   "content_utf8": ""}` (zero-byte + Chinese), `{"relative_path": "inbox/archive", "kind":
   "directory"}` + `{"relative_path": "inbox/archive/old-📎.csv", ...}` (second nesting level +
   emoji), and a ~120-char ASCII long name at top level.
2. `authorized_scan.rs`: replace the blanket `entry.bytes() > 0` with a comparison against the
   fixture's own `content_utf8` byte length (the `SampleTree` struct already carries it), which is
   a stronger assertion anyway.
3. In `scan_and_preview_leave_the_tree_byte_for_byte_unchanged`, before the first snapshot:
   `common::write_file(&space.authorized.join("large.bin.txt"), &"x".repeat(1 << 20));` — the
   1 MiB file rides through the existing hash comparison for free.

### MF-2 (F-04.2) — no `ActionKind::ALL` × `ExternalContent` matrix on the filename channel

Bar F-04.2 requires the same all-actions refusal loop that `soul-import/tests/injection_is_data.rs`
runs (lines 144–165 there loop `ActionKind::ALL` per corpus line). For WP11, only two kinds are
covered, via `fileplan_commands.rs:203-247` (`ScanDirectory`, `PlanFiles`). The refusal is
origin-based and structurally kind-independent (`hitl.rs:461-463` fires before any kind-specific
logic), so this is a pinning gap, not a behavior gap — but the bar asks for the pin per channel and
the patch is ~15 lines.

Smallest patch: in `crates/soul-fileplan/tests/filename_injection.rs` (soul-policy is already a
dependency) add:

```rust
#[test]
fn no_filename_can_authorize_any_action() {
    let mut issuer = soul_policy::hitl::TokenIssuer::new();
    for line in corpus() {
        let _name = UntrustedText::new(line); // the only type a name may become
        for kind in soul_policy::hitl::ActionKind::ALL {
            let request = soul_policy::hitl::ActionRequest::new(
                kind.as_str(), soul_policy::hitl::RequestOrigin::ExternalContent);
            let denial = soul_policy::hitl::check_action(&mut issuer, &request, 1_700_000_000_000)
                .expect_err("a file name is data");
            assert_eq!(denial.reason_code(), soul_policy::ReasonCode::ExternalContentNotAuthority);
        }
    }
    assert_eq!(issuer.issued_count(), 0);
}
```

## 4. Should-fix polish

1. **`no_write_api.rs` brace-import and `soft_link` evasions.** `fs::rename`/`fs::copy` are
   deliberately qualified (justified: bare `rename` hits `PlanAction::Rename`, bare `copy` hits
   `Copy` bounds), but `use std::fs::{self, rename};` + bare `rename(a, b)` slips past every
   needle, as does deprecated `std::fs::soft_link` and `use std::io::prelude::*`. `src/` today
   contains no `use std::fs` or `use std::io` at all (verified), so adding three needles —
   `"use std::fs"`, `"use std::io"`, `"soft_link"` — forces fully-qualified `std::fs::…` forms
   that the existing qualified needles then catch. Consider `"std::net"` and `"process::Command"`
   too: a scanner has no business with sockets or subprocesses, and the e0-audit dependency walk
   cannot see std.
2. **The decoy in `filename_injection.rs:117-147` can never be hit**, since every corpus URL points
   at `evil.example`; its `request_count() == 0` is decorative (the test's own comment admits it).
   Give it teeth: push `format!("get {} now.txt", decoy.chat_completions_url())` through the same
   `UntrustedText` → `urls_in` → `NetGuard::closed()` path and assert the loopback URL is refused
   too — then the zero count rules out the one server that was actually reachable.
3. **F-04.5's letter**: the audit-chain `LeakageChecker` corpus in
   `fileplan_commands.rs::the_audit_chain_carries_no_file_name` is the people-named tree (a strong
   corpus), but the bar says "以全部注入文件名为语料". Add two or three instruction-shaped names
   from `fixtures/injection/filenames.txt` to `build_tree` so the injection corpus itself is proven
   absent from a store-backed chain.
4. **`PolicySession` hides its ledger**, so `a_file_write_token_buys_no_execution` asserts
   `issued_count() == 0` on a parallel `TokenIssuer` running the same `check_action` shapes rather
   than on the session that ran the flow. A read-only `PolicySession::issued_count()` getter would
   let the test assert the actual session ledger stayed empty (F-05.2's letter).
5. **`Denied` + `ReasonCode::Routine`** is what a `NotADirectory`/`RootUnreadable` refusal writes
   to the chain (`error.rs:40-47` + `fileplan.rs:65`). The split is documented (disk facts are not
   policy refusals), but a denied row whose reason reads "ROUTINE" is odd chain vocabulary; worth a
   deliberate second look before the reason-code set fossilizes.

## 5. Non-issues (checked hostile, found clean)

- **`starts_with`-as-string:** absent. `authorize.rs:91-94` uses `Path::starts_with`
  (component-wise) on a canonicalized target against canonicalized roots, and
  `a_sibling_sharing_the_root_prefix_is_refused` proves `/data/ab` refuses under authorized
  `/data/a`. ST-02's mutation table shows the string-prefix mutant goes red.
- **Write APIs "for later":** none. No `apply`/`execute`/`undo` anywhere (fn-name scan, not
  substring, so doc prose doesn't false-positive), no `tempfile` in production, no dry-run
  rehearsal. The one "write-shaped" thing, `format!`-based error rendering (`error.rs:56-65`), is
  documented as the deliberate alternative to sink-macros the scanner forbids.
- **Hashing Debug:** absent. `PlanHash::of` takes the canonical JSON; paths are encoded by
  explicit component-join with `/` (`plan.rs:295-301`), so the same tree hashes identically across
  separator conventions. serde_json's `preserve_order` is enabled nowhere in `Cargo.lock`
  (serde_json 1.0.151 depends only on itoa/memchr/serde/serde_core/ryu), so `Value`'s BTreeMap
  key-ordering assumption in `PlanHash::of` holds.
- **Unsorted plan entries:** `scan.rs:206` and `plan.rs:235` both sort, and — the part naive suites
  miss — `the_same_tree_scanned_twice_yields_the_same_plan_hash:222-232` asserts sortedness
  directly, because within one process mere repeatability would mask a missing sort. `scan_id` is
  deliberately outside the hash so a re-scan of an unchanged tree keeps its approval (the
  refuse-in-the-wrong-direction trap avoided).
- **mtime in disk-unchanged snapshots:** absent by design. `common::Shot` is path/kind/bytes/
  content-digest, with the reason written down; the fixture README was corrected from its earlier
  "compare mtime" wording. This matches the bar's explicit instruction.
- **`written_to_disk` structurally false:** not a field, no public constructor, no setter; literal
  `false` (`plan.rs:96-99`) and hardcoded `false` in the hashed JSON. A caller cannot be persuaded.
- **Path digest in plan JSON, scan UUIDv7 in the chain:** this is the intentional split the product
  lock blesses. The chain is asserted (positively) not to contain the `root_fingerprint` literal,
  and `subject_refs` carries `Uuid::now_v7()`, which is what `audit.schema.json` demands. The
  `plan_hash` that does land in the chain is frozen-schema AC-19 machinery; confirming a guess
  through it requires already knowing the entire plan, tree and root included.
- **Escaping and dangling symlinks:** `follow_links(false)` plus per-entry target resolution
  (`scan.rs:188-191`, `219-223`); a link that cannot be resolved is skipped for the same reason as
  one that escapes ("cannot be shown inside" refuses). The remaining check-then-stat gap on a
  swapped link leaks at most a byte count — the scanner never opens file contents at all.
- **Tests write to disk:** they must (tree builders); `no_write_api.rs` scans `src/` only and says
  why. Consistent with F-03's stated scope and the `soul-collect` precedent.
- **No `soul-egress`/reqwest/tokio:** confirmed in `soul-fileplan/Cargo.toml`; roots arrive as
  arguments, no config, no store handle in the pure crate.
- **AC-18 wiring consistency:** `shell.rs::authorize_root` canonicalizes before storing;
  `AuthorizedRoots::canonicalized` re-canonicalizes (idempotent) and fails closed if the root
  vanished in between. No desktop IPC command invokes `scan_directory` yet — the files page is
  deliberately `Pending` (functional views are P1 per TASK_SPLIT §4), so this is tracked scope,
  not a WP11 defect.

## 6. Windows-only gaps for the manual list

The unix symlink test correctly leaves an `#[ignore]`d `cfg(not(unix))` placeholder pointing at the
manual list; ST-02.md §8 already carries items 1–3 below. Consolidated list:

1. **Junction escape** (`mklink /J A\outside B`): verify std/walkdir report the junction as a
   symlink so `stays_inside` fires; naming the junction as the scan target must refuse; scanning
   `A` must give `skipped_escaping_links == 1` with zero traces of `B`.
2. **`\\?\` verbatim prefix display:** `fs::canonicalize` on Windows returns `\\?\C:\...`. The
   roots echoed in `FilePlanError::PathNotAuthorized` and in `shell::authorized_roots()` will carry
   the prefix — verify the settings/preview surfaces either tolerate or strip it for display, and
   that `Path::starts_with` stays consistent when one side is `\\?\UNC\server\share`.
3. **NTFS case-insensitivity:** `C:\DATA\A` for authorized `C:\data\a` canonicalizes to the on-disk
   casing and is allowed — the documented "same directory" decision in
   `a_case_or_nfd_variant_outside_the_root_is_refused` needs one real-machine confirmation.
4. **MAX_PATH:** ~200-char corpus names under a deep temp directory can exceed 260 chars without
   long-path opt-in; the test's `fs::write` uses non-verbatim paths and silently skips failures
   (`is_ok()` filter), so verify `MIN_CREATED_FILES` still clears on Windows.
5. **Reserved device names created via verbatim paths:** files literally named `CON`/`NUL` can
   exist on NTFS when created through `\\?\`; the portable filter excludes them from the Linux
   test, so verify a Windows scan of a directory containing one lists it as data.
6. **NFD names:** NTFS is byte-preserving like ext4, so the Linux NFD result should extrapolate —
   confirm no shell-side normalization happens before `authorize_root`.

## 7. Evidence run during this review

- `cargo test -p soul-fileplan --all-targets`: 27 passed, 0 failed (module tests 3,
  authorized_scan 6, filename_injection 3, no_write_api 5, unauthorized_is_refused 10).
- `cargo test -p soulcore --test fileplan_commands`: 6 passed, 0 failed.
- `cargo test -p soul-policy --test hitl`: 15 passed, 0 failed — including the F-05.3 anchor, which
  commit `eb74904`'s file list confirms WP11 never touched.
- ST-02's mutation table (report §5) was cross-checked against the current tests; the three
  test-hole repairs it describes (direct sortedness, JSON fidelity, empty-roots sensitivity) are
  all present in the code as reviewed.
