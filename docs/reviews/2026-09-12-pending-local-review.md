# Pending local review (2026-09-13) - M0-3

## Soul - tip for G-W evidence

| Field | Value |
|-------|-------|
| Branch | `cursor/m0-3-win11-gw` |
| Base | `8aae8f3` |
| Claim Goal1 / G-W closed? | **No** until full `gate-win.ps1` green + Reviewer PASS on **this tip SHA** |

## Included

1. `fakeCore.ts`: hex UUID7-shaped evidence ids
2. `scripts/gate-win.ps1`: all cargo `--locked` + MSVC/Perl-OpenSSL/Cargo.lock probes
3. `apps/desktop/src-tauri/Cargo.lock`: MSRV-aware regen; **pin** `dlopen2` 0.8.0 + `dlopen2_derive` 0.4.1 (0.8.1+/0.4.2+ use edition2024 unreadable by Cargo 1.83)
4. `session.rs`: DPAPI notice includes `永久打不开` / `没有恢复入口`
5. `store_commands.rs`: newer-schema plant uses `DpapiKeyProvider` on Windows
6. `xtask` CRLF schema test: normalize before LF→CRLF (avoid `\r\r\n` on Windows checkouts)

## Gate

Ops solo `gate-win.ps1` on the frozen tip SHA only. Green paste → Reviewer. Merge PR #60 only after room PASS. No G-W / Goal1 claim from Implementer alone.
