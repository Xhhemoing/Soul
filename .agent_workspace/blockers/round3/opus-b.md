# Round 3 / opus-b — freeze confirmation of the M3 / S1 technical recipes

MODEL_SLUG: claude-opus-5-thinking-high-fast

Not FREEZE_OK. Two findings, one in each recipe. Read at `origin/cursor/soul-goal1-7b1c` = `bbe5637`; `docs/BLOCKERS.md` §1 still pins `3e88b48`, seven commits back. No product code touched. AC-18 is **not** weakened by anything in M3, and `soul-store` still forbids unsafe.

## 1. M3 contradicts itself about `case_is_decided_by_the_filesystem_rather_than_assumed`

§M3 says the probe "还要改四处（约 `:50` `:53` `:237` `:247`）". Those are the four `tree.lower_alpha()` call sites, but `:237` and `:247` are inside `case_is_decided_by_the_filesystem_rather_than_assumed` (`unauthorized_paths.rs:230–278`) — the test the very next paragraph says "不要改、不要跟探针走". Both cannot hold.

The "don't touch it" side is the correct one: **those two assertions pass unchanged on NTFS**, so the probe has two edit sites, not four. `Authorization::resolve` picks its root lexically against the *requested* spelling under the explicitly stated `PathMatching` (`authorize.rs:257–262` via `same_segment`, `:63–68`), before any canonicalization:

- `:237` (`PathMatching::Exact`): `same_segment("Alpha", "alpha")` is false, so `contains_screened` never matches and the call returns `Err(OutsideAuthorizedRoot)` on every filesystem. The assertion is `is_err()`. Green.
- `:247` (`PathMatching::CaseFolded`): folds on both platforms, and `contains_canonical` folds too, so it is `Ok` on Linux (`base/alpha/decoy.txt` is a real third directory) and `Ok` on Windows (canonicalization returns `base\Alpha\decoy.txt`, inside the root). The assertion is `is_ok()`. Green.

That test states both rules explicitly rather than inheriting the host's, which is exactly why it is filesystem-independent and must stay off the probe. Suggested repair, wording only: drop `:237 :247`, say **two** sites (`:50`, `:53`) plus the size guard at `:127–131`.

Nit, non-blocking, same paragraph: the exact guard `25 + (probe ? 2 : 0) + (cfg!(unix) ? 3 : 0)` only reads correctly if `probe` means "the filesystem is case-sensitive", while the prose one sentence earlier describes the probe as the thing that *removes* the two entries. Worth naming the boolean. The arithmetic itself checks out — base corpus is 27 entries (`:45–103`), so `>= 27` is exactly saturated on Windows as claimed; Linux 30, NTFS 25, case-insensitive APFS 28.

Rest of M3 confirmed. `Tree::build` does write `decoy.txt` into `base/alpha` (`common/mod.rs:104–108`), and `authorized_scan.rs:65` / `:165` are the precise failure points: `.txt` is a moved kind, so `decoy.txt` joins `moves` and `entries` but not `left_alone`, which leaves the third assertion at `:101` green. Bravo's three entries are unconditional (`:46–48`) and the probe drops only the two `lower_alpha` entries — on a case-insensitive filesystem `alpha` **is** the authorized `Alpha`, so it was never an unauthorized path and refusing to assert its refusal concedes nothing. `Session::open_with_keys` still does not exist anywhere on the branch, as §M3 assumes.

## 2. S1 names a crate that should not be created: it already shipped as `soul-win-dpapi`

§S1 and §5 row 7 say to put DPAPI in a new crate **`soul-winkeys`**. There is no such crate and creating one would duplicate working code: `40b3474` and `2f323d5` — both after the pinned `3e88b48` — landed it as **`soul-win-dpapi`**, wired into `DpapiKeyProvider` in `soul-store/src/keys.rs` with `crates/soul-store/tests/dpapi_key_chain.rs` alongside.

The recipe's substance is honored, so this is a rename plus a status correction, not a rework:

- `soul-store/src/lib.rs:25` is still `#![forbid(unsafe_code)]`, undowngraded. The one `unsafe` block lives in `soul-win-dpapi/src/sys.rs`; that crate carries `forbid(unsafe_code)` off Windows and `deny` on Windows, which is the isolated crate S1 asked for and not the thing S1 prohibited.
- Failure maps to `KeyError::Unavailable` (`keys.rs:39–41`), and a blob this build cannot read is reported as `Corrupt`, explicitly not overwritten (`:57–61`), so no fresh mint orphans a user's database.

Two deviations from the recipe's letter, both deliberate and both better than what S1 wrote. The crate is **not** `cfg(windows)`-only: it compiles everywhere and refuses through `SUPPORTED = cfg!(windows)`, precisely so `soul-store` stays free of `#[cfg]` and the provider is type-checked on Linux CI. And it is not literally 零依赖 — it takes `thiserror` and `zeroize`, with zero Win32 binding crates, which is what the constraint was for.

Consequence for the ordering table: §5 row 7 and S1's "P0 发货 / P1 合入" framing are counterfactual at the current tip. Row 7 should read `soul-win-dpapi`, marked landed, and the M3 note that "Windows 测试在 DPAPI 落地前会在 `Session::open` 处继续红" now needs rechecking against a tip where DPAPI has in fact landed — `open_with_keys` may no longer be the cheapest way to get `soulcore` green on Windows.
