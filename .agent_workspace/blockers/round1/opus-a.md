# Round 1 / slot opus-a — Goal 1 acceptance matrix audit (AC-01..AC-26)

Model slug: `claude-opus-5-thinking-high-fast`. Analysis only; no application code written.

Refs audited:

- `origin/cursor/soul-goal1-7b1c` @ `3e88b48` ("ci: run one_store on windows, where its answer is the interesting one") — the P0 tip.
- `origin/agent/dev-sota` @ `215a310` ("docs: defer QQ/WeChat client reading (D32)") — consulted where it diverges.

---

## 0. The three facts that reframe everything below

### 0.1 `origin/cursor/soul-goal1-7b1c` is an orphan branch

```
git merge-base origin/main origin/cursor/soul-goal1-7b1c   -> no common ancestor (exit 1)
git merge-base origin/main origin/agent/dev-sota           -> ea6f62f (Initial commit)
git merge-base origin/cursor/soul-goal1-7b1c origin/agent/dev-sota -> no common ancestor
```

`soul-goal1-7b1c` has 19 commits and its own root (`fbad85b`). `agent/dev-sota` has 70 commits and roots at
the repository's `Initial commit`. They share commit *subjects* but no history. Both PRs are
`mergeable=CONFLICTING, mergeStateStatus=DIRTY`:

| PR | Head | Files | Mergeable |
|---|---|---|---|
| [#2](https://github.com/Xhhemoing/Soul/pull/2) | `cursor/soul-goal1-7b1c` | 348 | CONFLICTING |
| [#4](https://github.com/Xhhemoing/Soul/pull/4) | `agent/dev-sota` | 360 | CONFLICTING |

This is a Goal-1-close blocker in its own right and it is not mentioned anywhere in `docs/STATUS.md`.

### 0.2 Windows CI is red, and the red is hiding 68 of 83 test binaries

Latest run on the tip (`32748412486`): `lint` ✅, `test (ubuntu, headless)` ✅, `sbom` ✅,
`package` ✅ (green on the previous identical-content run `32741602141`), **`test (windows-latest)` ❌**.

The failure is two exact-list assertions in `crates/soul-fileplan/tests/authorized_scan.rs`:

```
crates\soul-fileplan\tests\authorized_scan.rs:65  — moves list has an extra ("decoy.txt", "文档/decoy.txt")
crates\soul-fileplan\tests\authorized_scan.rs:165 — scan entry list has an extra "decoy.txt"
test result: FAILED. 7 passed; 2 failed
```

Root cause is in the fixture, not the product. `crates/soul-fileplan/tests/common/mod.rs` builds a third
directory whose whole purpose is to be `Alpha` under another spelling:

```104:108:/tmp/g1/crates/soul-fileplan/tests/common/mod.rs
        write(
            &base.join("alpha"),
            "decoy.txt",
            "同名不同大小写的第三个目录",
        );
```

On NTFS `base/alpha` *is* `base/Alpha`, so `decoy.txt` is written straight into the authorized root and
both exact-list assertions gain an entry. The product behaviour is correct — `PathMatching::CaseFolded`
is the right Windows rule and `unauthorized_paths.rs::case_is_decided_by_the_filesystem_rather_than_assumed`
deliberately asserts it. The fixture is what assumes a case-sensitive filesystem.

**The second-order damage is larger than the failure.** `cargo test --workspace --all-targets` is
fail-fast. Test binaries run in crate-alphabetical order, so the Windows job gets through
`soul-collect` → `soul-draft` → `soul-egress` → `soul-fileplan` and dies. Counted from the job log:
**15 of 83 test binaries execute on windows-latest.** Everything from `soul-graph` onward has never run
on Windows at this tip — `soul-graph`, `soul-import`, `soul-memory`, `soul-policy`, `soul-profile`,
`soul-schema`, `soul-store`, `soul-store-api`, `soul-testkit`, `soulcore`, `xtask`. And because the
`cargo test (desktop shell)` and `cargo test --no-run (ipc_roundtrip)` steps come *after* the dying step,
`one_store`, `command_surface`, `no_egress_path`, `shell_is_local_only` and the ipc compile check have
also never executed at this tip — including the `one_store` addition that the tip commit exists to make.

Windows-executed and green at the tip: `soul_collect` (lib + 4 tests), `soul_draft` (lib + 5 tests),
`soul_egress` (lib + `e1_origin`), `soul_fileplan` (lib only).

The last fully green Windows run was `32728368931`, at the WP05/WP06 tree — three batches of work ago.

### 0.3 On Windows the shipped product cannot open its own database

`DpapiKeyProvider::unprotect` returns `KeyError::Unsupported` on **both** cfg arms
(`crates/soul-store/src/keys.rs`), and `soulcore::commands::session::key_provider` selects it whenever
`cfg!(windows)`. `crates/soulcore/tests/session_commands.rs::the_session_says_which_key_material_opened_the_store`
asserts `!status.store_opened` on Windows and calls that the documented state.

So on the target platform: the wizard runs, the tray runs, `/files` runs (file planning never touches the
store) — and `/graph`, memory, profile, import, research preview and the audit chain are all unreachable.
Every store-backed acceptance criterion is green in CI only because the tests substitute `TestKeyProvider`.
This is recorded in STATUS as a leftover; it is in fact the gate on what AC-01 means.

---

## 1. AC-01..AC-26 verdicts

Verdict is against the matrix as written. "Windows" is a separate column because the matrix's platform is
Windows 11 and the two answers differ for most rows.

| AC | Verdict | Test evidence | Windows reality at tip |
|---|---|---|---|
| AC-01 | **UNPROVEN** (author-manual, correctly so) — machine half PARTIAL | `apps/desktop/src-tauri/tests/shell_is_local_only.rs` (7 assertions: `asInvoker`, `uiAccess="false"`, manifest embedded by `build.rs`, no updater endpoint/key, `webviewInstallMode: "skip"`, CSP `default-src 'self'`, binary named `soul`); `package` job reads the manifest out of the built `soul.exe` rather than out of `tauri.conf.json`; `scripts/install-smoke.ps1` (533 lines) + `crates/soulcore/tests/install_smoke_script.rs` (6) | `tauri build` has never run on any runner; the installer has never been executed (CI passes `-SkipInstall`); tray icon, UAC prompt and Task-Manager process name are all human-eye items. Checklist `scripts/author-manual-checklist.md` §§1–4. |
| AC-02 | **PASS** | `crates/soulcore/tests/shell_commands.rs::a_finished_wizard_leaves_every_capability_off` + `::the_wizard_would_refuse_a_configuration_with_a_switch_on`; `crates/soulcore/tests/config_defaults.rs` (5); `apps/desktop/src/routes/Wizard.test.tsx` (5); `apps/desktop/src-tauri/tests/ipc_roundtrip.rs`; `crates/soulcore/src/headless.rs` wizard step | soulcore never reached in the Windows test job. Partially rescued by the `package` job: `install-smoke.ps1` asserts `config.fully_closed` against a Windows **release** `soul-headless.exe`. |
| AC-03 | **PARTIAL** | `crates/soul-profile/tests/questionnaire_intake.rs` (6 — provenance, `user_stated`, sealed bodies, prose-as-pointer, partial fixture, rejection corpus); `crates/soul-profile/tests/one_questionnaire.rs`; `crates/soul-import/tests/questionnaire.rs` (6); `crates/soulcore/tests/import_and_graph_commands.rs::the_fallback_questionnaire_is_the_profile_s_own_and_leaves_a_profile_behind`; headless `profile` step | **No user-reachable path exists.** `apps/desktop/src/core.ts` `COMMANDS` has exactly 14 entries and none of them is a questionnaire command; `Wizard.tsx` renders a four-row defaults table and one acknowledge checkbox. `soulcore::commands::profile::{questions,intake}` are written but unregistered. CI green comes from a fixture answer sheet (`fixtures/questionnaire/answers_basic.json`), not from "完成问卷". Not run on Windows; and unreachable there anyway (0.3). |
| AC-04 | **PASS (Linux)** | `crates/soul-store/tests/no_plaintext_at_rest.rs`; `crates/soul-import/tests/soul_import_v1.rs::a_valid_file_lands_sealed_with_no_plaintext_left_on_disk` | Not executed on Windows at tip. |
| AC-05 | **PASS (Linux)** | `crates/soul-import/tests/telegram.rs` (7 — mapping, service-message skip, segmented `text` rejoin, named missing fields, chat titles absent) | Not executed on Windows at tip. |
| AC-06 | **PASS (Linux)** | `crates/soul-profile/tests/axes_and_evidence.rs` (both directions, incl. `DanglingEvidence`); `crates/soul-graph/tests/ego_graph.rs::a_tie_inference_dereferences_to_the_observations_behind_it` | Not executed on Windows at tip. |
| AC-07 | **PASS** | `crates/soul-profile/tests/correction_lock.rs`; `crates/soul-draft/tests/voice_and_template.rs::a_pinned_voice_field_survives_an_inference_that_disagrees_with_it` + `::the_prompt_carries_the_users_value_and_not_the_inferred_one` | Draft half green on Windows (`soul_draft` ran). Profile half not executed. |
| AC-08 | **PASS (Linux)** | `crates/soul-graph/tests/ego_graph.rs::three_conversation_partners_produce_three_evidence_backed_edges` — `third_party_nodes().len() >= 3`, one edge per partner, `resolve_evidence` called per edge with a length equality; `crates/soul-import/tests/import_to_graph.rs::a_four_partner_export_becomes_four_evidence_backed_edges` walks the same ground from a fixture file; headless `graph` step carries the literal `"AC-08 wants at least three people, got {}"`; `apps/desktop/src/routes/Graph.test.tsx` (7) renders nodes/edges/evidence counts | Genuinely strong — this is one of the better-evidenced rows. But `soul-graph` never runs on Windows at tip, and in the shipped Windows app `/graph` returns a refusal because the store will not open. |
| AC-09 | **PASS (fake source)** | `crates/soul-collect/tests/consent_gate.rs` (4, incl. `samples_taken() == 0` — the gate is in front of the source — and a positive control that writes 10 events with consent on) | Executed and green on Windows. But the source under test is `FakeForegroundSource`; `crates/soul-collect/src/windows.rs` has only ever been `cargo check`ed. Real-machine half is checklist §6. |
| AC-10 | **PASS (fake source)** | `crates/soul-collect/tests/collection_lifecycle.rs` (real thread, real `SqlCipherStore`, `stop()` ≤ 1s budget, 20 further switches + 1s wait with no new events, revocation path separately) | Same caveat as AC-09. |
| AC-11 | **PASS** | `crates/soul-egress/tests/e1_origin.rs`; `crates/soul-draft/tests/wire.rs::a_redirect_off_the_configured_origin_is_refused_and_the_target_never_hears_from_us` (asserts the redirect target's own `request_count() == 0`) + `::a_different_port_on_the_same_host_is_a_different_origin` | Executed and green on Windows. |
| AC-12 | **PASS** | `crates/soul-draft/tests/wire.rs::the_third_partys_words_do_not_reach_the_wire` (asserts on bytes `MockLlm` received); `crates/soul-policy/tests/redactor_leakage.rs` | Draft half green on Windows; policy half not executed. |
| AC-13 | **PASS** | `crates/soul-draft/tests/wire.rs::one_exemption_covers_one_request_and_the_next_is_placeheld_again` + `::an_exempted_body_still_placeholds_the_name_and_the_number`; `crates/soul-policy/tests/redactor_exemption.rs` | Draft half green on Windows. |
| AC-14 | **PASS (Linux)** | `crates/soul-memory/tests/crud_roundtrip.rs` (close-and-reopen before asserting; audit carries only ids and counts; chain through `LeakageChecker` at threshold 4) | Not executed on Windows at tip. |
| AC-15 | **PASS (Linux)** | `crates/soul-memory/tests/forget_reopen.rs`; `crates/soul-store/tests/forget.rs`; `crates/soul-store/tests/crash_recovery.rs::a_crash_between_content_key_deletions_leaves_no_half_forgotten_memory` | Not executed on Windows at tip. |
| AC-16 | **PASS** | `crates/soul-draft/tests/people_summary.rs` — `SummaryPoint::new` rejects an empty evidence list (type-level), unresolvable evidence refuses the whole summary, denylist tests, `::the_summary_names_nobody` | Executed and green on Windows. |
| AC-17 | **PASS** | `crates/soul-draft/tests/voice_and_template.rs::with_no_endpoint_the_draft_is_a_template_and_nothing_is_contacted` (`MockLlm.request_count() == 0`), `::the_template_is_a_function_of_its_inputs_and_nothing_else` (32 runs), `::every_voice_combination_renders_something_this_product_may_say` (81 × 2) | Executed and green on Windows. |
| AC-18 | **FAIL on Windows / PASS on Linux** | `crates/soul-fileplan/tests/authorized_scan.rs` (9 — the positive half, so the 100% refusal is not satisfiable by refusing everything); `crates/soul-fileplan/tests/unauthorized_paths.rs` (30-entry corpus, length itself asserted); `crates/soul-fileplan/tests/no_write_api.rs` | **RED.** Two exact-list assertions break on NTFS case folding (§0.2). Note the refusal half (`unauthorized_paths.rs`) and `no_write_api.rs` also never run on Windows — they are alphabetically after `authorized_scan` inside the same crate. |
| AC-19 | **PASS (Linux)** | `crates/soul-fileplan/tests/execution_is_refused.rs` (unknown action incl. every non-fileplan `ActionKind`; three plan-hash-mismatch paths; a really-issued-and-really-spent token replayed → `TOKEN_REPLAYED`; a request with nothing wrong with it → `WRITE_NOT_IMPLEMENTED`); `crates/soul-policy/tests/hitl.rs` (15) | Neither file executes on Windows at tip. |
| AC-20 | **PASS (Linux)** | `crates/soul-store/tests/research_preview.rs` — real `GROUP BY` aggregation, `zero_third_party_rows` as the only constructor, `third_party_rows_excluded > 0` proving the filter fired, output through `export-manifest.schema.json` + leakage checker, plus a test that reads `research_preview.rs` back and finds no write API | Not executed on Windows at tip. |
| AC-21 | **PARTIAL** | `crates/soulcore/src/headless.rs` (13 steps in one process) + `crates/soulcore/src/netwatch.rs` (`/proc/self/fd` → `/proc/net/{tcp,tcp6,udp,udp6}`, 5 ms sampling) + `crates/soulcore/tests/netwatch.rs` (the observer is proved non-vacuous against a real socket to a documentation address, plus a synthetic table); `xtask e0-audit` for the source half | **Vacuous on the target platform.** `netwatch` returns `Support::Unsupported` off Linux, and `headless::egress_findings` only `require`s `non_loopback_connections == 0` — which an Unsupported reading satisfies trivially. The strict check (`assert!(report.egress.observed, ...)`) lives only in `crates/soulcore/tests/headless_main_flow.rs:52`, which never runs on Windows. `install-smoke.ps1` line 473 also asserts only the count. The one real Windows observation is the script's own `Get-NetTCPConnection -OwningProcess` loop (lines 303–314), which does run in the green `package` job — but it is TCP-only (no UDP peer in the Windows endpoint table), and it watches `soul-headless.exe`, not `soul.exe` and not `msedgewebview2.exe`. |
| AC-22 | **PASS (Linux) / zero Windows machine evidence** | `apps/desktop/src/components/CloudToggle.test.tsx` (4 — visible, `aria-checked="false"` after five presses, explanation compared to the core's constant, and `fetch`/`XMLHttpRequest`/`WebSocket`/`EventSource`/`sendBeacon` all replaced with fail-on-call stubs); `apps/desktop/src/contract.test.ts` cross-language string comparison; `crates/soulcore/tests/shell_commands.rs::the_cloud_switch_says_the_same_thing_whichever_way_it_is_pressed` + `::no_configuration_can_claim_the_cloud_is_available`; `ipc_roundtrip.rs`; headless `cloud` step | The Windows job runs **no vitest at all** (cargo only), soulcore is never reached, and `ipc_roundtrip` is `--no-run`. So AC-22 has literally zero executed assertions on windows-latest. The OS-level half (resource monitor, `msedgewebview2.exe` accounted separately) is checklist §5 and correctly author-manual. |
| AC-23 | **PASS (Linux)** | `crates/soul-policy/tests/audit_chain.rs` (matrix replay through leakage checker; prose fields rejected before insert); headless `audit` step re-checks three prose strings against the serialized chain | Not executed on Windows at tip. |
| AC-24 | **PASS (Linux)** | `crates/soul-policy/tests/audit_crash.rs`; `crates/soul-store/tests/crash_recovery.rs::a_crash_mid_commit_loses_one_event_and_leaves_the_chain_verifiable` (`2*off->panic` fail point, subprocess abort, exactly one lost, `integrity_check` ok, chain verifies, with an unarmed control run) | Not executed on Windows at tip. |
| AC-25 | **PASS** | `crates/soul-draft/tests/injection.rs::nothing_from_a_paste_reaches_the_instruction_slot` (byte equality against `soul_policy::e1::DRAFTING_INSTRUCTION`) + `::a_server_at_the_address_an_injection_names_never_hears_from_us`; `crates/soul-import/tests/injection_is_data.rs`; `crates/soul-fileplan/tests/file_names_are_data.rs`; `crates/soul-policy/tests/injection.rs` | Only the draft third is green on Windows; the import, fileplan and policy thirds never execute there. |
| AC-26 | **FAIL** | — | Red on windows-latest. Beyond that: the Windows job runs **only** `cargo test` — no `cargo fmt`, no `clippy`, no `schema-freeze --check`, no `e0-audit`, no `denylist-audit`, no `fixtures-verify`, no vitest. So "lint/test/schema/红线" is Linux-only, and "打包绿" is met on no runner at all (`package` job stops before `tauri build` by design). |

Score: **13 clean PASS** (AC-02, 04, 05, 06, 07, 09, 10, 11, 12, 13, 14, 15, 16, 17, 19, 20, 23, 24, 25 —
of which 8 have Windows execution), **AC-03 and AC-21 PARTIAL**, **AC-01 UNPROVEN by design**,
**AC-18 and AC-26 FAIL**.

---

## 2. Where STATUS overstates what the tests cover on Windows

Each of these is a place where `docs/STATUS.md` reads as settled and the Windows evidence does not exist.

1. **STATUS line 7: "Windows CI 在 schema freeze CRLF 修复后一度全绿."** True, and the "一度" is doing a
   lot of work — that green run is `32728368931`, three batches back. The tree as it stands has never had a
   green Windows run and has never executed 68 of its 83 test binaries there. The sentence should name the
   commit and the count.

2. **WP11 leftover 14** narrates the `\\?\C:\` strip as the fix that un-reds windows-latest
   ("不剥的话，windows-latest 上临时目录连授权都过不了"). That fixed `authorize Alpha`. `authorized_scan`
   is still red for an unrelated reason — the case-folded fixture collision. STATUS presents a closed loop
   that is not closed.

3. **WP13 第二段 leftover 8**: "`one_store` 进了 windows-latest 那份点名清单." The step is in `ci.yml`, but
   the job dies in the preceding step, so it has never executed. Same for `command_surface`,
   `no_egress_path`, `shell_is_local_only`, and the `ipc_roundtrip --no-run` compile guard.

4. **WP13 第一段, AC-21 row**: "非回环连接 = 0，观察来的不是推理来的." On Windows it is precisely inferred.
   `egress_findings` checks the count and not `observed`; the module doc's own warning
   ("一个总是说「没看到连接」的观察器和一个坏掉的观察器长得一模一样") is not enforced at the one place it
   matters. `netwatch.rs` and WP13 leftover 4 both state the limitation honestly in prose — the gap is that
   nothing fails because of it.

5. **WP07 交付 table**: "CI 只替换最前面那一段，后面全是 Windows 机器上跑的同一份代码." The consent gate and
   store half do run on Windows (and did, at this tip). But `crates/soul-collect/src/windows.rs` — the only
   real `ForegroundSource` — has never been executed anywhere, on any runner or machine. AC-09/AC-10 are
   green against `FakeForegroundSource` on both platforms.

6. **WP03/WP06 marked 完成 and AC-03 implicitly claimed.** STATUS does say "还没有人画这十一道题" in the seam
   section, but the progress table reads 完成 and nothing states that AC-03 has no user-reachable path. The
   matrix's Given/When is "无导入 / 完成问卷"; what CI proves is "无导入 / 把一份 fixture 答卷喂给 `intake`".

7. **WP13 第二段 leftover 1 (DPAPI)** is filed as a leftover. It is a gate: without it, an installed Soul on
   Windows 11 starts, shows a tray, and cannot read a single row of its own data. AC-01's "安装启动" is
   satisfiable while the product is non-functional, which means AC-01 as written under-specifies.

8. **Nothing in STATUS mentions the branch topology.** `soul-goal1-7b1c` shares no ancestor with `main`.

---

## 3. Leftovers: what blocks Goal 1 close vs what is honest author-manual

### 3.1 Blocking — must be fixed in code before Goal 1 can close

| # | Item | Why it blocks |
|---|---|---|
| B1 | `authorized_scan.rs` red on NTFS | AC-18 FAIL, AC-26 FAIL |
| B2 | `cargo test --workspace` is fail-fast on the Windows job | One red hides 68 binaries; the matrix's "CI" column is unverifiable on the target platform |
| B3 | DPAPI unimplemented | The Windows product cannot open its store; AC-03/04/05/06/07/08/14/15/16/20/23/24 are all green in CI and all unreachable in the shipped app |
| B4 | `egress_findings` accepts `Support::Unsupported` as a pass | AC-21 is vacuous on the only platform Soul ships to |
| B5 | No questionnaire UI, no registered questionnaire command | AC-03 has no user path |
| B6 | Windows job runs no `schema-freeze` / `e0-audit` / `denylist-audit` / vitest | AC-26's "schema/红线" column is Linux-only |
| B7 | `ipc_roundtrip` cannot load on windows-latest (`STATUS_ENTRYPOINT_NOT_FOUND` from `WebView2Loader.dll`) | AC-02 and AC-22 have no IPC-level evidence on Windows |
| B8 | Two orphan branches, both PRs CONFLICTING | Neither PR can merge; Goal 1 has no landable artifact |

### 3.2 Honest author-manual — should **not** block code merge

These are correctly identified as human items in `scripts/author-manual-checklist.md`, and the reasoning
behind each is sound. They block *release*, not *merge*.

| Checklist § | Item | AC |
|---|---|---|
| §2 | Tray icon present in the notification area, tooltip `Soul`, two menu entries, close-to-tray, quit really quits | AC-01 |
| §3 | No shield overlay, no UAC on double-click, "Elevated = No" in Task Manager, works from a standard user account | AC-01 |
| §4 | Process shows as `soul.exe`; `%LOCALAPPDATA%\Soul\soul.exe` exists; product name/version | AC-01 |
| §5 | OS-level zero traffic on five cloud-toggle presses, with `msedgewebview2.exe` recorded separately | AC-22 |
| §6 | `collect-probe --i-consent --seconds 20` with a human switching windows | AC-09, AC-10 |
| §7 | WebView2 runtime on a clean Win11, CJK font + 150% DPI, uninstall vs user data, Defender/SmartScreen | env |

Two borderline items I would argue belong here rather than in §3.1, but which need an author decision
because AC-26 literally says "打包绿":

- **`tauri build` / NSIS bundling.** The stated reason is that a CI job downloading a third-party installer
  in order to prove the installer is trustworthy has argued itself in a circle. That reasoning holds. But
  AC-26's 打包 column is then satisfiable on no runner, and the `package` job's own name
  ("as far as CI can honestly go") concedes it. Either AC-26 is amended to read "package job green +
  author-signed bundle", or Goal 1 cannot close on CI alone. This is a matrix defect, not a code defect.
- **`soul-headless.exe` is not in the bundle** (`mainBinaryName` only), so `install-smoke.ps1 -Headless`
  points at a same-commit binary rather than the installed one. Fixing it means `externalBin`, i.e. a
  change to the product's packaging shape. Reasonable to defer, but the honest statement is
  "this build's core runs closed on this machine", not "the installed core runs closed", and the checklist
  already says exactly that.

---

## 4. Ranked: what blocks Goal 1 close

1. **AC-18 / AC-26 — Windows CI red.** One fixture, two assertions. Everything else on this list is
   invisible until it is fixed, because the job stops there.
2. **Fail-fast masking.** Even after (1), a single future red will re-hide the suite. The Windows job needs
   `--no-fail-fast` before anyone can claim "CI proves this on Windows".
3. **DPAPI.** Without it the product is a shell on its target platform. Twelve acceptance criteria are green
   in CI and dead in the installed app.
4. **AC-21 vacuous on Windows.** The Rust observer must fail loudly when it cannot look, and the PowerShell
   observer must be the thing that carries the criterion there.
5. **AC-03 has no user path.** The wizard does not ask the eleven questions and no command exposes them.
6. **AC-26 scope.** The Windows job is missing every red-line audit; 打包 is green nowhere.
7. **AC-22 has zero executed assertions on Windows.**
8. **Branch reconciliation.** Two orphan histories, two conflicting PRs, no landable artifact.

`agent/dev-sota` does not help with any of these: its `soul-fileplan` is an older design with no
`screen.rs`, its `soulcore` has no `headless.rs`/`netwatch.rs`/`session.rs`, it has no `install-smoke.ps1`
and no SBOM, its desktop-shell step uses `--all-targets` (which would *load* `ipc_roundtrip.exe` on Windows
and fail the way WP13 documents), and its `test (windows-latest)` job has been **cancelled on every run** —
it has no Windows evidence at all. Treat `soul-goal1-7b1c` as the P0 line and `dev-sota` as superseded.

---

## 5. Exact next work packages

Sized by blast radius, not by calendar. Each names the files it touches.

### WP-W1 — Make the Windows suite runnable (unblocks 1 and 2)

- `crates/soul-fileplan/tests/common/mod.rs`: the decoy directory must not collide with the authorized root
  under case folding. Two options, and the second is better:
  (a) rename `alpha` to a spelling that differs by more than case and lose the NTFS-collision reading; or
  (b) keep `alpha` but create it only `#[cfg(unix)]`, and add a `#[cfg(windows)]` assertion that
  `base/alpha` and `base/Alpha` are the same directory — which is the actual Windows fact and is currently
  tested nowhere.
- `crates/soul-fileplan/tests/authorized_scan.rs:65` and `:165`: derive the expected lists from the fixture
  rather than hard-coding them, or gate the two exact lists on the same cfg.
- `.github/workflows/ci.yml`, `test-windows` job: add `--no-fail-fast` to `cargo test --workspace
  --all-targets`, and to the desktop-shell step.
- Exit criterion: a windows-latest run that reports 83 test binaries.

### WP-W2 — Make AC-21 non-vacuous on Windows (unblocks 4)

- `crates/soulcore/src/headless.rs`, `egress_findings`: `require` that
  `findings.observed || <an explicit caller-supplied waiver>`. A run that could not look must not report
  `ok: true` silently — at minimum `MainFlowReport.ok` should be distinguishable from
  "observed and clean".
- `scripts/install-smoke.ps1` line ~473: add an assertion that when
  `smoke.Report.egress.observed` is false, the script's own `Get-NetTCPConnection` watch must have taken
  at least N samples — i.e. the PowerShell observer becomes load-bearing rather than decorative.
- Optionally: a `windows-sys` `GetExtendedTcpTable` implementation behind a feature, in a crate that is
  allowed `unsafe` (not `soulcore`, which is `forbid(unsafe_code)`). Larger change; the script assertion
  is the cheap correct step.

### WP-W3 — DPAPI (unblocks 3)

- New: a minimal Windows key crate (or a `#[cfg(windows)]` module allowed `unsafe`) wrapping
  `CryptProtectData` / `CryptUnprotectData`.
- `crates/soul-store/src/keys.rs`: `DpapiKeyProvider::unprotect` calls it; the `#[cfg(not(windows))]` arm
  keeps refusing.
- `crates/soulcore/tests/session_commands.rs::the_session_says_which_key_material_opened_the_store`:
  the Windows branch flips from `assert!(!status.store_opened)` to `assert!(status.store_opened)`.
- `apps/desktop/src-tauri/tests/one_store.rs::one_session_hands_out_one_store`: the `(None, None)` arm
  becomes unreachable and should be deleted, not left as an escape hatch.
- `docs/SECURITY.md` and STATUS WP02 leftover 3 / WP13-2 leftover 1 updated.
- Note: this is the work package that turns twelve CI-green criteria into product-true ones. It is the
  highest-value item on this list after WP-W1.

### WP-W4 — The eleven questions on screen (unblocks 5)

- `apps/desktop/src-tauri/src/commands.rs` + `apps/desktop/src/core.ts`: register
  `profile_questions` and `submit_questionnaire` (thin wrappers over
  `soulcore::commands::profile::{questions, intake}`); `COMMANDS` goes from 14 to 16 and
  `command_surface.rs` / `contract.test.ts` will hold both sides to it automatically.
- `apps/desktop/src/routes/Wizard.tsx` (or a new `/profile` step): render the eleven questions from
  `questions()`; `Answer::for_question(question_id, option)` is the only call the UI needs.
- New `apps/desktop/src/routes/Questionnaire.test.tsx`: assert the question ids rendered equal
  `fixtures/questionnaire/v0_1.json`, that a blank text box submits nothing, and that a completed
  questionnaire yields a non-empty profile through the fake core.
- Depends on WP-W3 on Windows (intake needs an open store).

### WP-W5 — Close AC-26's scope (unblocks 6, 7)

- `.github/workflows/ci.yml`, `test-windows` job: add `cargo run -p xtask -- schema-freeze --check`,
  `e0-audit`, `denylist-audit`, and `cargo fmt --check` + `clippy`. These are cheap and they are the
  "红线" column.
- Add a `vitest` run to the Windows job, or state in STATUS that the UI suite is platform-independent and
  Linux-only on purpose. Either is defensible; silence is not.
- `ipc_roundtrip` on windows-latest: investigate whether pinning a WebView2 SDK version or using
  `tauri::test::mock_context` without the loader avoids `STATUS_ENTRYPOINT_NOT_FOUND`. If it cannot be
  fixed, record it as a permanent platform gap in the matrix rather than as a WP13 leftover.
- Decide AC-26's 打包 column (see §3.2).

### WP-W6 — Branch reconciliation (unblocks 8)

- Rebase or replay `cursor/soul-goal1-7b1c` onto `origin/main` so it has an ancestor. Its 19 commits are
  self-contained; a fresh branch off `main` with the same tree is the low-risk route.
- Decide the fate of [#4](https://github.com/Xhhemoing/Soul/pull/4): `agent/dev-sota` is behind on
  `soul-fileplan`, `soulcore` and CI, and has no Windows evidence. Closing it in favour of
  [#2](https://github.com/Xhhemoing/Soul/pull/2) is the honest call; if any of its work is unique it should
  be cherry-picked first.
- Record the decision in `docs/STATUS.md`, which currently does not mention that two unrelated histories
  exist.

### Ordering

WP-W1 first and alone — nothing else can be measured until the Windows suite runs. Then WP-W6 (cheap,
unblocks landing) and WP-W2 (cheap, small diff) in parallel. Then WP-W3, which is the largest and the one
that changes what the product actually does on its target platform. WP-W4 depends on WP-W3 for its Windows
half. WP-W5 last, because its value is in guarding a suite that WP-W1 has to make runnable first.

The author-manual checklist (§3.2) can be run at any point after WP-W3 lands; running it before then would
only confirm that the app cannot open its database.
