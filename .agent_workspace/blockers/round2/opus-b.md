# Round 2 / opus-b — cross-validation deltas (M3, S1, S3, S4, S5)

MODEL_SLUG: claude-opus-5-thinking-high-fast
Read at `origin/cursor/soul-goal1-7b1c` (`3e88b48`). Deltas vs `docs/BLOCKERS.md` only. No crate patched. No doc typo found to patch — the two symbol paths I suspected (`soul_algo_trait::a2_render`, `::apply_intake`) are both correctly re-exported at that crate's root, and `test-windows`, `DEMOTE_ONE_BAND_DAYS`, `.agent_workspace/round2/fable-b/GRAPH_CORRECTION.md` all check out.

| Attack | Verdict |
|---|---|
| Probe fixture weakens AC-18 (Bravo 100 % refuse)? | **No** — but M3 as written cannot land: it trips a corpus size guard, and the tempting repair is the weakening. |
| `cfg!(windows)` expected lists safer? | **No, strictly worse** — it turns a real AC-18 hole into a compile-time exemption. |
| DPAPI in `soul-store` vs a tiny crate? | **Separate crate.** My own Round 1 said in-crate and was wrong: `forbid` makes it impossible without a downgrade. |
| Wrong ordering? | **Yes, two.** §5's "each step green first" is unsatisfiable at steps 2–8; S4 sits 9 steps too late. |

## M3 — the probe is the right instrument, the fix as written is incomplete

**D1. `unauthorized_paths.rs:128`'s `assert!(corpus.len() >= 27)` is exactly saturated, so the M3 fix is red on the job it was meant to green.** Counted on the branch: 27 base tuples plus 3 `cfg(unix)` = 30 on Linux, 27 on Windows. Gating the two `lower_alpha` entries behind a filesystem probe leaves 25 on NTFS and the guard fires. §M3 never mentions the guard, and its step 3 ("不要缩短语料（有长度断言）") reads as forbidding the very edit step 2 requires. **This is the actual AC-18 weakening vector**: the obvious way out is to relax the bound to `>= 25` unconditionally, which then lets Linux silently drop five entries forever. The guard has to become exact and probe-aware — `25 + 2*probe + 3*cfg!(unix)` — and §M3 should say so in step 2 rather than leaving it as a trap in step 3.

**D2. Bravo is untouched, verified rather than assumed.** Corpus entries 1–3 are unconditional, and the probe gates only entries 5–6, which name `alpha`, never `Bravo`. `case_is_decided_by_the_filesystem_rather_than_assumed:252-269` loops `Bravo`, its upper- and lower-cased spellings and `SECRET.TXT` under **both** `Exact` and `CaseFolded` on every host, and no part of the M3 fix touches it. That loop is where the 100 % claim actually lives.

**D3. The tripwire test survives the decoy not being written — this needed checking and BLOCKERS.md does not state it.** `authorize.rs:280`'s `canonicalize_deepest` returns `(path, exists=false)` for a missing path and `resolve` still returns `Ok`; that is how `Alpha/文档/report.pdf` passes at `:201`. So on a case-folding host with no decoy on disk, line 246's `folded.resolve(alpha/decoy.txt).is_ok()` still holds, and line 236's `exact(...).is_err()` holds lexically. No edit is needed there. Record it as a stated invariant of step 2, or someone will gate that test behind the probe too and delete the tripwire while "being consistent".

**D4. A residual M3 does not cover, and should predict.** On a Windows directory flagged case-sensitive (`fsutil file setCaseSensitiveInfo`, which is what WSL sets), the probe correctly reports `true`, so entries 5–6 stay in the corpus — but `Authorization::new()` takes `PathMatching::for_this_platform()`, which is itself `cfg!(windows)` and returns `CaseFolded`. It then folds `alpha` into the authorized root and **accepts a directory nobody authorized**. That is a genuine P2 product hole in the default matching rule, and the probe is what exposes it. Say in §M3 that such a host is expected red for a product reason, otherwise the next agent reverts to `cfg` to make it green.

**D5. `cfg!(windows)`, answered directly.** It is wrong in both directions: APFS folds case with `cfg!(windows) == false`, and case-sensitive NTFS directories do not fold with it `true`. The decisive argument is not the fixture though, it is `unauthorized_paths`: cfg-gating entries 5–6 would **remove a genuinely unauthorized directory from the must-refuse corpus** on exactly the host in D4. The probe fails safe, because a lying probe is caught by the paired positive assertion; `cfg` fails open and silently. Separately, cfg-conditional expected lists in `authorized_scan.rs` would encode fixture pollution as asserted product behaviour, and would not fix `unauthorized_paths` at all — you would need the dangerous cfg gate there anyway.

**D6. §M3 step 1 undercounts the job.** `test-windows` has two `cargo test` invocations, `ci.yml:131` (workspace) and `:152` (desktop shell). Write "every `cargo test` in the windows job", not "`cargo test`".

## S1 — DPAPI belongs in a new crate, not in `soul-store`

**D7. Correcting my own Round 1 §3.4.** `soul-store/src/lib.rs:25` is `#![forbid(unsafe_code)]`, and an inner `#![allow(unsafe_code)]` is a hard error, E0453 — verified with `rustc` rather than recalled. So implementing DPAPI inside `soul-store` *requires* downgrading the crate root to `cfg_attr(windows, deny(unsafe_code))`, which spends an unconditional `forbid` on the one crate holding the SQLCipher handle, the KEK and every piece of plaintext prose. `soul-collect` is not precedent for this: it never had a `forbid` to spend, its root already says `deny` on Windows.

**D8. Recommend `crates/soul-winkeys`:** zero dependencies, roughly 120 lines, entirely `#[cfg(windows)]`, pulled in through `target."cfg(windows)".dependencies` so non-Windows builds never see it. `soul-store` then keeps `forbid` on every platform, which is a claim `SECURITY.md` can make without qualification, and the unsafe surface becomes a whole crate to audit instead of a grep for stray `allow`s. The cost is one more workspace member in the same `Cargo.toml` that M2 already has to resolve. The asymmetry that settles it: `deny` can be locally overridden by any future edit anywhere in `soul-store`, `forbid` cannot.

**D9.** §S1 should name the target crate explicitly. By placement it currently reads as "implement it in `soul-store`", which is the most expensive of the options.

## S3 / S4 / S5

**D10. S3's fix is aimed one layer too high.** The false pass is minted in Rust, not in PowerShell: `headless.rs:771` is `require("egress", findings.non_loopback_connections == 0, ...)`, and `WatchReport::is_clean` (`netwatch.rs:266`) is just `non_loopback.is_empty()`. Both are true when the watcher saw nothing. The `EgressFindings` struct **already** carries `observed`, `observation_note` and `samples`, so the honest data is emitted and simply not gated — the fix is about three lines: a second `require` that `observed || explicit waiver`, and `is_clean` additionally requiring `Support::Observed` with `samples > 0`. Meanwhile `install-smoke.ps1` already samples `Get-NetTCPConnection` (`:303`, `:310`) and already reports its own absence (`:486`), so S3's "改信自己的采样" describes code that exists.

**D11. S4 must name both jobs.** The `package` job (`ci.yml:197`, its own `rust-cache` key) also builds `soul-store` and therefore vendored OpenSSL, and it has **no** perl/NASM step at all. S4 as written only implies `test-windows`.

**D12. S5, a low-risk detail worth recording so it is not re-litigated:** `tauri build --no-bundle` replaces the `release binaries` cargo line rather than adding to it (the CLI supplies `custom-protocol` for release builds itself), and makes the separate `frontend bundle` step redundant via `beforeBuildCommand`. The output path `apps/desktop/src-tauri/target/release/soul.exe` is unchanged, so the manifest-reading and smoke steps keep working untouched.

## Ordering

**D13. §5's gate is unsatisfiable from step 2 through step 8.** `session_commands.rs:55` asserts `status.store_opened` unconditionally and the file calls `Session::open` 19 times; DPAPI returns `Unsupported`, so the Windows job stays red from step 2 until DPAPI lands at step 9. Read literally, "每一步 CI 绿再做下一步" means steps 3–8 never start. Two ways out, and one of them has to be written down: (a) a checked-in, named waiver list of the DPAPI-caused Windows failures, with the gate meaning "no failure outside the list" and a test asserting the list only ever shrinks; or (b) insert a roughly ten-line `Session::open_with_keys(dir, provider)` test entry point at step 2.5 so `soulcore` can run on Windows under `TestKeyProvider` while the shipped `Session::open` stays honest. (b) is cheaper and makes step 1's red table actionable; (a) is more honest about AC-04 and AC-08 staying unproven. Do **not** solve it by promoting DPAPI to step 3 — that delays G1, which is the actual product.

**D14. S4 should move from step 10 to step 1**, batched with `--no-fail-fast`. Both are `ci.yml`-only with zero crate risk, and the latent perl failure is invisible today only because `rust-cache` restored OpenSSL. It will surface partway through steps 3–8, as an obscure `openssl-src` error, during the longest and most conflict-prone work. The same argument moves S3 and S5 early: they are script-only and independent of every crate change.

**D15.** DPAPI after `--no-fail-fast` is correct and should stay that way: until step 1 lands the Windows job never reaches `soulcore` at all, so the DPAPI blast radius is currently unmeasured rather than known.

## Out of slot, load-bearing

**D16.** §G1 step 5's "A2 的「半年」阈值必须是 `DEMOTE_ONE_BAND_DAYS` 的别名，禁止第二个 `180` 字面量" is not implementable as written. `soul-algo-trait` has zero dependencies by design and already defines its own `a2::DORMANT_AFTER_DAYS = 180`, while `DEMOTE_ONE_BAND_DAYS` lives in `soul-algo-tie`; aliasing would make one frozen crate depend on the other. Reword to "no third `180` in product code", or accept the two frozen constants as independent.
