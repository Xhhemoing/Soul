# gpt-sol-b — BUILD/AUDIT R2 egress probe

Scope: `cursor/goal1-build-audit-c441` at `982021e`, whose product ancestor is the requested `cursor/soul-goal1-7b1c` at `5309656`. Read-only probe; no product files changed.

## p0

None found.

## p1

### P1-1 — changing the configured origin does not invalidate a prepared E1 plan

- Contract: `docs/SECURITY.md:13` says an E1 configuration change invalidates the plan hash.
- Evidence:
  - `crates/soulcore/src/commands/draft.rs:112-118` holds a pending body and plan hash, but no origin or configuration generation.
  - `crates/soulcore/src/commands/policy.rs:236-243` hashes action/model/redaction counts only; the target origin is absent.
  - `crates/soulcore/src/commands/session.rs:847-863` changes the guard/config without discarding the pending draft.
  - `crates/soulcore/src/commands/policy.rs:209-217` sends through whichever endpoint is current at execution time.
- Consequence: prepare against endpoint A, save endpoint B, then replay the still-valid approval; the approved body is sent to B. This fails the explicit config-change invalidation rule even though `NetGuard` correctly authorizes B as the current exact origin.
- How a test goes red: in `crates/soulcore/tests/session_e1.rs`, start mock A and B, configure A, prepare, configure B, then call `generate_draft(plan.approval())`. Assert `PLAN_HASH_MISMATCH` (or equivalent stale-plan refusal) and `A.request_count() == B.request_count() == 0`. Today that assertion fails because B receives one request. Repeat across `apps/desktop/src-tauri/tests/ipc_roundtrip.rs` to pin the product boundary.

### P1-2 — automated “no egress” observes `soul-headless`, not the installed product

- Contract: `docs/FORMAL_WORK_PROMPT.md:98` requires the default main flow and network observation in CI.
- Evidence:
  - `scripts/install-smoke.ps1:42-47` explicitly admits the package does not contain the watched headless binary.
  - `scripts/install-smoke.ps1:296-341` watches only the supplied headless PID.
  - `scripts/install-smoke.ps1:423-467` locates/inspects `soul.exe` but never starts it; phase 3 starts `soul-headless.exe`.
  - `.github/workflows/ci.yml:295-322` builds both binaries, then passes `soul.exe` only for manifest inspection while the smoke executes headless.
  - `apps/desktop/src-tauri/src/lib.rs:94-145` contains product-only startup paths (single instance, WebView, tray) that the headless flow cannot execute.
- Consequence: startup traffic from the Tauri/WebView/tray process—or a future shell-only socket—can leave all current AC-21 automation green. Static CSP/dependency tests reduce risk but do not observe the shipped process.
- How a test goes red: a Windows product smoke should launch built/installed `soul.exe`, watch it and its WebView descendants from before startup through the default UI flow, and fail on any non-loopback peer. A mutation that opens one shell-startup connection would currently leave the headless smoke green; the product smoke must turn red.

### P1-3 — the E0 URL allowlist accepts attacker hosts by textual prefix

- Evidence:
  - `crates/xtask/src/egress.rs:67-73` allowlists raw string prefixes such as `http://localhost` and `http://127.0.0.1`.
  - `crates/xtask/src/egress.rs:475-485` applies `starts_with`, without parsing or an authority boundary.
  - This disagrees with `crates/soul-policy/tests/net_guard.rs:92-115`, which correctly proves `evil.localhost` and `localhost.example.invalid` are not loopback.
- Consequence: shipped source containing `http://localhost.attacker.invalid/x` or `http://127.0.0.1.attacker.invalid/x` passes `e0-audit` as “loopback,” despite naming an external DNS origin.
- How a test goes red: add an `xtask` self-test that scans a synthetic source containing those two URLs and expects two hits. It currently gets zero. Parse the authority or require a boundary of `/`, `:`, `?`, `#`, or end-of-string after the exact allowed host.

## p2

### P2-1 — “E0 absent” is enforced by finite client names, not by network capability

- Evidence:
  - `crates/xtask/src/egress.rs:35-48`, `deny.toml:89-105`, and `apps/desktop/src-tauri/tests/no_egress_path.rs:21-33` enumerate selected HTTP crates/plugins.
  - `crates/xtask/src/egress.rs:496-529` deliberately ignores dynamically assembled hosts.
  - None of these catches `std::net::TcpStream`, `tokio::net`, a command-spawned client, or an unlisted HTTP crate. The desktop test copies the list rather than consuming one canonical policy.
- Current-tree note: the source search found no raw network call in shipped `src` code outside the intended `soul-egress`; this is a regression-detector false-negative, not evidence of present E0 traffic.
- How a test goes red: a negative fixture using `TcpStream::connect(("203.0.113.10", 443))` or an unlisted client with a dynamically assembled target should be expected to fail the E0 audit. Today the audit remains clean, so that expectation is red.

### P2-2 — IPv6 origins parse but serialize into invalid request URLs

- Evidence:
  - `crates/soul-policy/src/net_guard.rs:142-152` strips `[` and `]` from an IPv6 literal.
  - `crates/soul-policy/src/net_guard.rs:127-138` never restores brackets in `Display`.
  - `crates/soul-policy/src/e1.rs:74-82` builds the actual request URL from that display value.
  - Existing `net_guard.rs:92-103` tests only `is_loopback()` for `[::1]`, not serialization or a request.
- Consequence: a valid local endpoint such as `http://[::1]:11434` becomes `http://::1:11434/v1/chat/completions` and fails closed at transport/URL parsing.
- How a test goes red: assert `Origin::parse("http://[::1]:11434").unwrap().to_string() == "http://[::1]:11434"` and add an IPv6 mock wire test. The first assertion is red now.

### P2-3 — runtime observation is sampled and the Windows observer is TCP/PID-only

- Evidence:
  - `crates/soulcore/src/netwatch.rs:41-43,308-355` samples owned sockets every 5 ms; a socket opened and closed between samples can disappear before its inode is correlated.
  - `scripts/install-smoke.ps1:49-52` documents that Windows UDP is invisible.
  - `scripts/install-smoke.ps1:315` filters one PID, not the WebView process tree.
- How a test goes red: calibrate the observer with a short-lived outbound socket and a child-process socket, requiring both to be reported. The present sampling/PID-only design can miss them. Prefer OS event tracing/firewall logging for acceptance evidence rather than treating snapshots as complete interception.

## assumptions

- The two concurrent edits under `apps/desktop/src/routes/Files*` belong to another R2 worker and were excluded from this probe.
- AC-28…AC-34 are not on this trunk. They remain **open-on-other-line** on open [PR #7](https://github.com/Xhhemoing/Soul/pull/7), head `6133307`, with no reported checks at probe time. They were not reimplemented or used to redefine this trunk.
- No Win11 GUI/product network capture was available in this Linux probe.
- Targeted baseline suites are green: `xtask/self_test` 31, `soul-policy/net_guard` 10, `soul-egress/e1_origin` 8, and `soulcore/session_e1` 22.

## next

1. Invalidate or origin-bind every pending E1 plan when endpoint configuration changes; add Session and IPC regressions first.
2. Replace URL-prefix allowlisting with parsed exact-host checks and add hostile-suffix self-tests.
3. Make AC-21 observe `soul.exe` plus descendants on Windows; keep the headless run as core coverage, not product proof.
4. Harden the structural E0 policy against raw/unlisted networking, then fix IPv6 origin rendering and observer calibration.
5. Keep AC-28…AC-34 recorded as open-on-other-line; no T4D reimplementation on this trunk.
