[Model: claude-opus-5-thinking-high-fast]
round: BUILD R3
role: implement
branch: `cursor/goal1-build-audit-c441`

## scope

The P0 from `round3/gpt-sol-a.md`: 姓名 reaching the user's endpoint out of a
body the user exempted for its 正文, on a Soul that has imported nobody. Files
touched, and nothing else:

- `crates/soul-policy/src/redactor.rs`
- `crates/soul-policy/tests/redactor_exemption.rs`
- `crates/soul-policy/tests/redactor_leakage.rs`
- `crates/soulcore/tests/session_e1.rs`
- `crates/soulcore/src/commands/draft.rs` (two doc comments; the notice string
  itself is unchanged, see *not done*)

Commits: `8cded4c` (policy + crate tests), `9b4bcd4` (product test), `e14425c`
(doc). Pushed.

## done

### the leak

`E1_PLAN_NOTICE` and the wizard's welcome page both promise
「姓名与账号两种情况下都占位」 with no condition attached. The redactor kept the
promise out of `KnownIdentifiers`, which `soulcore::commands::draft::known_identifiers`
fills from the third-party contact rows — so on a Soul that has imported
nothing the set is empty, and the shape scrub has nothing to fall back on:
`13800138000` and `@xiaoming` have shapes, `李 雷` is two ordinary characters
and a space. Everywhere but one place that costs nothing, because a
third-party turn is a placeholder whole. The exception is the single turn a
second confirmation exempts, which is the only prose that leaves verbatim —
and that is exactly the screen where the promise is made.

### the fix

`Redactor::build` now sends an exempted turn through `scrub_exempted_original`
instead of `scrub_identifiers`: everything the identifier set and the existing
shapes already did, plus one more shape.

`scrub_spaced_label_shapes` replaces a **spaced Han display label** with
`NAME_PLACEHOLDER`: two to four groups of one or two Han characters, joined by
single spaces (`U+0020` or `U+3000`), six characters at most, anchored at the
head of a Han run so it cannot match the tail of a word. That is how an export
spells a display name — `result_basic.json`'s personal chat `name` and every
`from` on the peer's messages are `李 雷` — and it is not how Chinese prose is
written, which is what makes the spacing a signal rather than a guess.

Three properties worth stating because they are what keep it from being an NER:

- **It is a shape, not a lexicon.** No surname list, no name model, nothing
  clinical. A name written without the spacing — `李雷` inside a sentence — is
  not distinguishable from ordinary words by any rule this file could hold, and
  the code declines to guess at one. That case is `KnownIdentifiers`'s, and it
  is why `known_identifiers` reads the contact rows.
- **It is scoped to the exempted turn.** The default path is byte-identical, so
  `a_confirmed_exemption_carries_exactly_one_original_and_the_next_draft_does_not`
  ("the draft after an exemption must be identical to the one before it") still
  means what it meant. `scrub_identifiers` — the public one, used for file
  names and the summary body — is untouched.
- **It fails closed on the whole run, not halfway.** A spaced run longer than a
  name could be is left alone rather than truncated to its first few
  characters; half a placeholder in the middle of a sentence would be a worse
  answer than the sentence, and a run that long is not the shape.

The exemption still buys what it is for. `李 雷 说周五的场地他已经订好了` goes
out as `[姓名已占位] 说周五的场地他已经订好了`: one name placeheld, not the
turn, and no `[第三人正文已占位]`. Emails, `@handles` and 7+ digit runs are
untouched by this change and still go.

### the test that said it was not a bug

`session_e1.rs::with_nothing_imported_the_same_name_is_a_word_like_any_other`
asserted `李 雷` **reaching** the mock, and its doc called that the shape
scrub's limit rather than a defect. Inverted, and renamed
`with_nothing_imported_the_same_name_is_placeheld_by_its_shape`: the same
paste, the same second confirmation, and now no name in the bytes, a
`NAME_PLACEHOLDER` that is present, 「场地」 still travelling, and no
third-party placeholder. Its doc says what the old reasoning got right (the
scrub genuinely cannot see it) and where it went wrong (PRODUCT_LOCK does not
make the promise conditional on an import, and the empty graph is the *common*
case for a first run, not the corner).

The control that test used to provide is replaced rather than dropped. `李 雷`
is now covered twice over — the set after an import, the shape without one — so
`a_name_this_soul_imported_is_placeheld_even_in_a_body_the_user_confirmed`
would pass for either reason. New:
`a_display_name_with_no_shape_is_placeheld_because_the_graph_learned_it` uses
this export's *other* sealed label, `Wang Xiao`. Two capitalized words are how
English writes most of a sentence, so no shape can recognize it and only the
contact rows can placehold it — the assertion is again about the graph being
wired. It reads the label out of the store first, so it rests on the database
rather than on a string copied from the fixture.

Crate-level, in `soul-policy`:

- `an_exempted_turn_placeholds_a_display_label_nobody_registered` — an empty
  `KnownIdentifiers` (asserted empty), an exempted turn, no label out,
  placeholder in, original prose still there.
- `the_label_shape_leaves_the_prose_the_exemption_was_for_alone` — five strings
  that must come back byte-identical, including `Roy 说这周先把方案定下来` (the
  owner's own name: PRODUCT_LOCK's placeholder is 第三人姓名, and a rule that
  ate capitalized words would redact the user out of their own draft),
  `下午 3 点，第 2 会议室，预算 45000`, and `他在 café 里等了很久`. A shape that
  fired on prose would take the exemption back by another route.
- `the_default_path_is_unchanged_by_the_label_rule` — the label rule does not
  reach `redact_for_e1`, asserted on a conversation that carries a label.
- `redactor_leakage.rs::a_display_label_nobody_registered_is_placeheld_inside_an_exempted_turn`
  — the same claim through `LeakageChecker`, with the identifier set empty so
  the placeholder can only have come from the shape.

## AC mapping

| AC | claim | where it is now proved |
| --- | --- | --- |
| AC-12 | 姓名/账号 placeheld in the E1 body, import or no import | `redactor_leakage.rs::a_display_label_nobody_registered_is_placeheld_inside_an_exempted_turn`; `session_e1.rs::with_nothing_imported_the_same_name_is_placeheld_by_its_shape`; `session_e1.rs::a_display_name_with_no_shape_is_placeheld_because_the_graph_learned_it` (the graph half, un-shadowed) |
| AC-13 | one confirmation carries one 正文, once, and nothing else | `redactor_exemption.rs::an_exempted_turn_placeholds_a_display_label_nobody_registered` + `the_label_shape_leaves_the_prose_the_exemption_was_for_alone` (the exemption still buys the message) + `the_default_path_is_unchanged_by_the_label_rule` (not remembered, unchanged) |
| AC-13 / product | the screen's promise is true on the machine that reads it | `session_e1.rs` as above, on bytes a real loopback `MockLlm` received |

PRODUCT_LOCK is not weakened anywhere: nothing that was placeheld before is
sent now, and `redact_for_research` still has no exemption parameter
(`the_research_path_has_no_exemption_entry_point` still passes, including its
source-shape assertion that exactly one function consumes an exemption).

## tests

Run from a clean worktree of `HEAD` with only my four files applied, because
the shared workspace did not compile for part of this session (see *note*).

- PASS `cargo test -p soul-policy --test redactor_exemption --test redactor_leakage` — 9/9 and 9/9 (was 6 and 8).
- PASS `cargo test -p soulcore --test session_e1` — 28/28 (was 27; one inverted, one added).
- PASS `cargo test --workspace` — 110 test binaries, 0 failures.
- PASS `cargo test --test ipc_roundtrip` in the desktop sub-workspace — 51/51, including `an_imported_name_is_placeheld_over_the_ipc_even_in_a_body_the_user_confirmed` and both `includeOriginal` roundtrips.
- PASS `cargo fmt --check -p soul-policy -p soulcore`.
- No vitest run: no UI copy changed. `Wizard.tsx`'s
  「姓名与账号两种情况下都占位」 was already the right sentence — it was the
  code that was not keeping it — so `Wizard.test.tsx:152` still passes
  unmodified and the page is now honest.

Local green is not hosted green.

## not done / left open

1. **A Latin display label is still only covered by the contact graph.** `Wang
   Xiao` in an exempted paste on a Soul that has imported nothing goes out
   intact. Closing it means matching capitalized Latin bigrams, which eats
   ordinary English (`See you Monday Tom`) inside the one body the user
   explicitly confirmed — worse than the leak for a user drafting in English,
   and the instruction not to invent an NER points the same way. It is written
   down where the shape is defined rather than pinned by a test asserting the
   leak. If the product wants it closed, the honest lever is the import path
   (more labels in the set), not a cleverer shape.
2. **The notice's wording is unchanged.** I wrote a stronger clause —
   「按原文带上的是那一段正文，不包括里面的姓名与账号，也不取决于你有没有导入
   过联系人」 — and reverted it: `E1_PLAN_NOTICE` has a byte-for-byte twin in
   `apps/desktop/src/test/fakeCore.ts` that `contract.test.ts` compares, and
   `fakeCore.ts` is another agent's file this round. The sentence that is there
   already carries the promise; only its emphasis would have improved. Whoever
   owns `fakeCore.ts` next can take both halves together.
3. `person_summary` was not touched, `Wizard.tsx` was not touched, and no
   threshold, schema or frozen document was edited. No Goal 2, no PR #7 absorb.

## note for the orchestrator

This checkout is shared with at least one other agent, and `git status` showed
in-flight edits to `soul-draft/{reply,analysis,draft}.rs`, `soul-policy/e1.rs`,
`soulcore/commands/{draft,policy}.rs`, `ipc_roundtrip.rs`, `Memory.tsx` and
`fakeCore.ts` that were not mine. The tree did not compile for part of the
session (`ReplyDefect::NotARewrite` added to `reply.rs` before `draft.rs`
matched on it), so every number above comes from `git worktree add /tmp/opus-a-wt
--detach HEAD` with only my files copied in, and its own `CARGO_TARGET_DIR`.

My commits stage individual paths, never `-A`. The `draft.rs` doc commit was
staged as a single hunk with `git apply --cached` so the other agent's
`Rephraser`/`PersonSummaryView` edits in that same file stayed out of it and
stayed in their working tree. `git push` worked from here.
