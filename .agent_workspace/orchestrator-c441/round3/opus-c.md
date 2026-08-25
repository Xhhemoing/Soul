[Model: claude-opus-5-thinking-high-fast]
round: BUILD R3
role: implement
branch: `cursor/goal1-build-audit-c441`

## scope

P1-1, P1-2 and P1-3 from `round3/gpt-sol-b.md`. Test-only work plus one product
fix on `Memory.tsx`. Files touched, and nothing else:

- `apps/desktop/src-tauri/tests/ipc_roundtrip.rs`
- `apps/desktop/src/routes/Memory.tsx`
- `apps/desktop/src/routes/Memory.test.tsx`
- `apps/desktop/src/test/fakeCore.ts` (forget/preview state only)
- `crates/soulcore/tests/session_screens.rs` (research snapshot only)

## done

### P1-1 — the false-green IPC success paths are gone

All 14 `return;` escape hatches are removed. Every acceptance precondition —
`commit_soul_import_v1`, `commit_telegram`, `preview_soul_import_v1`,
`preview_telegram`, `create_memory`, `answer_questionnaire`, `audit_chain`,
`people_graph` — is now an `expect`. The green mutant the probe named (stub a
command to always return `{reason_code, explanation}`) now fails the tests that
claim AC-03/04/05/06/08/12/14/15/16/20/23/25 rather than exiting before their
assertions.

The state those branches were tolerating is real and is now its own test rather
than an alternate pass branch:

- `a_store_that_will_not_open_is_a_coded_refusal_rather_than_an_empty_answer`
  points a `Shell` at a directory whose parent is a regular file, so
  `create_dir_all` and `open_store` both fail for a reason that has nothing to
  do with the host's key provider. It asserts `session_status.store_opened` is
  false first (otherwise the loop proves nothing), then requires each of seven
  commands to refuse with `ROUTINE`, with `STORE_UNAVAILABLE_NOTICE` in the
  explanation, and with exactly two fields on the record. An empty graph, an
  empty memory list and an empty chain would look on screen exactly like a Soul
  nobody has used yet, which is why the refusal is the contract.

The module doc now says why the acceptance tests require success, so the next
reader does not reintroduce the branch as a kindness to CI.

### P1-2 — the forget retry contract now reaches the product boundary

`session.rs` was not edited. It already matches before it takes; the drift was
above it.

- **IPC.** `a_memory_is_written_read_edited_and_forgotten_over_the_ipc` no
  longer re-prices after the mismatched confirmation — it sends the *same*
  preview the refusal did not consume, which is the claim — and then replays
  the spent confirmation and requires `PLAN_HASH_MISMATCH`.
- **IPC, new.** `a_forget_refused_over_the_ipc_leaves_the_preview_the_user_read_standing`
  writes two memories and drives the other half of the match: wrong preview id
  with the right memory, then the right preview id with the wrong memory,
  neither destroying anything, then the correct confirmation going through. It
  also pins two `hitl.deny` rows and one `forget.execute` on the chain the 审计
  page reads back, with no title or summary on it. This is the layer the probe
  said the contract stopped at: the held preview is one session's state, and a
  shell that opened a session per invoke would fail here while `soulcore`
  stayed green.
- **UI.** `Memory.tsx` cleared the displayed preview *before* the call and never
  restored it, so a refusal dropped the price the user had just read while the
  core was still holding it. The preview is now cleared only in the success
  branch. That is the one product change in this round.
- **Fake.** `fakeCore.ts` now keeps `heldForget` the way the session keeps
  `held_forget`: `preview_forget` records what the core issued, `forget_memory`
  matches both halves before taking, a mismatch leaves the preview standing and
  a success spends it. `pricing` still shapes the document the screen is handed,
  which is how a test says the panel is quoting a preview the core is not
  holding.
- **UI tests.** Exactly one `forget_memory` per click; the refused panel still
  showing the same `preview_id` with an enabled button and no silent
  re-pricing; the core's preview surviving that refusal (the correct
  confirmation still returns `matched_preview`); and after a successful forget,
  no panel to press and a replayed confirmation refused.

Mutation-checked: putting `setPreview(null)` back before the call turns
`被挡下来之后，用户读过的那份预览还在屏幕上，核心也还留着它` red.

### P1-3 — AC-20's "writes nothing" is now about bytes

Both `session_screens.rs::the_research_preview_writes_nothing_and_the_chain_holds_no_prose`
and `ipc_roundtrip.rs::the_research_preview_crosses_the_ipc_as_counts_and_no_third_party_row`
snapshot a sorted `(relative path, length, SHA-256)` footprint of the whole data
directory before and after the product call, and assert the snapshot is
unchanged. Both also assert the footprint is non-empty first, so the comparison
cannot hold because there was nothing there.

SQLite's `-shm` is excluded and the exclusion is documented: it is the
write-ahead index, holds none of Soul's data, is rebuilt from the log, and is
stamped by the act of taking a read lock. The `-wal` is included, which is where
an appended audit row lands.

The digest is spelled out in each test file rather than added as a dependency —
the desktop crate is its own workspace and depends on `soulcore` alone, and
neither `xtask e0-audit` nor `deny.toml` should have to grow an edge for one
assertion. `the_digest_agrees_with_the_published_vectors` checks the three FIPS
180-4 vectors, so a hash that answered a constant cannot make the comparison
pass.

Mutation-checked: inserting a `write_memory` between the two snapshots turns the
`soulcore` test red on both the length and the digest of `soul.db-wal`. The
store-level exclusion and leakage tests in `soul-store/tests/research_preview.rs`
were not touched.

## tests

- PASS `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test ipc_roundtrip` — 54/54 (was 51; +3: the closed-store refusal, the forget retry contract, the digest vectors).
- PASS `cargo test -p soulcore --test session_screens` — 11/11.
- PASS `pnpm exec vitest run src/routes/Memory.test.tsx` — 13/13 (was 11).
- PASS `pnpm exec vitest run` (full desktop suite) — 170/170 across 14 files.
- PASS `pnpm exec eslint` on the three changed TypeScript files; `pnpm exec tsc --noEmit` clean.
- PASS `cargo fmt --check -p soulcore` (0 diffs) and `cargo clippy -p soulcore --tests` (0 warnings). The desktop sub-workspace has 7 pre-existing `cargo fmt` diffs at baseline and still has exactly 7 — none of them mine.

## not done / left open

- P2-1 (`Draft.test.tsx` approval cardinality and a generate-stage redirect
  refusal) and P2-2 (`Research.test.tsx` asserting `research_preview` was
  invoked) were not dispatched to me and are untouched.
- N4/N5/N6 are unaffected. Nothing here closes Goal 1, and local green is not
  hosted green.

## do_not_touch — held

No change to `redactor`, `e1.rs`, `analysis.rs`, `Graph.tsx`, `Wizard.tsx`,
`session_e1.rs`, `draft.rs`, `net_guard.rs`, `soul-graph`, or any schema.
`session.rs` was read and left alone — Memory did not need it. No Cargo.toml,
lockfile, threshold constant or frozen plan was edited, and no crate test was
weakened.

## note for the orchestrator

This worktree is shared with at least one other agent. Mid-session the tree
briefly did not compile because `crates/soul-draft/{reply,analysis,draft}.rs`
were half-edited by somebody else; it recovered on its own and my runs above are
from the recovered tree. My commits stage only my five files.

`git push` is failing for everyone on this VM — the token in the `origin` URL
and in `gh`'s config are both rejected (`Invalid username or token`), and the
branch is ahead of `origin` by other agents' commits as well as mine. The work
is committed locally on `cursor/goal1-build-audit-c441`; it needs a refreshed
credential to reach the remote.
