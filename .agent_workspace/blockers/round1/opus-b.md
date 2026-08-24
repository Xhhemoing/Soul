# Round 1 / slot opus-b — Windows, key protection, fileplan blockers

MODEL_SLUG: claude-opus-5-thinking-high-fast

Branch under review: `origin/cursor/soul-goal1-7b1c` @ `3e88b48c851cd9b69ac99f45328b206910084a39`
CI evidence: runs [32741604360](https://github.com/Xhhemoing/Soul/actions/runs/32741604360) (`f6c39756`) and
[32748412486](https://github.com/Xhhemoing/Soul/actions/runs/32748412486) (`3e88b48c`); job `test (windows-latest)`
= 97477123290 / 97499320614. Earlier run [32737451542](https://github.com/Xhhemoing/Soul/actions/runs/32737451542)
(`fbad85b4`, job 97463681611) is used below to separate two different Windows bugs.

Analysis only. No source in `crates/soul-fileplan` or `crates/soul-store` was modified by this slot.

---

## 0. Headline

Four findings, in the order they cost the project:

1. **`screen.rs`'s `\\?\C:\` handling is correct and is not the cause of the two remaining
   failures.** It was written for a real Windows bug and it fixed that bug: at `fbad85b4` all
   **9** tests in `authorized_scan.rs` died at line 22 (`.expect("authorize Alpha")`); after
   `9ba160d7` ("fileplan: Windows canonicalize is a local drive, not a UNC share") **7** pass.
   The remaining 2 are a **test-fixture bug**: `Tree::build` writes `alpha/decoy.txt` as a
   sibling of `Alpha`, and on NTFS those are one directory, so the decoy lands *inside the
   authorized root*.
2. **The Windows job has never run past `soul-fileplan` since WP11 landed.** `cargo test
   --workspace --all-targets` has no `--no-fail-fast`, so cargo aborts at the first failing
   test binary. Ten crates — including `soul-store-api/tests/sqlcipher_smoke.rs`, which
   `SECURITY.md` cites as running on both CI jobs — have not executed on windows-latest on
   this branch. Several of them will fail when they finally do.
3. **DPAPI is a policy gap, not a technical one.** `soul-collect/src/windows.rs` already
   declares five Win32 entry points by hand under `cfg_attr(not(windows), forbid(unsafe_code))`
   with **no binding crate**. DPAPI needs strictly less: two `crypt32` functions, one struct,
   one `kernel32` call. `SECURITY.md` line 44's reason for the stub is therefore misleading as
   written. DPAPI blocks **AC-04 and AC-08** on a real Win11 box; **AC-01 is not blocked by it**.
4. **perl/NASM and `tauri build` are both "green because nothing checks".** The prerequisite
   step cannot fail and measures the wrong perl; `tauri build` has never run on any runner, and
   the part of it that would fetch NSIS is only the last step.

| # | Item | Sev | One-line reason |
|---|---|---|---|
| B1 | `authorized_scan.rs` 65/165 — case-folded fixture collision | **P0** | Windows CI is red; the fix is 6 lines and the wrong fix silently weakens AC-18 |
| B2 | `cargo test` fail-fast hides 10 crates on Windows | **P0** | `sqlcipher_smoke` and all of `soulcore` have not run; `SECURITY.md` §加密落地 is currently false |
| B3 | `DpapiKeyProvider` stub | **P0** | AC-04 and AC-08 are unreachable on the only OS Soul ships on; also makes ~19 `Session::open` tests unrunnable on Windows |
| B4 | perl/NASM prerequisite step is a report, not a gate, and reads Cygwin perl | **P1** | A runner-image change takes out Windows CI *and* release artifacts with an obscure `openssl-src` error |
| B5 | `tauri build` never runs; `tauri.conf.json` validated only as text | **P1** | AC-01/AC-26 packaging column rests entirely on the author's laptop; `--no-bundle` closes most of it for free |
| B6 | WebView2 `skip` + Evergreen assumption | **P2** | Correct choice, undocumented supported-platform boundary |
| B7 | Corpus entries 12/13 are degenerate on Windows | **P2** | `tree.bravo()` is already `\\?\`-prefixed, so "a verbatim prefix" tests a double prefix |
| B8 | `\\?\C:/x` screens as `C:\x` | **P2** | Verbatim paths do not translate `/`; not an escape, but the two spellings collide |

---

## 1. `crates/soul-fileplan` — the two remaining Windows failures

### 1.1 What the assertions actually say

`crates/soul-fileplan/tests/authorized_scan.rs:65` is the `moves` list in
`an_authorized_directory_previews_a_plan_and_the_disk_does_not_move`; `:165` is the `relatives`
list in `the_scan_reads_directory_entries_and_stops_there`. Both are exact `assert_eq!` over an
ordered `Vec`. From job 97477123290:

```
thread 'an_authorized_directory_previews_a_plan_and_the_disk_does_not_move'
panicked at crates\soul-fileplan\tests\authorized_scan.rs:65:5:
  left: [("budget.csv", …), ("decoy.txt", "文档/decoy.txt"), ("photo.jpg", …), ("report.pdf", …), ("笔记.txt", …)]
 right: [("budget.csv", …),                                  ("photo.jpg", …), ("report.pdf", …), ("笔记.txt", …)]

thread 'the_scan_reads_directory_entries_and_stops_there'
panicked at crates\soul-fileplan\tests\authorized_scan.rs:165:5:
  left: ["Makefile", "budget.csv", "decoy.txt", "mystery.qqq", …]
 right: ["Makefile", "budget.csv",              "mystery.qqq", …]
```

One symbol, in both: `decoy.txt`.

### 1.2 Root cause

`crates/soul-fileplan/tests/common/mod.rs`, `Tree::build`:

```rust
write(&base.join("Alpha"), "photo.jpg", …);          // the authorized root
…
write(&base.join("alpha"), "decoy.txt", "同名不同大小写的第三个目录");
```

`write()` calls `std::fs::create_dir_all(directory)` first. On NTFS, `CreateDirectoryW("alpha")`
fails with `ERROR_ALREADY_EXISTS`, `create_dir_all` sees `path.is_dir()` and returns `Ok(())`,
and `decoy.txt` is written into the existing **`Alpha`**. The fixture's own module doc says the
quiet part out loud — "on Windows it is `Alpha` under another spelling" — and then writes into it
anyway. `.txt` maps to `FileKind::Doc`, so the scan reports a 13th entry and the plan a 5th move.

`screen.rs` is not involved. `as_local_drive_spelling` behaves correctly on every case in the
corpus: `\\?\C:\…` → stripped to `C:\…`; `\\?\UNC\…`, `\\?\GLOBALROOT\…`, `\\?\HarddiskVolume1\…`,
`\\?\Volume{…}` all fall through to the two-separator rule because the second character after the
prefix is not `:`. All 11 `screen::tests` pass on windows-latest, and the 7 sibling tests in
`authorized_scan.rs` that do not assert exact lists pass too.

### 1.3 The failure this hides (and why fixing only 65/165 makes things worse)

`cargo test` aborts the whole run at the first failing binary. `unauthorized_paths.rs` has
**never executed on windows-latest**. It will fail as soon as it does, and it will look like an
AC-18 violation:

`refusable()` puts two entries in the must-be-refused corpus that are, on NTFS, *inside the
authorized root*:

```rust
("a sibling that was never named", tree.lower_alpha()),
("a file in that sibling", format!("{}/decoy.txt", tree.lower_alpha())),
```

`authorized()` builds `Authorization::new()`, whose `PathMatching::default()` is
`for_this_platform()` = `CaseFolded` on Windows. Tracing `resolve(<base>\alpha)`:
`contains_screened` folds `alpha`↔`Alpha` → root found, `below` empty → no symlink walk →
`canonicalize_deepest` returns `\\?\C:\…\Alpha` → `contains_canonical` matches → **`Ok`**. The
test then panics with *"an unauthorized path was accepted"*, and
`the_preview_refuses_the_same_corpus_and_leaves_the_disk_alone` panics with
*"`…` produced a preview"*.

That is not a hole. On Windows `<base>\alpha` **is** the authorized root, and accepting it is the
correct answer — `case_is_decided_by_the_filesystem_rather_than_assumed` already asserts exactly
this under `PathMatching::CaseFolded`. The corpus is wrong, not the resolver. But the failure text
will read like an AC-18 breach, and the tempting "fix" (drop the two entries, or relax the
assertion) trips the `corpus.len() >= 27` guard or removes it — and *that* would weaken AC-18.

### 1.4 Proposed patch (does not weaken AC-18)

Three files. The governing idea: **ask the filesystem, do not ask `cfg!(windows)`.** A Windows
directory can be flagged case-sensitive (`fsutil file setCaseSensitiveInfo`, standard with WSL),
and APFS is case-insensitive by default — so a macOS developer hits this bug today with
`cfg!(windows) == false`.

**(a) `crates/soul-fileplan/tests/common/mod.rs` — `Tree` / `Tree::build`**

Add a field and a probe; write the decoy only where it is a third directory.

```rust
pub struct Tree {
    directory: tempfile::TempDir,
    base: PathBuf,
    /// Whether this filesystem handed back `alpha` and `Alpha` as two
    /// directories. Not `cfg!(windows)`: a Windows directory can be marked
    /// case-sensitive, and APFS folds case by default. What the fixture needs
    /// to know is what this temporary directory actually did.
    case_variant_is_separate: bool,
}

// in build(), after the Alpha writes:
let lower = base.join("alpha");
let case_variant_is_separate = std::fs::create_dir_all(&lower).is_ok()
    && match (std::fs::canonicalize(&lower), std::fs::canonicalize(base.join("Alpha"))) {
        (Ok(lower), Ok(upper)) => lower != upper,
        _ => false,
    };
if case_variant_is_separate {
    // Where case folds, this file would land inside the authorized root and
    // the exact-list assertions in authorized_scan.rs would be describing a
    // directory the fixture polluted rather than the one it built.
    write(&lower, "decoy.txt", "同名不同大小写的第三个目录");
}

pub fn case_variant_is_a_third_directory(&self) -> bool { self.case_variant_is_separate }
```

Probe logic validated on this host (`/tmp/probe/probe.rs`): case-sensitive → `true`; two
spellings of one directory → `false`; decoy stays out of `Alpha`. The Windows half is inferred
from the CI log, which is direct evidence of the collision — I could not mount a case-insensitive
filesystem here (no `CONFIG_UNICODE` for ext4 casefold, no `vfat`, no network for `dosfstools`).

**(b) `crates/soul-fileplan/tests/unauthorized_paths.rs` — `refusable` and its size guard**

Move the two case entries out of the literal `vec!` and behind the probe, and make the guard
**exact** so a silent shrink still fails:

```rust
if tree.case_variant_is_a_third_directory() {
    corpus.push(("a sibling that was never named", tree.lower_alpha()));
    corpus.push(("a file in that sibling", format!("{}/decoy.txt", tree.lower_alpha())));
}
// …
let expected = 25
    + if tree.case_variant_is_a_third_directory() { 2 } else { 0 }
    + if cfg!(unix) { 3 } else { 0 };
assert_eq!(corpus.len(), expected, "the corpus changed size");
```

and add the positive half, so the case-folding reading is asserted rather than merely omitted:

```rust
/// Where the filesystem folds case, `alpha` *is* the root the user authorized,
/// and saying so is the right answer rather than a hole. What must not move is
/// the directory nobody named: `case_is_decided_by_the_filesystem_rather_than_
/// assumed` keeps `Bravo` refused in every case under both rules on every host.
#[test]
fn a_case_variant_of_the_root_follows_the_filesystem_rather_than_the_spelling() {
    let tree = Tree::build();
    let authorization = authorized(&tree);
    assert_eq!(
        authorization.resolve(&tree.lower_alpha()).is_ok(),
        !tree.case_variant_is_a_third_directory(),
    );
}
```

**Why this does not weaken AC-18.** AC-18's B is `Bravo`. Nothing about `Bravo` changes: it stays
in the corpus unconditionally on both platforms, and
`case_is_decided_by_the_filesystem_rather_than_assumed` keeps asserting, under *both*
`PathMatching::Exact` and `PathMatching::CaseFolded` on every host, that `bravo` /
`BRAVO` / `<bravo>/SECRET.TXT` are all refused. Those loops are the 100 % claim and they are
untouched. What is removed on a case-folding host is two entries that were never unauthorized
there — keeping them would not make AC-18 stronger, it would make the suite assert something
false about NTFS.

**(c) `crates/soul-fileplan/tests/authorized_scan.rs` — a fixture guard**

The two list assertions are exact, so the fixture has to be exact too, and that has to be its own
test rather than an implication of the two big ones:

```rust
/// The lists below name every entry. That only means anything if the fixture
/// put exactly those entries there — which is how this file first went red on
/// windows-latest: `alpha/decoy.txt` and `Alpha/decoy.txt` are one file on NTFS.
#[test]
fn the_fixture_leaked_nothing_into_the_authorized_root() {
    let tree = Tree::build();
    let mut names: Vec<String> = std::fs::read_dir(tree.alpha())
        .expect("read Alpha")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n != "escape" && n != "loopback")
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["Makefile", "budget.csv", "mystery.qqq", "photo.jpg",
         "report.pdf", "sub", "taken.png", "图片", "笔记.txt"],
    );
}
```

### 1.5 Tests that must stay red if the patch is wrong

| Test | Must be red when |
|---|---|
| `authorized_scan::the_fixture_leaked_nothing_into_the_authorized_root` *(new)* | the decoy lands in `Alpha` on any host — this is the assertion that fails *first* and names the cause, instead of two list diffs |
| `unauthorized_paths::every_way_of_naming_the_unauthorized_directory_is_refused` with the **exact** size guard | anyone makes Windows green by deleting corpus entries |
| `unauthorized_paths::case_is_decided_by_the_filesystem_rather_than_assumed` (unchanged, unconditional) | either matching rule ever lets `Bravo` in, in any case — the AC-18 tripwire |
| `unauthorized_paths::a_case_variant_of_the_root_follows_the_filesystem_rather_than_the_spelling` *(new)* | the probe lies about the host, in either direction |
| `screen::tests::an_extended_local_drive_path_is_the_same_directory` (unchanged) | someone "fixes" this by reverting `as_local_drive_spelling` — the real Windows bug it solved would come straight back |

---

## 2. `cargo test` fail-fast — the finding underneath everything else

`.github/workflows/ci.yml:131`: `cargo test --workspace --all-targets`. No `--no-fail-fast`.

Test binaries actually executed in job 97477123290, in order:

```
soul_collect (unit) · collection_lifecycle · consent_gate · the_tests_do_not_fake_collection ·
window_titles_are_not_collected · soul_draft (unit) · injection · never_sends · people_summary ·
voice_and_template · wire · soul_egress (unit) · e1_origin · soul_fileplan (unit) ·
authorized_scan  ← aborts here
```

Never reached on windows-latest since WP11 landed at `fbad85b4` (14:05 UTC; last green Windows
run was `01de568e` at 12:40, before `soul-fileplan` existed):

`soul-fileplan/{execution_is_refused, file_names_are_data, no_write_api, unauthorized_paths}`,
`soul-graph`, `soul-import`, `soul-memory`, `soul-policy`, `soul-profile`, `soul-schema`,
`soul-store`, **`soul-store-api` (including `sqlcipher_smoke.rs`)**, `soul-testkit`, `soulcore`,
`xtask`.

Two consequences worth stating plainly:

- `docs/SECURITY.md` §加密落地 says the SQLCipher smoke test runs on "ubuntu 与 windows-latest 两个
  CI job". On this branch it does not. Everything the smoke test underwrites — `PRAGMA
  cipher_version` = 4.5.7 *on the target platform*, wrong-key-fails, no-plaintext-in-the-file — is
  currently Linux-only evidence.
- `soulcore/tests/session_commands.rs:55` asserts `status.store_opened` unconditionally, and
  `:57` asserts `soul.db` exists. Both are false on Windows because DPAPI refuses. That file
  calls `Session::open` 19 times. So even a perfect fileplan patch leaves the Windows job red
  — this is the ordering constraint between B1 and B3.

**Patch:** `.github/workflows/ci.yml`, `test-windows` job → `cargo test --workspace --all-targets
--no-fail-fast`. Cheap, and it converts "one failure" into a complete list. Do this **first**, in
its own commit, so the next Windows run enumerates the real backlog instead of revealing it one
crate at a time.

**Test that must stay red:** extend `crates/xtask/tests/self_test.rs` (the repo already has the
read-the-file-back idiom in `one_store.rs`, `shell_is_local_only.rs`, `install_smoke_script.rs`)
with an assertion that every `runs-on: windows-latest` job's `cargo test` invocation carries
`--no-fail-fast`. Red the moment someone trims it back.

---

## 3. `DpapiKeyProvider` — the honest v0.1 path

### 3.1 What is actually there

`crates/soul-store/src/keys.rs:230-247`: both `#[cfg(windows)]` and `#[cfg(not(windows))]` arms of
`unprotect` return `KeyError::Unsupported`. `crates/soul-store/src/lib.rs:25` is
`#![forbid(unsafe_code)]`. `soulcore/src/commands/session.rs:170` `key_provider` picks
`DpapiKeyProvider` on Windows via `cfg!`, so `Session::open` → `open_store` → `Err` →
`StoreHandle::Unavailable`, and `SessionStatus.store_notice` says so. `apps/desktop/src-tauri/tests/one_store.rs:120`
encodes the gap as an accepted branch: `(None, None) => assert!(!session.status().store_opened)`.

### 3.2 The `forbid(unsafe_code)` reason does not survive contact with the workspace

`crates/soul-collect/src/lib.rs:35-36`:

```rust
#![cfg_attr(not(windows), forbid(unsafe_code))]
#![cfg_attr(windows, deny(unsafe_code))]
```

and `crates/soul-collect/src/windows.rs:24` `#![allow(unsafe_code)]`, containing hand-written
`extern "system"` declarations for `GetForegroundWindow`, `GetWindowThreadProcessId`,
`OpenProcess`, `QueryFullProcessImageNameW`, `CloseHandle` — **five entry points, no binding
crate**, with the rationale spelled out in the file: *"the workspace pins every third-party
version centrally, and four declarations are cheaper to audit than a binding crate."*

DPAPI is smaller: `CryptProtectData` + `CryptUnprotectData` (`crypt32`), one `#[repr(C)]
DATA_BLOB`, `LocalFree` (`kernel32`). So `SECURITY.md:44`'s "Win32 绑定要引入 `unsafe`，本 crate 现在
`#![forbid(unsafe_code)]`" is true as a statement about the current lint attribute and misleading
as a *reason*: the workspace already decided this question, in the opposite direction, one crate
over. That sentence needs rewriting whichever option is chosen.

### 3.3 The three options

**(1) Implement DPAPI — recommended.**
No new dependency (keeps `cargo deny`, the central version pin, and the "SBOM with no HTTP
client" story exactly as they are). ~120 lines in one new `#[cfg(windows)]` module, mirroring a
file that already exists. Testable on windows-latest without a desktop: DPAPI works for the
runner's own account and `CRYPTPROTECT_UI_FORBIDDEN` guarantees no prompt.

**(2) Keep the stub and disclose forever — not viable for v0.1.**
AC-04 and AC-08 are CI gates in the Goal 1 matrix and are unreachable on the target platform (see
3.5). "电子版的你" that cannot read its own database on the only OS it ships on is not a v0.1.
`STATUS.md`'s own 下一步 §3 already says this: *"DPAPI 要真的实现，否则 Windows 上库打不开…这是 Goal 1
在目标平台上能不能读自己数据的前提."*

**(3) P0-style plaintext key file with a UI notice — reject as the shipping path.**
`SECURITY.md` §加密落地 states the purpose of whole-database encryption: *"整库加密挡的是把 `soul.db`
拷走的人."* A plaintext seed file beside `soul.db` travels in the same directory as `soul.db`, so
the copy carries its own key and that sentence stops being true. Disclosure does not repair it; it
converts a loud, honest gap into a quiet, disclosed non-protection, which is worse for a product
whose whole pitch is that the disclosure can be trusted.

Two qualifications on (3), because the proposal as stated is more careful than the blunt version:

- **"only on `Unsupported`" is the right guard, and it must never widen to `Unavailable`.** My
  patch in 3.4 returns `KeyError::Unavailable` for a real DPAPI failure (blob written by another
  account, roaming-profile loss, password reset) and keeps `Unsupported` for "this build has no
  DPAPI at all". A fallback keyed on `Unsupported` therefore fires only where the stub is still
  present, and *cannot* fire on the dangerous case. Widening it to `Unavailable` would silently
  re-key a user whose real blob is temporarily unreadable and orphan their whole database.
- **If P0's actual need is "Windows CI green before DPAPI lands", the fallback is the wrong
  lever.** Add a test-only `Session::open_with_keys(directory, provider)` in `soulcore` so
  `session_commands.rs` can drive a session on Windows with `TestKeyProvider` explicitly. That
  keeps the shipped `Session::open` honest, does not add a second `open_store` call (so
  `one_store.rs` stays satisfied), and matches the existing decision that the shell may not
  reach `open_test_store`.

### 3.4 Patch sketch

**`crates/soul-store/src/lib.rs`** (lints + module):

```rust
#![cfg_attr(not(windows), forbid(unsafe_code))]
#![cfg_attr(windows, deny(unsafe_code))]
…
#[cfg(windows)]
mod dpapi;
```

**`crates/soul-store/src/dpapi.rs`** (new, the only `unsafe` in the crate, same shape as
`soul-collect/src/windows.rs`):

```rust
#![allow(unsafe_code)]

/// User scope. `CRYPTPROTECT_LOCAL_MACHINE` is deliberately absent: it would
/// let every account on the box unwrap the seed, which is the opposite of
/// what DPAPI is here for. UI_FORBIDDEN because nothing may prompt.
const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

pub fn protect(plain: &[u8], entropy: &[u8]) -> Result<Vec<u8>, u32>;
pub fn unprotect(blob: &[u8], entropy: &[u8]) -> Result<Vec<u8>, u32>;

#[allow(non_snake_case)]
mod ffi {
    use std::ffi::c_void;
    #[repr(C)]
    pub struct DataBlob { pub cbData: u32, pub pbData: *mut u8 }
    #[link(name = "crypt32")]
    extern "system" {
        pub fn CryptProtectData(  data_in: *const DataBlob, descr: *const u16,
            entropy: *const DataBlob, reserved: *mut c_void, prompt: *mut c_void,
            flags: u32, data_out: *mut DataBlob) -> i32;
        pub fn CryptUnprotectData(data_in: *const DataBlob, descr: *mut *mut u16,
            entropy: *const DataBlob, reserved: *mut c_void, prompt: *mut c_void,
            flags: u32, data_out: *mut DataBlob) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        pub fn LocalFree(mem: *mut c_void) -> *mut c_void;
        pub fn GetLastError() -> u32;
    }
}
```

Both wrappers copy `data_out` into a `Vec` and then `LocalFree(data_out.pbData)`.

**`crates/soul-store/src/keys.rs`** — new `DpapiKeyProvider::seed`, and `unprotect` becomes a
derivation rather than a refusal. The `#[cfg(not(windows))]` arm is untouched.

```rust
/// Read the DPAPI-protected seed, creating one on first use.
///
/// Two providers on the same `blob_path` therefore give the same DEK, and a
/// copy of the directory taken to another machine gives neither: the blob is
/// bound to this Windows account.
#[cfg(windows)]
fn seed(&self) -> KeyResult<Vec<u8>> {
    const SEED_LEN: usize = 64;
    const ENTROPY: &[u8] = b"soul/v1/keys.dpapi";
    match std::fs::read(&self.blob_path) {
        Ok(blob) => crate::dpapi::unprotect(&blob, ENTROPY).map_err(|code| {
            KeyError::Unavailable(format!(
                "DPAPI 打不开 {}（错误 {code}）。这通常表示它是另一个 Windows 帐户或另一台机器写的。",
                self.blob_path.display(),
            ))
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut seed = vec![0u8; SEED_LEN];
            OsRng.fill_bytes(&mut seed);
            let blob = crate::dpapi::protect(&seed, ENTROPY)
                .map_err(|code| KeyError::Unavailable(format!("DPAPI 拒绝保护新种子（错误 {code}）")))?;
            write_new_blob(&self.blob_path, &blob)?;   // .partial + rename, as write_stored_config does
            Ok(seed)
        }
        Err(e) => Err(KeyError::Io(e.to_string())),
    }
}

#[cfg(windows)]
fn unprotect(&self, domain: &str) -> KeyResult<SecretKey> {
    Ok(SecretKey::derive(domain, &self.seed()?))
}
```

Domain separation stays where it already is: `SecretKey::derive(DEK_DOMAIN | KEK_DOMAIN, seed)`,
identical to `TestKeyProvider`, so the two providers cannot disagree about what a KEK is.

**Docs:** `docs/SECURITY.md:44` and `docs/STATUS.md:78` must lose the "骨架" wording, and
`KeyProtection::Dpapi`'s doc comment in `soulcore/src/commands/session.rs:137` must lose
"Still a skeleton".

### 3.5 Which ACs it blocks on a real Win11 box

| AC | Blocked by DPAPI? | Why |
|---|---|---|
| **AC-01** (干净 Win11 → 托盘出现且不提权) | **No** | `Session::open` never fails; the tray, the manifest and the window do not need a store. AC-01 is blocked by B5 (packaging never run) and by the author-manual tray/UAC items, not by keys. |
| **AC-04** (导入 → 落加密库，无明文残留) | **Yes, and vacuously "passing"** | There is no open store to import into, so nothing is written, so "no plaintext residue" is trivially true and proves nothing. The CI evidence (`no_plaintext_at_rest.rs`) runs on `TestKeyProvider`, i.e. it tests the SQLCipher layer, not the shipped key chain. |
| **AC-08** (≥3 对话对象 → 节点≥3，边有证据) | **Yes** | `Session::people()` → `opened_store()` → `Err`. `STATUS.md` WP13 §1 says it: *"那台机器上 `store_opened` 是 false，`/graph` 给拒绝."* |

Secondary blast radius already noted in §2: 19 `Session::open` call sites in
`soulcore/tests/session_commands.rs` cannot pass on Windows until either DPAPI lands or
`open_with_keys` exists.

### 3.6 Tests that must stay red if the patch is wrong

| Test | Must be red when |
|---|---|
| `keys::tests::the_dpapi_provider_constructs_and_refuses_rather_than_inventing_a_key`, re-gated `#[cfg(not(windows))]` | the non-Windows arm ever gains a fallback instead of `Unsupported` |
| `a_blob_this_provider_did_not_write_is_a_failure_not_a_new_key` *(new, windows)* | garbage in `keys.dpapi` produces a fresh key. Must assert **both** `Err(KeyError::Unavailable(_))` **and** that the file is byte-identical afterwards. This is the single most important one: "if it does not decrypt, make a new one" silently destroys the user's database access and then presents a working, empty app |
| `the_seed_file_is_not_the_seed` *(new, windows)* | `std::fs::read(blob_path)` contains the raw seed bytes |
| `the_seed_is_created_once_and_read_back` *(new, windows)* | two providers on one path disagree, or `database_key() == key_encryption_key()` |
| `session_commands::the_session_says_which_key_material_opened_the_store` — flip the Windows arm to `store_opened == true` | DPAPI is claimed but the store still does not open |
| `the_reported_protection_matches_the_provider_that_answered` *(new)* | a fallback ships while `key_protection` still reads `"dpapi"`. This is the AC-honesty tripwire against option (3) |
| `one_store::one_session_hands_out_one_store` — remove the `(None, None)` arm on Windows | the documented gap is quietly kept as an accepted outcome after it is supposed to be closed |

---

## 4. perl / NASM / vendored OpenSSL on windows-latest — P1

**Current state.** `ci.yml:122-126`:

```yaml
- name: report build prerequisites
  shell: bash
  run: |
    perl --version | head -2 || echo "perl missing"
    nasm -v || echo "nasm missing"
```

Job 97477123290 output: `This is perl 5, version 42, subversion 2 (v5.42.2) built for
x86_64-cygwin` and `NASM version 2.16.01`. Both present. OpenSSL itself did not rebuild in either
run (`Swatinem/rust-cache` restored it — only workspace crates appear as `Compiling`), so the
vendored build is currently proven only by an older cold run.

**Three defects in the check itself.**

1. **It cannot fail.** Every command ends in `|| echo "… missing"`, and there is no
   `set -euo pipefail`. The step's own comment says naming the tools *"makes the dependency
   visible if that ever changes"* — visible in a log nobody opens on a green run. If GitHub drops
   Strawberry Perl from the image, this step prints `perl missing`, goes green, and the job dies
   ten minutes later inside `openssl-src`'s build script.
2. **It measures the wrong perl.** `openssl-src` runs
   `env::var("OPENSSL_SRC_PERL").unwrap_or(env::var("PERL").unwrap_or("perl"))` and resolves it
   against the **Windows** `PATH` of the cargo process (pwsh here). The step runs under
   `shell: bash`, where Git-for-Windows' Cygwin perl shadows Strawberry — which is exactly the
   perl it reported. The known failure mode is documented in
   [rust-openssl#2149](https://github.com/rust-openssl/rust-openssl/issues/2149): Cygwin perl
   splits `D:\…` `@INC` entries on `:` and `Configure` dies with a Unix-path error list.
3. **NASM's absence degrades silently.** `openssl-src` probes with `cmd /C where nasm` and, if it
   is missing, configures `no-asm` **without failing**. A silently different crypto build is
   precisely what the SBOM and `cargo deny` story exists to prevent.

**Blast radius.** `STATUS.md` WP02 §7 records that `soulcore` now depends on `soul-store` →
`rusqlite bundled-sqlcipher-vendored-openssl`, so this is on the critical path of *both* the
`test (windows-latest)` job and the `package` job's `cargo build --release -p soulcore --bin
soul-headless` and Tauri release build. One runner-image regression takes out Windows CI and the
release artifacts together.

**Patch sketch** — `.github/workflows/ci.yml`, a real gate in both `test-windows` and `package`,
before any cargo step:

```yaml
# The perl openssl-src will actually run is resolved against the Windows PATH,
# not bash's — Git's Cygwin perl shadows Strawberry there and breaks Configure
# on @INC paths that contain a drive letter (rust-openssl#2149). So pin it, and
# make a missing tool a failure rather than a line in a log.
- name: pin the toolchain the vendored OpenSSL build needs
  shell: pwsh
  run: |
    $perl = (Get-Command perl -All |
             Where-Object { $_.Source -notmatch '\\Git\\|\\msys64\\|\\cygwin' } |
             Select-Object -First 1).Source
    if (-not $perl) { throw 'no Windows-native perl on PATH; openssl-src cannot configure' }
    if (-not (Get-Command nasm -ErrorAction SilentlyContinue)) { throw 'nasm missing' }
    "PERL=$perl"              | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8
    "OPENSSL_SRC_PERL=$perl"  | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8
    "OPENSSL_RUST_USE_NASM=1" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8
```

`OPENSSL_RUST_USE_NASM=1` turns "nasm quietly missing → no-asm build" into a documented panic.

**Test that must stay red:** extend `crates/xtask/tests/self_test.rs` to read
`.github/workflows/ci.yml` and require that every `runs-on: windows-latest` job sets
`OPENSSL_SRC_PERL`, and that no prerequisite step in those jobs ends a command with `|| echo`.
Red if the gate is reverted to a report.

---

## 5. `tauri build` / NSIS / WebView2 — author-manual vs the `package` job — P1/P2

**What the `package` job does** (job 97477123106, ~41 min): checkout · toolchain · rust-cache ·
pnpm · node · `pnpm install` · `pnpm --filter @soul/desktop build` · `cargo build --release
--manifest-path apps/desktop/src-tauri/Cargo.toml --features custom-protocol` **(38 m 26 s)** +
`cargo build --release -p soulcore --bin soul-headless` · read the embedded manifest out of
`soul.exe` and assert `asInvoker` · `install-smoke.ps1 -SkipInstall` · upload two binaries.

**What it never does:** run the Tauri CLI at all. `bundle.active: true`, `targets: ["nsis"]`,
`nsis.installMode: "currentUser"`, `webviewInstallMode: {type: "skip"}`,
`createUpdaterArtifacts: false` are asserted only as *text* by `shell_is_local_only.rs`.

**The circularity argument is right about the wrong step.** *"为了证明安装器可信而去下载一个第三方安装器，是把自己绕进去了"*
applies to **bundling**. It does not apply to **building**: Tauri 2's `tauri build --no-bundle`
runs `beforeBuildCommand`, parses and validates `tauri.conf.json` with the tool that consumes it,
processes icons, embeds the frontend and runs cargo — and returns before
`crate::bundle::bundle()`, which is the only code path that fetches NSIS
([tauri-cli `build.rs`](https://github.com/tauri-apps/tauri/blob/8718d081/crates/tauri-cli/src/build.rs):
`if !options.no_bundle && (config.bundle.active || …)`). Today `cargo build --features
custom-protocol` bypasses the CLI entirely, so a malformed config — bad icon path, an invalid
`bundle` key, schema drift after a CLI bump — is discovered by the author at bundle time and
nowhere else.

**Patch sketch** — `.github/workflows/ci.yml`, `package` job. `@tauri-apps/cli` is already a
devDependency of `@soul/desktop` with a `tauri` script, so no new tooling:

```yaml
# --no-bundle stops before the bundler, which is the only step that would fetch
# NSIS. Everything the bundler needs first — the config the CLI actually parses,
# the icons, the embedded frontend — is exercised here rather than discovered on
# the author's machine. Bundling itself stays author-manual, checklist section 0.
- name: tauri build (everything up to the bundler)
  run: pnpm --filter @soul/desktop exec tauri build --no-bundle
- name: headless binary
  run: cargo build --release -p soulcore --bin soul-headless
```

The manifest-reading and smoke steps keep working: `--no-bundle` still emits
`apps/desktop/src-tauri/target/release/soul.exe`. `beforeBuildCommand` makes the separate
"frontend bundle" step redundant.

**Keeping the manual half from rotting (P2).** `scripts/author-manual-checklist.md` §0 should
require the produced installer's SHA-256, size and `tauri --version` to be pasted into a
checked-in `docs/RELEASE_EVIDENCE.md`, and `shell_is_local_only.rs` should gain an assertion that
fails when the `bundle` block of `tauri.conf.json` changes without that file being touched. Same
read-the-file-back idiom already used across this repo.

**WebView2 (B6, P2).** `webviewInstallMode: skip` is the correct call for AC-01 and AC-21 — the
installer neither elevates nor downloads — and it must **not** be changed to
`downloadBootstrapper`, which would put a network fetch inside the installer. What is missing is
the boundary written down: Windows 11 x64 ships the Evergreen runtime, Windows Server 2022/2025
and LTSC do not. A one-line supported-platform statement in the README and the checklist turns an
assumption into a promise. `ipc_roundtrip`'s `--no-run` skip is correctly reasoned and needs no
change, but `STATUS.md` should say that AC-02/AC-22's *IPC-layer* evidence on Windows is
compile-only.

---

## 6. Two smaller `screen.rs` notes (P2, not blockers)

- **B7.** In `unauthorized_paths::refusable`, entry 12 `("a verbatim prefix", format!("\\\\?\\{bravo}"))`
  and entry 13 `("a device prefix", …)` are degenerate on Windows: `tree.bravo()` is already
  `\\?\C:\…` because `Tree::build` canonicalizes, so these build `\\?\\\?\C:\…`. They are still
  refused (the strip leaves `\\?\C:\…`, which the two-separator rule catches), but they no longer
  test what their labels say. The plain verbatim spelling of `Bravo` *is* refused, via
  `OutsideAuthorizedRoot`, so this is a corpus-honesty defect rather than a hole. Fix: derive
  these two from a de-verbatimed base rather than from `tree.bravo()`.
- **B8.** `as_local_drive_spelling` accepts `\\?\C:/x` and `screen` then treats `/` as a
  separator, yielding `C:\x`. Win32 does not translate `/` inside a verbatim path, so the two
  spellings name different things on the target platform while screening identically. Not an
  escape — the result is still containment-checked and `canonicalize` will simply fail — but the
  screen's own promise is "two different strings that name the same file are caught", and this is
  the inverse. Fix if touched: refuse a `/` in a `\\?\`-prefixed input before stripping.

---

## 7. Ordering

`B2` (`--no-fail-fast`, one line) → `B1` (fileplan fixture) → `B3` (DPAPI) → `B4` (perl gate) →
`B5` (`--no-bundle`). `B2` first because until it lands, every subsequent Windows run reports one
failure at a time and the backlog stays invisible. `B1` before `B3` because the fileplan patch is
small and self-contained, and `B3` is the one that decides whether Goal 1 has a target platform.
