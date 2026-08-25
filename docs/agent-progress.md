# Agent progress

Parent exclusive branch: `cursor/soul-integration-4a8e`.
Working base: `origin/cursor/first-test-candidate-c441` @ `85aae68` (strict FF of unique trunk `cursor/soul-goal1-7b1c` @ `5309656`, AD-8).
Goal: complete Soul so it delivers (1) multi-dimensional interpersonal and self analysis, (2) work and social assistance, (3) simple behavior and group prediction, (4) data collection for continuous optimization.
Process: Fable (`claude-fable-5-thinking-xhigh`) scans/reviews; Opus (`claude-opus-5-thinking-high-fast`) lands code. Commit one purpose at a time. PR when there is a real artifact. Merge when safe; otherwise record `BLOCKED` and continue.

## Current round

**ROUND 13** — review the Unicode phone-shape landing and remaining identifier/honesty leftovers. Do not re-open closed R4–R12 items (D55, D61–D63, P2 E1 mutex, fileplan, config.json, COMMANDS=38, contiguous+grouped+fullwidth/dash-family phone shape, tray mutex, NSIS data-dir hook, keys.dpapi create_new, cloud toggle inert, import-tx wrap, questionnaire `transact` wrap).

| Field | Value |
|---|---|
| Current task | Round 12 closed. Landed fullwidth/dash-family phone placeholder. Round 13 Fable: landing review; remaining Pd/soft-hyphen; add_account width-literal; intake refusal-notice pin; fakeCore truncation; denylist frozen-ten; NSIS comment; SECURITY DPAPI tense; PendingForget; landable vs 20-round floor. |
| Parent model | product/account setting |
| Hosted CI | `BLOCKED` — billing/spending |
| Exclusive tip | `e893109` (+ this docs closeout); PR #15 |

### Round 13 Fable dispatch

| # | Direction |
|---|---|
| 1 | Review the phone-unicode landing (`a988fe2` / merge `e893109`) for regressions |
| 2 | Remaining dash/hyphen leftovers outside the landed family: U+2015, U+00AD, U+FE58, U+FE63, U+30FC |
| 3 | `add_account` stays width-literal for non-phone registered identifiers |
| 4 | Live pin that a questionnaire refusal carries `INTAKE_ROLLED_BACK_NOTICE` |
| 5 | `fakeCore.ts` forget-mismatch truncation vs `FORGET_NOT_PREVIEWED_NOTICE` |
| 6 | Denylist frozen-ten vs runtime `diagnostic_terms.txt` honesty |
| 7 | NSIS PREUNINSTALL comment vs pre-hook uninstallers |
| 8 | SECURITY.md DPAPI present tense vs last windows-latest run on `2e72ddf` |
| 9 | `ForgetState::PendingForget` unreachable vs Graph/Memory `== Forgotten` checks |
| 10 | Synthesis: remaining landable v0.1 slices vs 20-round floor vs author-machine-only |

## ROUND 12 record

| Field | Value |
|---|---|
| ROUND | 12 |
| 子代理任务 | 10× Fable + Opus: intake-transact landing review; fullwidth/en-dash/fullwidth-hyphen phones; denylist frozen-ten; fakeCore truncation; NSIS comment; intake refusal-notice pin; SECURITY DPAPI tense; PendingForget; Graph source-label; landable vs 20-round floor |
| 发现问题 | Fullwidth digits (`１３８００１３８０００`) never entered `phone_shape_end`; dash-family groups (U+2010–U+2014, U+2212, U+FF0D) left each run under 7 digits; mixed `138-0013–8000` leaked `[账号已占位]–8000` |
| 修复问题 | `is_phone_digit` (ASCII + U+FF10–U+FF19); `is_group_separator` learns the dash family; leakage checker `digit_skeleton`/`number_needle` in lockstep. No NFKC. Accepted false positives: year range `2019–2026`, fullwidth date `２０２６－０８－２５` |
| 测试结果 | Parent re-run on merge `e893109`: redactor_exemption 16; redactor_leakage 16; leakage_checker 10; fixture_corpus 9; session_e1 33. Opus mutation: ASCII-only digit kills 3; old separators kill 5; `number_needle` None kills 2 |
| Commit | `a988fe2` on `cursor/phone-unicode-shape-4a8e`; merge `e893109` |
| PR | #15 |
| Merge状态 | Exclusive has the merge. → `main` still `BLOCKED`. PR #7 still not merged |
| 下一轮重点 | Phone-unicode landing review; remaining Pd/soft-hyphen; add_account width-literal; intake notice pin; fakeCore; denylist; NSIS comment; SECURITY tense; PendingForget; synthesis toward 20-round floor |
| NO_HIGH_VALUE | Intake-transact landing (already merged); denylist frozen-ten; fakeCore truncation; NSIS comment; intake refusal-notice pin; SECURITY DPAPI tense; PendingForget; Graph source-label |

## ROUND 11 record

| Field | Value |
|---|---|
| ROUND | 11 |
| 子代理任务 | 10× Fable + Opus: grouped-phone landing review; remaining +86/paren shapes; denylist frozen-ten; questionnaire intake crash; Graph source-label copy; NSIS comment; SECURITY DPAPI tense; fakeCore truncation; PendingForget; landable vs author-machine |
| 发现问题 | `Session::answer_questionnaire` wrote one savepoint per answer. A crash mid-batch left orphan `ui.questionnaire` events and Questionnaire evidence (STATUS WP06 leftover 8; AC-24 strict reading) |
| 修复问题 | Wrap `profile_commands::intake_from` in `SqlCipherStore::transact`. `rolled_back` takes a caller notice so the wizard is not shown the import-file sentence; intake uses `INTAKE_ROLLED_BACK_NOTICE`. Failpoint reuses `STORE_EVENT_COMMIT_MID` |
| 测试结果 | Parent re-run on merge `28601cc`: `session_crash` 9; `session_screens` 11. Opus: soulcore whole suite; desktop `ipc_roundtrip` 56 / `command_surface` 6; soul-profile 26; soul-import 63; soul-store 57. Fable mutation: wrap removed → leftover `(2, 2)` |
| Commit | `4d827ac` on `cursor/intake-transact-4a8e`; merge `28601cc` |
| PR | #15 |
| Merge状态 | Exclusive has the merge. → `main` still `BLOCKED`. PR #7 still not merged |
| 下一轮重点 | Intake landing review; remaining en-dash/fullwidth hyphen/fullwidth digits; denylist frozen-ten; fakeCore truncation; NSIS comment; intake refusal-notice pin; SECURITY tense; PendingForget; Graph source-label; synthesis toward 20-round floor |
| NO_HIGH_VALUE | Grouped-phone landing (already merged); remaining +86/paren (below bar); denylist frozen-ten; intake crash as P2 until synthesis #10; Graph source-label copy; NSIS comment; SECURITY DPAPI tense; fakeCore truncation; PendingForget |

## ROUND 10 record

| Field | Value |
|---|---|
| ROUND | 10 |
| 子代理任务 | 10× Fable + Opus: tray/single-instance; NSIS hooks; keys.dpapi; redactor exemption; cloud toggle; denylist; fixture corpus; fakeCore; crash/WAL; IPC roundtrip; grouped-phone shape |
| 发现问题 | Grouped phones (`138 0013 8000`, `138-0013-8000`) skipped the 7-digit contiguous shape and reached the exempted turn |
| 修复问题 | `phone_shape_end` counts digit groups joined by one space/ideographic-space/hyphen/dot; ISO `2026-08-25` is the accepted false positive |
| 测试结果 | redactor_exemption 15; redactor_leakage 13; leakage_checker 7; fixture_corpus 9. ipc_roundtrip still covers all 38 commands |
| Commit | `911e6d0` on `cursor/phone-group-shape-4a8e`; merge `a311598` |
| PR | #15 |
| Merge状态 | Exclusive has the merge. → `main` still `BLOCKED`. PR #7 still not merged |
| 下一轮重点 | Landing review; remaining +86/paren shapes; denylist frozen-ten; intake crash; digit-reuse; NSIS comment; SECURITY tense; fakeCore; PendingForget; synthesis |
| NO_HIGH_VALUE | Tray/single-instance; NSIS data-dir hook; keys.dpapi create_new; cloud toggle; denylist/clinical; fixture corpus; fakeCore (truncation stays below bar); crash/WAL; IPC 38/38 roundtrips |

## ROUND 9 record

| Field | Value |
|---|---|
| ROUND | 9 |
| 子代理任务 | 10× Fable: Tauri capabilities; COPY_ZH twins; collect Bucket; audit forbidden-fields; wizard persist; Graph as_of; A2 vs COPY_ZH; headless-vs-ci; cfg(windows) vs STATUS; landable vs blocked |
| 发现问题 | None at HIGH_VALUE. Below-bar: `fakeCore.ts` forget-mismatch explanation is a truncated core notice (not user-visible) |
| 修复问题 | None. No product-code landing |
| 测试结果 | Reviewers re-ran pinned gates on `72d32a2`: session_e1 33; session_import 13; session_collect 11; session_projection 4; session_graph_correct 7; session_screens 11; vitest 190; contract.test 14 |
| Commit | this closeout (docs) |
| PR | #15 |
| Merge状态 | Exclusive → `main` still `BLOCKED`. PR #7 still not merged |
| 下一轮重点 | Tray/single-instance; NSIS hooks; keys.dpapi; redactor exemption; cloud toggle; denylist; fixture corpus; fakeCore notices; crash/WAL; IPC roundtrip coverage |
| NO_HIGH_VALUE | All ten. Four author requirements remain wired. Remainder is hosted CI, Win11 checklist, NSIS/DPAPI manual, file write v0.1.1, D55, D61, D63, P2 E1 mutex |

## ROUND 8 record

| Field | Value |
|---|---|
| ROUND | 8 |
| 子代理任务 | 10× Fable: eleven-route state; fileplan write-absence; memory/forget ledger; schema freeze vs index; E1 HITL/redirect/cap; config.json pin; injection; local-gate counts; Unicode/path; draft voice vs lock |
| 发现问题 | STATUS still quoted unique-trunk `478f19f` vitest 161 / `ipc_roundtrip` 50 as the current gate; this tip is vitest 190 / `ipc_roundtrip` 56 |
| 修复问题 | Docs-only: STATUS milestone, AC-26 row, 下一步, and SOUL-7C local-green line now name exclusive-tip counts beside the `478f19f` historical record |
| 测试结果 | Reviewers: vitest 190; ipc_roundtrip 56; command_surface 6; no_egress_path 3; one_store 3; shell_is_local_only 21; schema-freeze --check matches lock |
| Commit | this closeout (docs) |
| PR | #15 |
| Merge状态 | Exclusive → `main` still `BLOCKED`. PR #7 still not merged |
| 下一轮重点 | Tauri capabilities; COPY_ZH twins; collect Bucket; audit fields; wizard persist; as_of clock; A2 vs COPY_ZH; headless-vs-ci; cfg(windows) vs STATUS; landable vs author-machine |
| NO_HIGH_VALUE | Eleven-route state; fileplan write-absence; memory/forget ledger (P3 WAL-busy / headless execute_forget); schema freeze; E1 HITL remainder; config.json pin; injection; Unicode/path; draft voice vs lock |

## ROUND 7 record

| Field | Value |
|---|---|
| ROUND | 7 |
| 子代理任务 | 10× Fable: R6 landings review; remaining `store_opened`; remaining O(n); Import menu pin; projection-on-Graph; research remainder; `COMMANDS` 38 pin; STATUS/Windows honesty; Telegram `@` leak; author-four synthesis |
| 发现问题 | None at HIGH_VALUE. Below-bar: WP02 `## 阻塞 / 无` stale vs Goal 1 BLOCKED; STATUS header still said Round 6 in flight; smoke counts any projection closer rather than `MODERATE_TO_WEAK_KEY`; `forget_memory` closed-store copy uses plan-hash mismatch |
| 修复问题 | Parent docs only: STATUS header + WP02 阻塞 clarification. No product-code landing |
| 测试结果 | Reviewers re-ran pinned suites on `c470d67`: session_e1 33; e1_watch 5; telegram 12; session_import 13; events_by_source_index 2; headless_main_flow 5; e0-audit clean. Author-four: 29 focused session tests + 90 vitest across six requirement screens |
| Commit | this closeout (docs) |
| PR | #15 |
| Merge状态 | Exclusive → `main` still `BLOCKED`. PR #7 still not merged |
| 下一轮重点 | Eleven-route state; fileplan write-absence; memory/forget ledger; schema freeze; E1 HITL/redirect/cap; config.json pin; injection; local-gate counts; Unicode/path; draft voice vs lock |
| NO_HIGH_VALUE | All ten scanners. Landings hold. Projection reaches Graph. Research honors `research_export`. `COMMANDS` 38 three-way pin (no COMMANDS.md file; the pin is `core.ts`). `/audit` and research-preview full scans stay P2. Author four requirements wired; remaining BLOCKED are hosted CI, author Win11 checklist, file write v0.1.1, D55, P2 E1 mutex, D61 |

## ROUND 6 record

| Field | Value |
|---|---|
| ROUND | 6 |
| 子代理任务 | 10× Fable + Opus: closed-store E1, `events_by_source` index, Telegram `@` alias, e1-watch instrument; parent: D62, D63, Telegram full-export menu copy |
| 发现问题 | Opening a newer-stamped db wrote stamp 2 then used the old schema; E1 still reached a configured endpoint with the store closed; collect 1s poll full-scanned `events.source`; Telegram 2026 `@@username` folded twice; AC-21 headless never woke reqwest so lag was unmeasured; importer copy still named the per-chat export this adapter refuses |
| 修复问题 | Refuse newer `schema_version` without touching the file (D62); `prepare_draft`/`generate_draft` call `opened_store()?` first (paste stays store-free); additive `events_by_source` index, version stays 2; `username_handle` strips one leading `@` then prefixes once; `soul-headless e1-watch` watches the one path that opens a socket; D63 session-only collect consent; importer copy names Settings → Advanced → Export Telegram data |
| 测试结果 | schema_version 5; store_commands 2; events_by_source_index 2; telegram 12; e1_watch 5; session_e1 33; session_import 13; vitest Import 13. rustfmt --check clean on the copy commit |
| Commit | merges of e1-closed-store, events-source-index, telegram-username-alias, e1-netwatch-instrument; D62/D63 docs; `e093da1` Telegram menu copy |
| PR | #15 |
| Merge状态 | Exclusive has the merges. → `main` still `BLOCKED`. PR #7 still not merged |
| 下一轮重点 | Review the four landings; remaining store_opened; remaining O(n); Import three-way pin; smoke vs Graph UI; research remainder; COMMANDS=38; STATUS vs Windows BLOCKED; `@` leakage; author-four synthesis |
| NO_HIGH_VALUE | IPC argument sweep (38-command serde boundary refuses, no panic, no third-party echo); key-loss UI after `store_notice`; gate coverage (algo crates in e0; denylist exempt + runtime clinical screen); Session lock-order (P2 E1 mutex not upgraded) |

## ROUND 5 record

| Field | Value |
|---|---|
| ROUND | 5 |
| 子代理任务 | 10× Fable + Opus: forget×summary, research `research_export`, G1+ owner-fold, import-tx wrap, forgotten-peer tie-correct, E1 multiline provenance, DPAPI `store_notice`, checklist honesty |
| 发现问题 | Forgotten contact still summarized/E1-sent and resurrected Live on rebuild; research preview ignored `research_export`; two self ids minted two Owner rows; v1 offset timestamps; import fsync-bound ~3 tx/message; tombstone band buttons still live; endpoint line-break provenance; DPAPI unrecoverability unspoken; checklist overstated `install-smoke` |
| 修复问题 | `DraftError::Forgotten` + Active-only `known`; preview publishes owner+`bucket` only; `alias` merges + commit `AmbiguousOwner`; `to_utc` at v1 parse; `SqlCipherStore::transact` around commit+rebuild; `correct_tie`/`release_tie` refuse non-Active; `read_grounded_in` refuses line breaks; `store_notice` unrecoverability; checklist/STATUS honesty |
| 测试结果 | session_forget_summary 3; forget_rebuild 3; session_forget_tie_correct 3; session_research 1; research_preview 5; session_collect 11; session_import 13; session_crash 6; import_to_graph 9; soul_import_v1 13; people_summary 20; session_summary 4; session_commands 20 |
| Commit | merges of forget-summary-cite, research-disposition, g1-owner-fold, import-tx-wrap, forget-tie-correct; E1 multiline; D61 |
| PR | #15 |
| Merge状态 | Exclusive has the merges. → `main` still `BLOCKED`. PR #7 still not merged |
| 下一轮重点 | Schema-version; store_opened=false sweep; AC-21 headless lag; IPC argument sweep; aging; key-loss remaining surfaces; Telegram 2026; gate coverage; AD-12; Session lock-order |
| NO_HIGH_VALUE | R4 landings review (all eight hold); audit-crash remainder documented; import idempotency parked D55; Windows NSIS/DPAPI/tray BLOCKED as environment; empty-graph name residual is D61 |

## ROUND 4 record

| Field | Value |
|---|---|
| ROUND | 4 |
| 子代理任务 | 10× Fable + Opus: Session §6 pin, Graph/Profile same-band, name-harvest fold, forget-impact bind, collect `count_events`, as_of=None pin, keys.dpapi lock reclaim, E1 response cap |
| 发现问题 | keys.dpapi empty-file reclaim TOCTOU (10/20 empirical); Session §6 unpinned; Graph/Profile greyed current band; imported `李 雷` leaked as `李雷` on exemption; forget preview ids matched while impact drifted; collect_status O(n) JSON parse; E1 unbounded body; COPY_ZH §6 `{天数}` collision; as_of=None untested |
| 修复问题 | In-place fs4 lock reclaim; session_projection.rs; same-band pin UI+session test; spaced-Han fold; re-quote forget impact; SQL count(*); 4 MiB E1 cap; COPY_ZH `{降档天数}`/`{封弱天数}`; as_of=None crate pin |
| 测试结果 | soul-store 18+integration green incl. racing empty recoverers; session_projection 4; session_graph_correct 7; session_e1 32; redactor_exemption 14; session_screens 11; session_collect 10; soul-egress e1_origin 11; vitest 185; projection_sentences 21 |
| Commit | merges of predict-session-pin, graph-same-band, forget-impact-bind, collect-event-count, name-harvest-fold, e1-response-cap, key-blob-reclaim |
| PR | #15 |
| Merge状态 | Exclusive has the merges. → `main` still `BLOCKED`. PR #7 still not merged |
| 下一轮重点 | Cross-slice, forget×summary, import idempotency, G1+ edges, bounded perf. Do not re-litigate parked P2 E1 mutex without a P0/P1 upgrade |
| NO_HIGH_VALUE | Consent unused topics (honest for v0.1); collect-vs-E1 leak (none); SOUL-7B review passed; AC-31-on-this-tree was a misnomer |

## ROUND 3 record

| Field | Value |
|---|---|
| ROUND | 3 |
| 子代理任务 | 10× Fable (T4D review, independent, frontend, AC map, remaining modules, G2 UI gap, graph-UI review, prediction review, wizard+keys review, synthesis) + 5× Opus (graph UI, prediction, wizard, key-blob, intake ignored UI) |
| 发现问题 | T4D port genuine; SOUL-7A/7B/AD-9/wizard/key-blob needed; key-blob empty-file concurrent reclaim TOCTOU residual; projection lacks Session-level pin |
| 修复问题 | COMMANDS 36→38 graph correction UI; COPY_ZH §6 projection on person_summary; wizard `{reason_code,explanation}`; keys.dpapi `create_new`; Profile ignored receipt (SOUL-7B) |
| 测试结果 | Parent: `cargo test -p soul-draft -p soul-store -p soulcore` green (projection 20, session_graph_correct 6, session_e1 31, shell wizard 12). Fable: vitest 177 then 182; ipc_roundtrip 56 |
| Commit | merges of `predict-slice`, `key-blob-race`, `wizard-refusal`, `graph-correct-ui`, `intake-ignored-ui` |
| PR | #15 |
| Merge状态 | Exclusive line has the merges. → `main` still `BLOCKED`. PR #7 still not merged |
| 下一轮重点 | SOUL-7B merged this closeout; next: projection product pin, key-blob reclaim TOCTOU, consent scan, name variants |

## ROUND 2 record

| Field | Value |
|---|---|
| ROUND | 2 |
| 子代理任务 | 10× Fable (c441 delta, redactor, fileplan revoke, COPY_ZH draft, graph UX spec, intake receipt spec, research isolation, independent overlap check; G4/collect/policy-ipc Fable+Opus hit async cap) + 2× Opus (T4D port, store-forget) |
| 发现问题 | c441 already closed E1 origin-in-hash; remaining: wizard error shape, key-blob race, consent lock vs egress, name spelling variants, graph UI for correction, prediction sentences |
| 修复问题 | T4D/G1+/G2/G3/A2 ported from PR #7 (not merged); forget `secure_delete`+checkpoint+destroyed-id ledger |
| 测试结果 | Opus T4D: `just ci` green (vitest 14/172). Parent post-merge: `cargo test -p soul-graph -p soul-store -p soul-profile` all green (T4D product/parity/correction + forget ledger) |
| Commit | merge `cursor/port-t4d-4a8e`, merge `cursor/store-forget-4a8e` |
| PR | #15 updated; sub-branches pushed |
| Merge状态 | Sub-branches merged into exclusive line. Exclusive → `main` still `BLOCKED`. PR #7 still must not be merged |
| 下一轮重点 | Review T4D; graph correction IPC/UI; prediction projection; wizard refusal; key-blob race |

Fileplan revoke: defer v0.1.1 (Fable #4). Collect consent persistence: product board, not a silent code change. E1 plan-hash: already on c441 — do not re-do.

## ROUND 1 record

| Field | Value |
|---|---|
| ROUND | 1 |
| 子代理任务 | 10× Fable scanners: frontend, backend, API/IPC, data, auth, graph/T4D, soul layer, prediction, tests/CI, independent review |
| 发现问题 | See findings below |
| 修复问题 | None in R1 (scan only). Parent FF'd exclusive branch onto c441 (git, not product code) |
| 测试结果 | Independent review ran vitest 14/161, `session_e1` 22/22, matrix replay, xtask e0/denylist/schema-freeze — green on the pre-FF tree; post-FF not re-run yet |
| Commit | tracker docs; rebase onto `85aae68` |
| PR | #15 |
| Merge状态 | `BLOCKED` to `main` (same contract-tree as #2). Merge of #7/#4 forbidden. Parent cannot merge PRs (no merge tool) |
| 下一轮重点 | Port T4D onto FF'd line; independent store/policy/IPC fixes; prediction thin slice waits for T4D |

## Coverage map (after Round 3 landings; Round 4 scanning)

Do not treat the Round 1 findings list below as still-open. T0 / G1+ / G2 / G3 / wizard shape / forget pages / prediction crate path closed in R2–R3.

| Area | Evidence on exclusive tip `d16e4cf` | Remaining honesty vs this Goal |
|---|---|---|
| Frontend | 11 routes + wizard; Graph band buttons + lock + `machine_band` (SOUL-7A); Profile ignored receipt (SOUL-7B); COPY_ZH 弱/中等/强 | Graph may disable same-band pin; projection sentences only via existing points list |
| Backend | Session is the one stateful module; T4D rebuild; G1+ import; G2 intake lock; `Session::correct_tie` / `release_tie` | Name-harvest spacing variants; unused ConsentTopics |
| API | 38 commands, three-way pinned (`correct_tie` / `release_tie`) | No predict command (AD-13: none wanted); directory revoke is v0.1.1 (AD-11) |
| Database | SQLCipher + typed `tie_strength` + CK + forget atomic + destroyed-key ledger | Residual keys.dpapi empty-file reclaim TOCTOU |
| Login & permissions | Local DPAPI key chain; no cloud login | Expected |
| Storage | `secure_delete` + WAL checkpoint after forget; `create_new` key blob | Same TOCTOU residual; do not touch `forget.rs` in R4 keys work |
| Cache | None found | — |
| Third-party | Import + E1 redactor; origin in plan hash (c441); KnownIdentifiers from opened labels | Unspaced variant of imported spaced name; unimported name on exemption path needs PRODUCT_LOCK ruling |
| Core business | Soul + agent draft-only, file preview not write; T4D + A2 + axis/tie correction | Thin prediction not pinned at `Session::person_summary`; no learned model (by lock) |
| Tests | AC-02–25 product-path; T4D crate/store/parity; projection 20 crate tests; graph correct 6 | Session-level §6 pin; AC-31-like real SQLCipher if that AC exists; AC-34 only full import→store path |
| Build | Tauri 2 + pnpm + cargo + just | NSIS author-manual |
| CI/CD | Auto-push `main` + unique trunk only (AD-10) | Hosted empty runner `BLOCKED` |
| Performance | Unmeasured | Prefer NO_HIGH_VALUE unless unbounded path proven |
| Security | E0 type-absent; HITL for E1; wizard `{reason_code,explanation}` | Forget vs capability tokens; unused ConsentTopics — R4 Fable #1/#9 |
| Reliability | Crash harnesses, one-store, single-instance | Author Win11 checklist open |

## Findings this round (R1)

1. **P0 G1:** this tree still bands with T0 3/10/3 in `crates/soul-graph/src/build.rs`. Frozen answer is T4D. Algo crates not in workspace.
2. **P0 G1+:** owner group messages still fan Outgoing to every historical speaker (`soul-import/src/commit.rs`).
3. **P0 G2:** `intake` calls `place_axis` without lock check; locked axes move while still showing the lock badge.
4. **P0 G3:** rebuild writes `UserVerdict::Unreviewed`; no product `correct_tie`.
5. **P0 topology:** PR #7 has the port complete but is behind c441 honesty/tests. Port, do not merge. Exclusive branch is now FF'd onto c441 so the port does not collide with 77 already-landed commits.
6. **Prediction:** thinnest lock-compatible slice is T4D demotion clock read forward (bands/dates, no model, no new WP, no new command). Lands after T4D. Research preview already satisfies v0.1 data-collection preview (D8).
7. **Store:** forget does not `secure_delete` / checkpoint, so wrapped CK can remain in free pages while DEK still opens the file.
8. **IPC:** `complete_wizard` refusals are not `SessionRefusal` shape.
9. **Policy:** E1 plan hash ignores origin; changing endpoint after prepare still sends.

## Fixes this round

- Fast-forward exclusive branch onto `85aae68` (AD-8). Tracker docs only.

## Opus dispatch (Round 2)

| Owner | Scope | Must not touch |
|---|---|---|
| Opus T4D | Port from `origin/cursor/goal1-unblock-a073` per Fable #6 brief | Frozen algo rules; PR #7 wholesale merge; `soul-store` forget path |
| Opus store | `secure_delete` + checkpoint after forget | graph/import/profile/draft |
| Opus policy/IPC | Wizard refusal shape + E1 origin in plan hash | graph/T4D files |

## Commits / PRs

| When | What | SHA / PR | Merge |
|---|---|---|---|
| R1 start | Exclusive branch tracker | `3c6354f` (rebased) / #15 | Not to `main` |
| R1 end | FF onto c441 + this record | (this commit) / #15 | Same |
| R2 | Opus T4D port, 7 commits `f7a1a88`…`aac2b39` on `cursor/port-t4d-4a8e` | branch off `9ca342a` | Onto the exclusive line only |
| R2 | SOUL-7A tie-correction UI+IPC, 2 commits `2a7e2e1` / `fc54529` on `cursor/graph-correct-ui-4a8e` | branch off `cursor/soul-integration-4a8e` | Onto the exclusive line only |

### Opus T4D port (Round 2)

Sources copied from `origin/cursor/goal1-unblock-a073` (verified 2026-08-25 at
`6133307`), never by merging PR #7. Phase per commit: frozen algo crates into
the workspace; soul-graph T4D plus the typed `tie_strength` schema and its lock
digest in one commit; soul-import G1+ attribution and the D37/D38 clocks;
soul-draft on the frozen A2 renderer; soul-profile G2 intake lock with the
soulcore correction commands and the xtask denylist exemptions; the desktop
hunk merge; and the three gap tests (AC-29 self-heal at the store, AC-34
`last_contact`, fixture parity against `score()`, D52 constant equality).

Hand-merged both ways rather than taken from one side: `soul-import`'s
`commit.rs` and `tests/telegram.rs`, `soulcore`'s `session_import.rs`,
`soul-draft`'s `analysis.rs` and `tests/people_summary.rs`, and the four
desktop TypeScript files. No IPC command was added by this port: `COMMANDS` was
still 36 at `aac2b39`. SOUL-7A takes it to 38.

`just ci` green locally on `aac2b39`. Hosted still billing-blocked, and the
`src-tauri` sub-workspace was not built here (no GTK/WebView stack on this
box).

### SOUL-7A tie-correction UI+IPC (Round 2)

`cursor/graph-correct-ui-4a8e` off `origin/cursor/soul-integration-4a8e`, two
commits `2a7e2e1` (core + IPC) and `fc54529` (screen). PR #7 not merged, `main`
not merged, trunk not rebased.

The T4D port left `soul_graph::correct_tie` and
`soulcore::commands::graph::correct_tie` with no product path: no `Session`
method, no command, no `COMMANDS` key, so the three band words on `/graph` were
three words. Constraint 10 says the user overrules the machine, and on the
installed build they could not. Closed by `Session::correct_tie` /
`Session::release_tie` (shaped like `correct_axis`: the band vocabulary is
checked at this layer, the whole `PeopleGraphView` comes back), two forwarding
commands taking `COMMANDS` 36 → 38, `correctTie` / `releaseTie` in `core.ts`,
and a `Tie` that renders band buttons, a lock badge, the `machine_band`
disagreement line and a release button — the same lock shape `Profile.tsx` uses.

New gates: `soulcore/tests/session_graph_correct.rs` (6, including three
refusals), `ipc_roundtrip` 54 → 56, `Graph.test.tsx` 10 → 14, and a stateful
`correct_tie` / `release_tie` in `fakeCore` so those four are not empty
assertions. Untouched: `soul-store` keys, COPY_ZH prediction keys, wizard
refusal, forget path, schema, `StoredConfig`, `config.json`.

Local green: `cargo test -p soulcore`, `cargo test --manifest-path
apps/desktop/src-tauri/Cargo.toml --all-targets` (`ipc_roundtrip` 56,
`command_surface` 6, `no_egress_path` 3, `one_store` 3, `shell_is_local_only`
21), vitest 14 files / 176, fmt, clippy `--all-features -D warnings`,
schema-freeze, e0-audit, denylist-audit, sbom, fixture corpus, smoke-lint.
Hosted still billing-blocked.

## Blocked

- Hosted Actions runners (billing/spending).
- Merge to `main` until M2 after T4D+schema.
- Merge of PR #7 / #4 / #2 (contract or D49).
- Author Win11 manual checklist.
- Parent has no merge-PR capability — record and continue.
- Prediction thin slice unblocked: T4D is on `cursor/port-t4d-4a8e`.
- ~~Desktop shell cargo tests (`apps/desktop/src-tauri`) cannot run on this box:
  the GTK/WebView system libraries are missing, so only its `Cargo.lock` was
  refreshed.~~ Cleared on the SOUL-7A box: `libwebkit2gtk-4.1-dev` and
  `libgtk-3-dev` installed, `--all-targets` green there. Still per-box, not a
  repo change.

## Next round focus

Round 4 Fable in flight. After they return: Opus on confirmed HIGH_VALUE only (key-blob TOCTOU, Session §6 pin, Graph same-band if confirmed). Do not start empty Goal 2 polish. Do not merge PR #7 / #4 / to `main`.
