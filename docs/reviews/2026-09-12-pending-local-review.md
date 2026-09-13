# Pending local review (2026-09-13) - M0-3

## Soul - tip for G-W evidence

| Field | Value |
|-------|-------|
| Branch | `cursor/m0-3-win11-gw` |
| Base before this tip | `8aae8f3` |
| Claim Goal1 / G-W closed? | **No** until full `gate-win.ps1` green + Reviewer PASS on this tip SHA |

## Included in this tip

1. `fakeCore.ts`: illegal evidence id with `r` → hex UUID7-shaped id
2. `scripts/gate-win.ps1`: all cargo `--locked`; prereq classes MSVC/SDK | Perl/OpenSSL | Cargo lock | Rust
3. `apps/desktop/src-tauri/Cargo.lock`: regenerated (`cargo +stable generate-lockfile` / MSRV 1.83-compatible); `cargo +1.83 metadata --locked` OK
4. `session.rs`: DPAPI notice includes `永久打不开` / `没有恢复入口`
5. `store_commands.rs`: newer-schema plant uses `DpapiKeyProvider` on Windows (matches `Session`)

## Stale — do not use

- Bind evidence to `633469e` (M0-2). Base is `8aae8f3`; gate on **this tip SHA**.
- Historical Perl/LMS blockers after `LMS_OK` unless re-hit.

## Gate

Ops runs full `gate-win.ps1` on the frozen tip SHA only (not dirty). Green paste → Reviewer. No merge / no G-W claim from Implementer.
