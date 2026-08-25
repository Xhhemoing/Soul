MODEL: gpt-5.6-sol-xhigh-fast

# WP10/WP11 optimization-gap probe

Scope: read-only review of `SOTA_BARS.md`, `crates/soul-draft`, `crates/soul-fileplan`,
`soulcore` draft/fileplan command tests, session-store opening, and the audit leakage
checker. No Goal 2, UI, DPAPI implementation, or file-write execution is proposed.

## Must-fix gaps

### 1. D-01 dependency guard does not ban messaging transports

- **Where:** `crates/xtask/src/egress.rs:35-48`;
  `crates/xtask/tests/self_test.rs:149-171`
- **Attack:** add `lettre`, `async-smtp`, `imap`, `async-imap`, `teloxide`,
  `grammers-client`, `matrix-sdk`, `tokio-tungstenite`, or `tungstenite` as a normal
  dependency of a shipped crate. The dependency audit only knows HTTP clients and Tauri
  plugins, so D-01's first layer stays green.
- **Smallest assertion:** extend the banned dependency set with the SOTA list and add a
  synthetic-metadata control proving a normal/build `lettre` edge is reported from its
  shipped root.

### 2. D-01's source check is a word blacklist, not a public-surface lock

- **Where:** `crates/soul-draft/tests/no_send_api.rs:14-22,47-53`
- **Attack:** add `DraftOutcome::forward_to`, `publish`, or `dispatch` taking a callback or
  transport trait. It need not contain `fn send`, `smtp`, `sendmail`, `deliver`, or
  `transmit`, so every current check passes while the draft type gains an acting API.
- **Smallest assertion:** source-parse the `DraftOutcome` impl and assert its public method
  names exactly equal the approved constructor/getter allowlist; add a synthetic
  `pub fn forward_to` control.

### 3. D-03 can pass with a mostly constant wire body

- **Where:** `crates/soulcore/tests/draft_commands.rs:368-398`
- **Attack:** always send the three placeholders plus the hard-coded prefix
  `好，我回复`, while ignoring the actual owner turn. The leakage check, placeholder checks,
  and current owner assertion all pass.
- **Smallest assertion:** make two drafts with distinct runtime-generated owner sentinels;
  assert each raw `RecordedRequest.body` contains its own sentinel and not the other's.

### 4. D-04 never puts an `@handle` in the exempted turn

- **Where:** `crates/soulcore/tests/draft_commands.rs:65-76,430-475`
- **Attack:** an exemption implementation can continue redacting names and phone numbers
  but release handles/accounts from exempted prose. The account fixture only occurs in the
  owner turn, so this passes.
- **Smallest assertion:** include the fixture account in `third_party_line()`, then assert
  the exempted raw body omits it and contains `ACCOUNT_PLACEHOLDER`.

### 5. D-04 does not pin the exempted turn ID in audit

- **Where:** `crates/soulcore/tests/draft_commands.rs:484-490`;
  `crates/soulcore/src/commands/policy.rs:242-255`
- **Attack:** audit only the generic `ThirdPartyBodyIncluded` reason, or attribute the
  exemption to the wrong turn. Current tests inspect reason codes only; the implementation
  does not put the exempted turn in `subject_refs`.
- **Smallest assertion:** select the allowed exempted `EgressRequest` entry and assert
  `subject_refs == [exempted_turn]`, while retaining the no-prose leakage assertion.

### 6. D-07's configured-summary/model-output cases are absent

- **Where:** `crates/soulcore/src/commands/draft.rs:311-355`;
  `crates/soulcore/tests/draft_commands.rs:802-846`;
  `crates/soul-testkit/src/mock_llm.rs:223-231`
- **Attack:** add a configured-endpoint branch that accepts a diagnostic model answer or
  model-supplied evidence UUIDs. All summary tests use the local path, and `MockLlm` cannot
  return a chosen answer.
- **Smallest assertion:** with a configured mock, assert the current local-only design makes
  zero requests and returns the same locally resolved IDs. If model summaries are intended,
  make the mock response configurable and assert diagnostic text is rejected and forged
  UUIDs never appear in claims.

### 7. D-08/F-04 do not test `ActionKind::ALL`

- **Where:** `crates/soulcore/tests/draft_commands.rs:679-691`;
  `crates/soulcore/tests/fileplan_commands.rs:202-247`
- **Attack:** allow external content to request one of the uniterated actions, such as
  `PreviewResearch` or `GenerateWithUserEndpoint`. Draft tests cover two actions,
  fileplan covers two, and the separate policy test samples only `ExecuteForget`.
- **Smallest assertion:** iterate `ActionKind::ALL`, build any required plan/token setup,
  and assert every external-origin request returns
  `ExternalContentNotAuthority` before spending a token.

### 8. D-08's decoy is not one of the pasted URLs

- **Where:** `crates/soulcore/tests/draft_commands.rs:621-677`
- **Attack:** execute/fetch every URL in the paste corpus. They point to
  `evil.example`, while the asserted-zero decoy has an unrelated random loopback URL, so
  its count remains zero.
- **Smallest assertion:** append `decoy.chat_completions_url()` to an actual pasted
  injection turn, run `draft_reply`, and assert that same decoy remains at zero.

### 9. D-08 does not exercise a tool-call-shaped model response

- **Where:** `crates/soulcore/tests/draft_commands.rs:11-15,617-742`;
  `crates/soul-testkit/src/mock_llm.rs:223-231`
- **Attack:** parse `tool_calls` or tool-shaped `content` from the endpoint answer and turn
  it into an action or extra capability issuance. The mock always answers with empty
  content, so this branch is invisible.
- **Smallest assertion:** let `MockLlm` return a tool-call-shaped completion; assert the
  outcome is text/data only, no extra audit action or connection occurs, and the session
  has only the one expected E1 token issuance.

### 10. D-09 never puts a nonempty model answer in its audit corpus

- **Where:** `crates/soulcore/tests/draft_commands.rs:848-935`
- **Attack:** copy endpoint answer text into an audit detail. Because every mock answer is
  empty, the chain still passes the current corpus.
- **Smallest assertion:** configure a distinctive nonempty completion, add that exact answer
  to the chain corpus, and retain a positive control showing the checker detects it when
  appended to the candidate.

### 11. F-01's fixture omits required edge files and excludes zero-byte files

- **Where:** `fixtures/fileplan/sample_tree.json:4-43`;
  `crates/soul-fileplan/tests/authorized_scan.rs:49-59`
- **Attack:** skip zero-byte files, mishandle emoji/very-long names, or impose a small-file
  cutoff. The sample has none of the SOTA zero-byte, large, emoji, or very-long cases, and
  explicitly asserts every fixture file is nonempty.
- **Smallest assertion:** add one zero-byte file, one large file, and emoji/long-name entries;
  assert exact scanned path, kind, and byte length for each while preserving the recursive
  digest snapshot.

### 12. F-02's NFD case is not an authorization collision

- **Where:** `crates/soul-fileplan/tests/unauthorized_is_refused.rs:191-205`
- **Attack:** normalize requested path spellings before authorization. The current composed
  directory is already outside root and the decomposed path does not exist, so both are
  refused even by this sloppy implementation.
- **Smallest assertion:** create composed and decomposed sibling directories, authorize only
  the composed one, place a secret in the decomposed sibling, and assert scanning the
  decomposed sibling returns typed refusal with no secret in output.

### 13. F-04 does not pin injection filenames to ordinary plan semantics

- **Where:** `crates/soul-fileplan/tests/filename_injection.rs:90-113`
- **Attack:** emit one invented action per real injection-named file. The test checks only
  uniqueness, source existence, and `len <= files`; it never checks the exact action/target
  dictated by the normal rename/move/group rules.
- **Smallest assertion:** for each created corpus file, assert the exact expected action and
  target (or no entry), and assert the action set remains exactly
  `Group`, `Move`, `Rename`.

### 14. F-04 audit leakage uses a small benign name set, not the injection corpus

- **Where:** `crates/soulcore/tests/fileplan_commands.rs:364-406`;
  `crates/soul-fileplan/tests/filename_injection.rs:58-114`
- **Attack:** audit only suspicious names such as `忽略之前指令.txt` or shell-shaped names.
  The command-level checker sees only five ordinary fixture names; the injection test never
  goes through the command/store audit path.
- **Smallest assertion:** build the portable injection tree in the command test, run
  scan+plan, and add every created corpus name to the chain `LeakageChecker`.

### 15. F-05 checks a fresh issuer, not the session that ran scan+plan

- **Where:** `crates/soulcore/tests/fileplan_commands.rs:323-356`
- **Attack:** `scan_directory` or `plan_files` can issue a token on `session`; the test then
  creates an unrelated `TokenIssuer::new()` and proves only that the new object has zero
  issuances.
- **Smallest assertion:** expose a read-only/test accessor for
  `PolicySession`'s issuance count and assert it remains zero immediately after the actual
  scan+preview workflow.

### 16. `open_store_for_session`'s fail-closed distinction is untested

- **Where:** `crates/soulcore/src/commands/store.rs:118-136`;
  `crates/soulcore/tests/shell_commands.rs:240-268`
- **Attack:** broaden the current `Err(KeyError::Unsupported(_))` arm back to `Err(_)`.
  Existing tests only exercise today's `DpapiKeyProvider`, which always returns
  `Unsupported`, so `Unavailable`, `Io`, and `Malformed` would silently create a plaintext
  fallback seed without a failure.
- **Smallest assertion:** extract a provider-injected private helper and unit-test each
  non-`Unsupported` variant returns `StoreError` and leaves both `soul-test-keys.bin` and
  `soul.db` absent; keep one `Unsupported` control that does fall back.

## Should-fix gaps

### 17. D-05's zero-connection decoy is unreachable from the code under test

- **Where:** `crates/soulcore/tests/draft_commands.rs:493-552`
- **Attack:** probe a default/local-discovery URL on the template route. The mock uses a
  random port whose URL is never passed into any reachable configuration or discovery seam,
  so its zero count is vacuous.
- **Smallest assertion:** inject the decoy URL through the non-E1 discovery input the draft
  path could otherwise probe, while leaving `e1_endpoint` absent, then assert zero requests.
  If no such input exists by design, add a source/dependency assertion that the template
  branch cannot call an egress function.

### 18. D-06 tests the service path only for E1, not templates

- **Where:** `crates/soulcore/tests/draft_commands.rs:555-615`
- **Attack:** read the stored voice on E1 drafts but always use
  `VoiceProfile::default()` on the template branch. Pure template unit tests and the E1
  command test both pass.
- **Smallest assertion:** repeat set-voice/suggest-voice through a `closed_session`; assert
  the very next and subsequent template drafts retain the user-set directness and differ
  from the pre-set draft.

### 19. D-07's dangling-evidence negative is not command/store end-to-end

- **Where:** `crates/soul-draft/tests/people_summary.rs:190-230`;
  `crates/soulcore/tests/draft_commands.rs:802-846`
- **Attack:** the command can catch a store resolution error, drop that edge, and return a
  shorter summary. The pure `summarize` test proves its second net, but the command test has
  only fully resolvable data.
- **Smallest assertion:** create a real graph edge, make one cited evidence row
  unresolvable, call `people_summary`, and assert an error rather than an empty/shorter
  claim list.

### 20. D-09 omits explicit chain verification

- **Where:** `crates/soulcore/tests/draft_commands.rs:896-916`
- **Attack:** return cleanly serialized entries with broken `prev_hash`/`entry_hash`
  linkage. The leakage assertion still passes.
- **Smallest assertion:** call `store.verify_audit_chain().expect(...)` before extracting
  audit values.

### 21. D-09's 4-gram workaround trades flake risk for false negatives

- **Where:** `crates/soulcore/tests/draft_commands.rs:169-249,909-916`
- **Attack:** leak any 4-7 scalar ASCII fragment, or a four-character Chinese fragment from
  a body that also contains one ASCII digit. `shares_an_alphabet_with_the_chain` sends the
  whole body to the default 8-gram checker, so the leak passes.
- **Flake mechanism:** naively moving `third_party_line()` to a 4-gram checker is unsafe
  because it embeds phone `13800138000`; its 4-gram `3800` can occur in random UUID/64-hex
  audit values. One 64-hex digest has roughly a 0.093% chance of containing a chosen
  4-hex run, so a multi-entry chain makes this a realistic intermittent failure.
- **Smallest assertion:** make the check field-aware. Validate and exclude only designated
  structural UUID/hash fields from n-gram matching, then apply `min_ngram(4)` to every
  prose-capable value regardless of alphabet. Add controls for a four-character leak from
  mixed Chinese/ASCII prose and for a hash containing `3800`.

### 22. F-04's filename-URL decoy is explicitly disconnected

- **Where:** `crates/soul-fileplan/tests/filename_injection.rs:116-146`
- **Attack:** fetch a URL derived from scanned/rendered path data. The test itself notes the
  corpus URLs “would not go” to its random decoy; moreover literal `://` URLs are filtered
  out as non-portable and are never real scanned basenames.
- **Smallest assertion:** construct a platform-valid scanned path fixture that the
  production renderer/parser recognizes as the decoy target, then assert the same decoy
  gets zero requests. If such a filename is impossible on supported filesystems, replace
  this vacuous network assertion with an explicit source/dependency lock and retain
  `NetGuard` URL tests at the policy boundary.

## Count

- **Must-fix:** 16
- **Should-fix:** 6
