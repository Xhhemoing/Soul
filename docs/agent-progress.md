# Agent progress

Parent exclusive branch: `cursor/soul-integration-4a8e`.
Working base: `origin/cursor/first-test-candidate-c441` @ `85aae68` (strict FF of unique trunk `cursor/soul-goal1-7b1c` @ `5309656`, AD-8).
Goal: complete Soul so it delivers (1) multi-dimensional interpersonal and self analysis, (2) work and social assistance, (3) simple behavior and group prediction, (4) data collection for continuous optimization.
Process: Fable (`claude-fable-5-thinking-xhigh`) scans/reviews; Opus (`claude-opus-5-thinking-high-fast`) lands code. Commit one purpose at a time. PR when there is a real artifact. Merge when safe; otherwise record `BLOCKED` and continue.

## Current round

**ROUND 5** — cover under-scanned crossings (forget×summary, import idempotency, audit crash, G1+ edges, bounded perf). Do not re-open closed R4 items.

| Field | Value |
|---|---|
| Current task | Round 5 Fable still in flight (cross-slice, forget×summary, R4 review, G1+, perf). #3 D55 park; #5 audit crash NO_HIGH_VALUE; #6 usage-policy BLOCKED, replaced by #6b HIGH_VALUE multiline provenance (Opus in flight); #9 Windows checklist honesty landed; #10 DPAPI unrecoverability notice landed on `store_notice`. E1 mutex stall parked P2. Hosted Windows `LockFileEx` pending billing. |
| Parent model | product/account setting |
| Hosted CI | `BLOCKED` — billing/spending |
| Exclusive tip | key-blob reclaim merged; PR #15 |

### Round 5 Fable dispatch

| # | Direction |
|---|---|
| 1 | Cross-slice: import → lock → graph → summary+projection → collect → research → audit |
| 2 | Forget × summary: destroyed evidence must not stay cited |
| 3 | Import idempotency: same `result.json` twice must not inflate bands |
| 4 | Independent review of R4 landings (no mechanical re-scan of closed defects) |
| 5 | Audit-chain crash consistency at `Session::append_audit` |
| 6 | Hostile E1 remainder (control chars / diagnosis) after the 4 MiB cap — **BLOCKED** by usage-policy filter; replaced by E1 reply screening #6b |
| 7 | G1+ post-port: multi-venue same-pair, owner in two groups |
| 8 | Bounded performance probe (synthetic import→rebuild→summary) or evidenced NO_HIGH_VALUE |
| 9 | Windows-only / author-checklist desk-check (record BLOCKED if unrunnable) |
| 10 | Independent synthesis vs four author requirements; Round 6 directions |

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
