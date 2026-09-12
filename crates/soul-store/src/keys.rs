//! Where the two root secrets come from.
//!
//! `docs/SECURITY.md` pins the chain: DPAPI protects a KEK, the KEK wraps the
//! database DEK and every content key. This module is the seam between that
//! chain and the platform that stores it, so the storage code never has to know
//! whether it is running on Windows or on a headless Linux CI host.
//!
//! Two providers exist:
//!
//! * [`TestKeyProvider`] keeps key material in a fixed derivation or a file in
//!   a directory the caller owns. It touches no platform key store and is the
//!   only provider Linux CI uses.
//! * [`DpapiKeyProvider`] is the Windows one, and it now implements the chain
//!   rather than describing it: the KEK is protected by DPAPI under the
//!   logged-in user, and the database DEK is wrapped under that KEK. The two
//!   Win32 calls live in `soul-win-dpapi` so this crate can stay
//!   `#![forbid(unsafe_code)]`.
//!
//! Neither type is behind a `#[cfg]`. `soul-win-dpapi` compiles everywhere and
//! refuses everywhere it has no DPAPI, so the Windows provider is type-checked
//! by the Linux build and its file format is unit-tested there too — only the
//! syscall itself is Windows-only, and that is tested in `soul-win-dpapi`.

use std::fmt;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chacha20poly1305::aead::rand_core::RngCore;
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use fs4::FileExt;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

/// Length of every root secret in this module.
pub const KEY_LEN: usize = 32;

pub type KeyResult<T> = Result<T, KeyError>;

#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("key material is unavailable: {0}")]
    Unavailable(String),

    /// The provider cannot work on this platform at all.
    #[error("{0}")]
    Unsupported(String),

    #[error("reading or writing the key file failed: {0}")]
    Io(String),

    #[error("the key file at {path} holds {found} bytes, expected {expected}")]
    Malformed {
        path: String,
        found: usize,
        expected: usize,
    },

    /// The platform key blob exists and is not the shape this build writes.
    /// Reported rather than replaced: overwriting it would mint a new DEK and
    /// leave the database that the old one opened unreadable for good.
    #[error("the key blob at {path} is not one this build can read ({detail}); it has not been overwritten")]
    Corrupt { path: String, detail: String },
}

/// Thirty-two bytes of key material, wiped when dropped and never printed.
#[derive(Clone)]
pub struct SecretKey([u8; KEY_LEN]);

impl Drop for SecretKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl SecretKey {
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        SecretKey(bytes)
    }

    pub fn expose(&self) -> &[u8; KEY_LEN] {
        &self.0
    }

    /// Lowercase hex, as SQLCipher's raw-key pragma wants it.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Thirty-two bytes from the OS entropy source.
    pub fn random() -> Self {
        let mut bytes = [0u8; KEY_LEN];
        OsRng.fill_bytes(&mut bytes);
        SecretKey(bytes)
    }

    /// Domain-separated derivation, so one seed can yield several keys that
    /// cannot be substituted for one another.
    pub fn derive(domain: &str, seed: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(domain.as_bytes());
        hasher.update([0u8]);
        hasher.update(seed);
        let digest = hasher.finalize();
        let mut bytes = [0u8; KEY_LEN];
        bytes.copy_from_slice(&digest);
        SecretKey(bytes)
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretKey(<redacted>)")
    }
}

/// Supplies the two root secrets the encrypted store needs.
pub trait KeyProvider: fmt::Debug + Send + Sync {
    /// Whole-file key handed to SQLCipher. Protects every page of `soul.db`.
    fn database_key(&self) -> KeyResult<SecretKey>;

    /// Wraps every per-forget-unit content key in the `content_keys` table.
    fn key_encryption_key(&self) -> KeyResult<SecretKey>;

    /// Short label for logs and for the audit trail. Never key material.
    fn describe(&self) -> String;
}

const DEK_DOMAIN: &str = "soul/v1/database-dek";
const KEK_DOMAIN: &str = "soul/v1/kek";

/// Key material for headless tests and Linux CI.
///
/// It deliberately has no relationship with any platform key store: its whole
/// purpose is to let the storage contract run somewhere DPAPI does not exist.
/// Nothing here is suitable for a real installation, which is why the type name
/// says so and why `soulcore` only reaches for it from a test entry point.
#[derive(Debug, Clone)]
pub struct TestKeyProvider {
    source: TestKeySource,
}

#[derive(Debug, Clone)]
enum TestKeySource {
    /// Both keys derived from a caller-chosen string. Reproducible across
    /// processes, which is what the crash-recovery children need.
    Seed(String),
    /// Both keys read from, or created in, a file the caller owns. Usually a
    /// temporary directory that disappears with the test.
    File(PathBuf),
}

impl TestKeyProvider {
    /// Derive both keys from a fixed string.
    pub fn from_seed(seed: impl Into<String>) -> Self {
        TestKeyProvider {
            source: TestKeySource::Seed(seed.into()),
        }
    }

    /// Keep both keys in `dir/soul-test-keys.bin`, creating them on first use.
    pub fn in_dir(dir: impl AsRef<Path>) -> Self {
        TestKeyProvider {
            source: TestKeySource::File(dir.as_ref().join("soul-test-keys.bin")),
        }
    }

    fn material(&self) -> KeyResult<Vec<u8>> {
        match &self.source {
            TestKeySource::Seed(seed) => Ok(seed.as_bytes().to_vec()),
            TestKeySource::File(path) => read_or_create_key_file(path),
        }
    }
}

impl KeyProvider for TestKeyProvider {
    fn database_key(&self) -> KeyResult<SecretKey> {
        Ok(SecretKey::derive(DEK_DOMAIN, &self.material()?))
    }

    fn key_encryption_key(&self) -> KeyResult<SecretKey> {
        Ok(SecretKey::derive(KEK_DOMAIN, &self.material()?))
    }

    fn describe(&self) -> String {
        match &self.source {
            TestKeySource::Seed(_) => "test-key-provider(seed)".into(),
            TestKeySource::File(_) => "test-key-provider(file)".into(),
        }
    }
}

fn read_or_create_key_file(path: &Path) -> KeyResult<Vec<u8>> {
    const SEED_LEN: usize = 64;

    let settled = match std::fs::read(path) {
        Ok(bytes) if !bytes.is_empty() => bytes,
        // An empty file is not a malformed seed, it is a first run another
        // process is in the middle of. `mint_key_file` waits for it.
        Ok(_) => mint_key_file(path, SEED_LEN)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            mint_key_file(path, SEED_LEN)?
        }
        Err(error) => return Err(KeyError::Io(error.to_string())),
    };

    if settled.len() != SEED_LEN {
        return Err(KeyError::Malformed {
            path: path.display().to_string(),
            found: settled.len(),
            expected: SEED_LEN,
        });
    }
    Ok(settled)
}

/// Mint a seed for `path`, or read the one another process minted first.
fn mint_key_file(path: &Path, len: usize) -> KeyResult<Vec<u8>> {
    let mut seed = vec![0u8; len];
    OsRng.fill_bytes(&mut seed);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| KeyError::Io(e.to_string()))?;
    }
    let settled = publish_once(path, &seed).map_err(|e| KeyError::Io(e.to_string()));
    seed.zeroize();
    settled
}

// ------------------------------------------------ claiming the key file ---
//
// Every file in this module is a root secret that a database is then built
// against, so the first process to write one has to be the only process that
// ever does. Writing a temporary file and renaming it over the final name
// does not give that: two first runs both see no file, both mint, both
// rename, and the one that renames second silently takes the database the
// first one just created away from it — its DEK is gone and `soul.db` never
// opens again.
//
// `create_new` settles half of that: exactly one caller creates the name,
// everyone else is told `AlreadyExists` and reads what the winner wrote. It
// says nothing about the other half — a name that was created by a process
// that died before writing a byte into it. Somebody has to fill that empty
// file, and the obvious repair, unlink it and `create_new` again, puts the
// original race straight back: two recoverers both unlink and both create,
// the second one's file is the one that survives, and the first walks off
// with key material that is on nobody's disk. An unlink is worse than that
// even, because it can land after a third process has already filled the
// name, deleting a key some database is already open under.
//
// So the name is never unlinked. Nothing in this module deletes it, and
// nothing may be added that does. An empty file is settled where it lies,
// under an exclusive lock on the file itself: whoever takes the lock first
// looks at the length, writes if it is zero, reads if it is not, and the next
// holder of the lock therefore finds bytes rather than a decision to make.

/// How long a caller that lost the race waits for the winner's bytes.
///
/// The window it covers is the moment between the winner creating the name
/// and its single `write_all` landing — microseconds on a local disk. Running
/// out of it is neither an error nor a wrong answer any more: it hands the
/// question to the claim lock in [`fill_or_adopt`], which is slower and
/// certain. Polling first is worth it because it is much the cheaper of the
/// two on the ordinary second-launch path.
const CLAIM_SETTLE_TIMEOUT: Duration = Duration::from_millis(250);

/// Gap between looks at a file another process has claimed.
const CLAIM_SETTLE_POLL: Duration = Duration::from_millis(2);

/// Put `bytes` at `path` if and only if nothing is there yet, and hand back
/// whatever ended up there — our bytes if we were the caller that filled the
/// file, the winner's if somebody else was.
///
/// The caller must use the returned bytes and not the ones it passed in. That
/// is the whole point: a process that lost the race has to open the database
/// with the key that is on disk, not with the key it happened to mint.
fn publish_once(path: &Path, bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    // The loop is not a retry of the claim — `fill_or_adopt` settles that in
    // one pass. It bounds how many times the name may be seen to exist and
    // then be gone again by the time it is opened. Nothing here unlinks it, so
    // one turn is the whole story unless somebody is deleting the key file by
    // hand while Soul starts, and eight turns of that is enough to say so.
    for _ in 0..8 {
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(file) => return fill_or_adopt(file, bytes),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if let Some(existing) = settled_contents(path)? {
                    return Ok(existing);
                }
                // Still empty. Somebody created the name and did not fill it,
                // which is what an interrupted first run leaves behind. Open
                // what is there and settle it under the lock.
                match OpenOptions::new().read(true).write(true).open(path) {
                    Ok(file) => return fill_or_adopt(file, bytes),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => return Err(error),
                }
            }
            Err(error) => return Err(error),
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("{} kept vanishing while being claimed", path.display()),
    ))
}

/// Fill an already-created key file, or adopt what is already in it.
///
/// The exclusive lock is what makes the length check mean anything. Without
/// it, two recoverers of the same abandoned empty name both see zero bytes
/// and both write, and only one of those writes is the file anybody else will
/// read. With it the second one blocks, wakes up looking at a non-zero
/// length, and adopts.
///
/// The adopting read is issued on the locked handle rather than through a
/// second open of the path, and that is not a convenience. Windows locks a
/// byte range, so an unlocked read of a range a peer holds fails outright;
/// Unix `flock` is only advisory, so an unlocked read is free to come back
/// with half of a write that is still in progress. Neither is a hazard for a
/// reader that is holding the lock itself.
fn fill_or_adopt(mut file: std::fs::File, bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    FileExt::lock(&file)?;
    let outcome = if file.metadata()?.len() == 0 {
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map(|_| bytes.to_vec())
    } else {
        let mut existing = Vec::new();
        file.seek(SeekFrom::Start(0))
            .and_then(|_| file.read_to_end(&mut existing))
            .map(|_| existing)
    };
    // Closing the handle would release the lock anyway; unlocking first keeps
    // the window shut for exactly as long as the answer took to produce, and
    // a failure to unlock is not something the caller can act on.
    let _ = FileExt::unlock(&file);
    outcome
}

/// Read a file somebody else claimed, waiting out the gap between the claim
/// and the write that fills it.
///
/// `None` means the gap never closed: the file is still empty after
/// [`CLAIM_SETTLE_TIMEOUT`], or it has gone away again. Both answers send the
/// caller to [`fill_or_adopt`], which settles the question under the lock.
fn settled_contents(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    let deadline = Instant::now() + CLAIM_SETTLE_TIMEOUT;
    loop {
        match std::fs::read(path) {
            Ok(bytes) if !bytes.is_empty() => return Ok(Some(bytes)),
            Ok(_) => {}
            // A name that is not there is not a slow claim, so there is
            // nothing to wait for.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            // Anything else is treated as the transient it usually is. This
            // read is unlocked, and on Windows an unlocked read of a range a
            // peer holds the claim lock over fails with a lock violation —
            // which is precisely the moment this function exists to wait out.
            // An error that outlives the deadline is not swallowed: the
            // caller's next move is to open the same path, and a real
            // failure is reported from there.
            Err(_) => {}
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        std::thread::sleep(CLAIM_SETTLE_POLL);
    }
}

/// DPAPI's optional secondary entropy, for every call Soul makes.
///
/// Not a secret, and it is not pretending to be one: it is in this source
/// file. What it buys is that a blob Soul protected is not one another
/// application running as the same user unprotects by accident, and that a
/// future second use of DPAPI in this product can be given a different string
/// and so a separate blob. The protection itself comes from the user's
/// credentials.
const DPAPI_ENTROPY: &[u8] = b"soul/v1/dpapi/key-blob";

/// First eight bytes of `keys.dpapi`: a name and a format version.
const BLOB_MAGIC: &[u8; 8] = b"SOULKEY\x01";

/// XChaCha20-Poly1305 nonces are 24 bytes.
const NONCE_LEN: usize = 24;

/// Bound into the AEAD tag over the wrapped DEK, so the wrapped DEK cannot be
/// swapped for some other 32 bytes that happen to be wrapped under this KEK.
const DEK_WRAP_AAD: &str = DEK_DOMAIN;

/// The Windows provider named by `docs/SECURITY.md`, and the chain it states.
///
/// `blob_path` — normally `%LOCALAPPDATA%\Soul\keys.dpapi` — holds three
/// things:
///
/// 1. the **KEK**, protected by `CryptProtectData` under the logged-in user.
///    Nothing in this repository can recover it; only that account on that
///    machine can;
/// 2. the **database DEK**, a separate random key wrapped under the KEK with
///    XChaCha20-Poly1305 and [`DEK_WRAP_AAD`] as additional data;
/// 3. enough of a header to tell a blob this build understands from one it
///    does not.
///
/// That is `DPAPI → KEK → DEK`, with the third arrow — `KEK → 每单元 CK` —
/// already implemented by [`crate::store`] in the `content_keys` table. The
/// file is created on first use and never rewritten afterwards: it is the only
/// thing on the machine that can open `soul.db`, so a provider that replaced a
/// blob it could not read would be a provider that could throw the database
/// away. "Created on first use" is enforced against two first runs happening
/// at once, not just against the second launch — see [`publish_once`].
///
/// Off Windows every accessor returns [`KeyError::Unsupported`] and no file is
/// written. That is not a placeholder — it is the honest answer, and it is why
/// Linux CI runs [`TestKeyProvider`] instead.
#[derive(Debug, Clone)]
pub struct DpapiKeyProvider {
    blob_path: PathBuf,
}

/// The two root secrets, as they come out of one read of the blob.
struct RootKeys {
    dek: SecretKey,
    kek: SecretKey,
}

impl DpapiKeyProvider {
    /// `blob_path` is where the DPAPI-protected KEK lives, normally
    /// `%LOCALAPPDATA%\Soul\keys.dpapi`.
    pub fn new(blob_path: impl Into<PathBuf>) -> Self {
        DpapiKeyProvider {
            blob_path: blob_path.into(),
        }
    }

    pub fn blob_path(&self) -> &Path {
        &self.blob_path
    }

    /// Whether this build can reach DPAPI at all. Cheaper to ask than to find
    /// out, and it is the difference between "no key material here yet" and
    /// "no key material here, ever".
    pub fn is_available() -> bool {
        soul_win_dpapi::SUPPORTED
    }

    /// Read the blob, creating it on first use, and unwrap both root secrets.
    ///
    /// Called once per accessor rather than cached. Two DPAPI calls per store
    /// open is nothing, and a cache would be a copy of the KEK living for as
    /// long as the provider rather than for as long as the call.
    fn root_keys(&self) -> KeyResult<RootKeys> {
        if !Self::is_available() {
            return Err(KeyError::Unsupported(format!(
                "DPAPI is a Windows API and this is not Windows, so the key blob at {} cannot be \
                 opened; headless and CI runs use TestKeyProvider",
                self.blob_path.display(),
            )));
        }

        match std::fs::read(&self.blob_path) {
            Ok(bytes) if !bytes.is_empty() => self.open_blob(&bytes),
            // An empty blob is a first run another process is part-way
            // through, not a blob; `create_blob` waits for it.
            Ok(_) => {
                let bytes = self.create_blob()?;
                self.open_blob(&bytes)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let bytes = self.create_blob()?;
                self.open_blob(&bytes)
            }
            Err(error) => Err(KeyError::Io(format!(
                "{}: {error}",
                self.blob_path.display()
            ))),
        }
    }

    /// First run on this machine: mint both keys and hand the KEK to Windows.
    ///
    /// Returns the bytes that are *on disk* afterwards, which are not always
    /// the ones minted here. Two first runs at once — the installer's launch
    /// and the user's double-click, say — both find no blob and both mint a
    /// KEK and a DEK. Only one of them may end up in the file, and the other
    /// has to adopt it rather than open the database under a DEK that is about
    /// to be overwritten. [`publish_once`] decides which one that is; the
    /// keys minted by the loser fall out of scope here and are wiped.
    fn create_blob(&self) -> KeyResult<Vec<u8>> {
        let kek = SecretKey::random();
        let dek = SecretKey::random();

        let protected_kek = soul_win_dpapi::protect(kek.expose(), DPAPI_ENTROPY)
            .map_err(|error| self.dpapi_error("protecting a new KEK", error))?;
        let (nonce, wrapped_dek) = wrap_dek(&kek, &dek).map_err(|detail| KeyError::Corrupt {
            path: self.blob_path.display().to_string(),
            detail,
        })?;

        let encoded = encode_blob(&protected_kek, &nonce, &wrapped_dek);
        if let Some(parent) = self.blob_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| KeyError::Io(format!("{}: {error}", parent.display())))?;
        }
        publish_once(&self.blob_path, &encoded)
            .map_err(|error| KeyError::Io(format!("{}: {error}", self.blob_path.display())))
    }

    fn open_blob(&self, bytes: &[u8]) -> KeyResult<RootKeys> {
        let parts = decode_blob(bytes).map_err(|detail| KeyError::Corrupt {
            path: self.blob_path.display().to_string(),
            detail,
        })?;

        let kek_bytes = soul_win_dpapi::unprotect(parts.protected_kek, DPAPI_ENTROPY)
            .map_err(|error| self.dpapi_error("unprotecting the KEK", error))?;
        let kek = secret_from(&kek_bytes).map_err(|detail| KeyError::Corrupt {
            path: self.blob_path.display().to_string(),
            detail: format!("the protected KEK {detail}"),
        })?;

        let dek = unwrap_dek(&kek, parts.dek_nonce, parts.wrapped_dek).map_err(|detail| {
            KeyError::Corrupt {
                path: self.blob_path.display().to_string(),
                detail,
            }
        })?;

        Ok(RootKeys { dek, kek })
    }

    /// A DPAPI failure, named without repeating any of the bytes involved.
    fn dpapi_error(&self, doing: &str, error: soul_win_dpapi::DpapiError) -> KeyError {
        match error {
            soul_win_dpapi::DpapiError::Unsupported => KeyError::Unsupported(format!(
                "DPAPI is unavailable in this build, so {doing} for {} is impossible",
                self.blob_path.display(),
            )),
            other => KeyError::Unavailable(format!(
                "{doing} for {} failed: {other}",
                self.blob_path.display(),
            )),
        }
    }
}

impl KeyProvider for DpapiKeyProvider {
    fn database_key(&self) -> KeyResult<SecretKey> {
        Ok(self.root_keys()?.dek)
    }

    fn key_encryption_key(&self) -> KeyResult<SecretKey> {
        Ok(self.root_keys()?.kek)
    }

    fn describe(&self) -> String {
        "dpapi-key-provider(current-user)".into()
    }
}

/// Wrap the DEK under the KEK. The nonce is fresh and stored beside the
/// ciphertext; the KEK never encrypts anything else in this file.
fn wrap_dek(kek: &SecretKey, dek: &SecretKey) -> Result<(Vec<u8>, Vec<u8>), String> {
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let wrapped = XChaCha20Poly1305::new(Key::from_slice(kek.expose()))
        .encrypt(
            &nonce,
            Payload {
                msg: dek.expose(),
                aad: DEK_WRAP_AAD.as_bytes(),
            },
        )
        .map_err(|_| "wrapping the database key under the KEK failed".to_owned())?;
    Ok((nonce.to_vec(), wrapped))
}

fn unwrap_dek(kek: &SecretKey, nonce: &[u8], wrapped: &[u8]) -> Result<SecretKey, String> {
    if nonce.len() != NONCE_LEN {
        return Err(format!(
            "the wrapping nonce is {} bytes, expected {NONCE_LEN}",
            nonce.len()
        ));
    }
    let raw = XChaCha20Poly1305::new(Key::from_slice(kek.expose()))
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: wrapped,
                aad: DEK_WRAP_AAD.as_bytes(),
            },
        )
        .map_err(|_| "the database key did not unwrap under this KEK".to_owned())?;
    let mut raw = raw;
    let key = secret_from(&raw).map_err(|detail| format!("the unwrapped database key {detail}"));
    raw.zeroize();
    key
}

fn secret_from(bytes: &[u8]) -> Result<SecretKey, String> {
    if bytes.len() != KEY_LEN {
        return Err(format!("is {} bytes, expected {KEY_LEN}", bytes.len()));
    }
    let mut fixed = [0u8; KEY_LEN];
    fixed.copy_from_slice(bytes);
    Ok(SecretKey::from_bytes(fixed))
}

/// The three variable-length pieces of `keys.dpapi`, borrowed from the file.
struct BlobParts<'a> {
    protected_kek: &'a [u8],
    dek_nonce: &'a [u8],
    wrapped_dek: &'a [u8],
}

/// `magic ‖ u32 len ‖ protected KEK ‖ 24-byte nonce ‖ u32 len ‖ wrapped DEK`.
///
/// Lengths are little-endian. Two length fields for three pieces because the
/// nonce is fixed-width, and the file has to end exactly where the second
/// length says it does — see [`decode_blob`].
fn encode_blob(protected_kek: &[u8], dek_nonce: &[u8], wrapped_dek: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        BLOB_MAGIC.len() + 8 + protected_kek.len() + NONCE_LEN + wrapped_dek.len(),
    );
    out.extend_from_slice(BLOB_MAGIC);
    out.extend_from_slice(&(protected_kek.len() as u32).to_le_bytes());
    out.extend_from_slice(protected_kek);
    out.extend_from_slice(dek_nonce);
    out.extend_from_slice(&(wrapped_dek.len() as u32).to_le_bytes());
    out.extend_from_slice(wrapped_dek);
    out
}

/// Read a blob back, or say what is wrong with it in one clause.
///
/// Strict on purpose, including about trailing bytes: this file is the only
/// way into the database, and a parser that shrugged at extra bytes would be
/// one that could be steered by appending to it.
fn decode_blob(bytes: &[u8]) -> Result<BlobParts<'_>, String> {
    let mut rest = bytes
        .strip_prefix(&BLOB_MAGIC[..])
        .ok_or_else(|| match bytes.len() {
            0 => "the file is empty".to_owned(),
            _ => "the file does not start with this build's key-blob header".to_owned(),
        })?;

    let protected_kek = take_length_prefixed(&mut rest, "the protected KEK")?;
    if rest.len() < NONCE_LEN {
        return Err(format!(
            "the file ends before its {NONCE_LEN}-byte wrapping nonce"
        ));
    }
    let (dek_nonce, after_nonce) = rest.split_at(NONCE_LEN);
    rest = after_nonce;
    let wrapped_dek = take_length_prefixed(&mut rest, "the wrapped database key")?;
    if !rest.is_empty() {
        return Err(format!("{} unexpected trailing byte(s)", rest.len()));
    }

    Ok(BlobParts {
        protected_kek,
        dek_nonce,
        wrapped_dek,
    })
}

fn take_length_prefixed<'a>(rest: &mut &'a [u8], what: &str) -> Result<&'a [u8], String> {
    if rest.len() < 4 {
        return Err(format!("the file ends before the length of {what}"));
    }
    let (length, after) = rest.split_at(4);
    let length = u32::from_le_bytes(length.try_into().expect("four bytes")) as usize;
    if length == 0 {
        return Err(format!("{what} is declared as zero bytes"));
    }
    if after.len() < length {
        return Err(format!(
            "{what} is declared as {length} bytes and only {} follow",
            after.len(),
        ));
    }
    let (taken, remaining) = after.split_at(length);
    *rest = remaining;
    Ok(taken)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_yields_two_different_keys_and_repeats_them() {
        let provider = TestKeyProvider::from_seed("fixed");
        let dek = provider.database_key().expect("dek");
        let kek = provider.key_encryption_key().expect("kek");
        assert_ne!(
            dek.expose(),
            kek.expose(),
            "domain separation must keep the database key and the KEK distinct",
        );
        assert_eq!(
            dek.to_hex(),
            TestKeyProvider::from_seed("fixed")
                .database_key()
                .expect("dek again")
                .to_hex(),
            "a fixed seed must survive a process boundary",
        );
        assert_ne!(
            dek.to_hex(),
            TestKeyProvider::from_seed("other")
                .database_key()
                .expect("dek")
                .to_hex(),
        );
        assert_eq!(dek.to_hex().len(), KEY_LEN * 2);
    }

    #[test]
    fn a_secret_key_never_prints_its_bytes() {
        let key = SecretKey::from_bytes([7u8; KEY_LEN]);
        let rendered = format!("{key:?}");
        assert!(!rendered.contains('7'), "got {rendered}");
        assert!(rendered.contains("redacted"));
    }

    fn scratch() -> tempfile::TempDir {
        tempfile::tempdir().expect("temporary directory")
    }

    // ------------------------------------------------- one first run wins ---
    //
    // The claim is platform-independent — it is `create_new` and nothing else
    // — so it is tested wherever the suite runs, including the Linux CI host
    // that has no DPAPI to mint a blob with.

    /// Eight processes mint eight different keys at once. Seven of them must
    /// come back with the eighth one's bytes.
    ///
    /// The failure this pins is not "the file is torn". It is that a caller
    /// which lost the race used to be handed the key material it minted
    /// itself, build a database against it, and lose that database the moment
    /// the winner's rename landed on top.
    #[test]
    fn only_one_of_many_first_runs_writes_the_key_file_and_the_rest_adopt_it() {
        let directory = scratch();
        let path = directory.path().join("keys.dpapi");
        let racers = 8;
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(racers));

        let settled: Vec<Vec<u8>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..racers)
                .map(|index| {
                    let barrier = std::sync::Arc::clone(&barrier);
                    let path = path.clone();
                    scope.spawn(move || {
                        // Distinct payloads, so the answer names its author.
                        let mine = vec![index as u8 + 1; 128];
                        barrier.wait();
                        publish_once(&path, &mine).expect("claim the key file")
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("racer"))
                .collect()
        });

        let on_disk = std::fs::read(&path).expect("the key file");
        assert!(!on_disk.is_empty());
        for (index, answer) in settled.iter().enumerate() {
            assert_eq!(
                answer, &on_disk,
                "racer {index} was handed key material that is not the one on disk",
            );
        }
        assert_eq!(
            std::fs::read_dir(directory.path())
                .expect("read the directory")
                .count(),
            1,
            "the race left something behind beside the key file",
        );
    }

    /// A second call is a second launch, and it must not mint anything.
    #[test]
    fn a_published_key_file_is_never_rewritten() {
        let directory = scratch();
        let path = directory.path().join("keys.dpapi");

        assert_eq!(publish_once(&path, b"first").expect("first"), b"first");
        assert_eq!(
            publish_once(&path, b"second").expect("second"),
            b"first",
            "the second caller overwrote key material that was already in use",
        );
        assert_eq!(std::fs::read(&path).expect("the file"), b"first");
    }

    /// The one state the claim can leave behind: a name created by a process
    /// that died before it wrote. Nothing was ever opened with those zero
    /// bytes, so the next run may fill them in place and mint for real —
    /// without the name ever leaving the filesystem.
    #[test]
    fn a_name_claimed_and_left_empty_is_reclaimed_rather_than_reported() {
        let directory = scratch();
        let path = directory.path().join("keys.dpapi");
        std::fs::write(&path, b"").expect("an interrupted first run");

        assert_eq!(publish_once(&path, b"minted").expect("mint"), b"minted");
        assert_eq!(std::fs::read(&path).expect("the file"), b"minted");
    }

    /// The same recovery, but with eight processes attempting it at once.
    ///
    /// This is the harder half of the empty-file case and the one that used to
    /// be wrong. Every racer sees the same abandoned name, so every racer is a
    /// recoverer; the file has to end up holding exactly one of their keys and
    /// all eight have to be handed that one. A recovery that unlinks the name
    /// before minting cannot promise that — two recoverers both unlink it and
    /// both create it, and the loser walks away with key material that is not
    /// on disk. Four rounds because the divergence is a timing window and one
    /// round can miss it.
    #[test]
    fn racing_recoverers_of_an_abandoned_empty_file_agree_on_one_key() {
        let racers = 8;

        for round in 0..4 {
            let directory = scratch();
            let path = directory.path().join("keys.dpapi");
            std::fs::write(&path, b"").expect("an interrupted first run");
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(racers));

            let settled: Vec<Vec<u8>> = std::thread::scope(|scope| {
                let handles: Vec<_> = (0..racers)
                    .map(|index| {
                        let barrier = std::sync::Arc::clone(&barrier);
                        let path = path.clone();
                        scope.spawn(move || {
                            // Distinct payloads, so the answer names its author.
                            let mine = vec![index as u8 + 1; 128];
                            barrier.wait();
                            publish_once(&path, &mine).expect("reclaim the key file")
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .map(|handle| handle.join().expect("racer"))
                    .collect()
            });

            let on_disk = std::fs::read(&path).expect("the key file");
            assert!(
                !on_disk.is_empty(),
                "round {round} left the abandoned name still empty",
            );
            for (index, answer) in settled.iter().enumerate() {
                assert_eq!(
                    answer, &on_disk,
                    "round {round}: recoverer {index} was handed key material that is not the \
                     one on disk",
                );
            }
            assert_eq!(
                std::fs::read_dir(directory.path())
                    .expect("read the directory")
                    .count(),
                1,
                "round {round} left something behind beside the key file",
            );
        }
    }

    /// The same race through the provider that Linux CI actually uses.
    #[test]
    fn racing_test_providers_in_one_directory_agree_on_both_root_secrets() {
        let directory = scratch();
        let racers = 8;
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(racers));

        let keys: Vec<(String, String)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..racers)
                .map(|_| {
                    let barrier = std::sync::Arc::clone(&barrier);
                    let provider = TestKeyProvider::in_dir(directory.path());
                    scope.spawn(move || {
                        barrier.wait();
                        (
                            provider.database_key().expect("dek").to_hex(),
                            provider.key_encryption_key().expect("kek").to_hex(),
                        )
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("racer"))
                .collect()
        });

        let next_launch = TestKeyProvider::in_dir(directory.path());
        let expected = (
            next_launch.database_key().expect("dek").to_hex(),
            next_launch.key_encryption_key().expect("kek").to_hex(),
        );
        for (index, pair) in keys.iter().enumerate() {
            assert_eq!(
                pair, &expected,
                "racer {index} would have built a database against a key nobody else has",
            );
        }
        assert_ne!(expected.0, expected.1);
    }

    // ------------------------------------------------- the blob, anywhere ---
    //
    // The file format and the KEK-wraps-DEK step have no platform in them, so
    // they are checked on whichever host is running the suite. What is left
    // for Windows is the DPAPI call itself, and `soul-win-dpapi` owns that.

    #[test]
    fn a_blob_survives_a_round_trip_through_its_own_encoding() {
        let protected = vec![7u8; 300];
        let nonce = vec![9u8; NONCE_LEN];
        let wrapped = vec![3u8; KEY_LEN + 16];

        let encoded = encode_blob(&protected, &nonce, &wrapped);
        let parts = decode_blob(&encoded).expect("its own output");
        assert_eq!(parts.protected_kek, &protected[..]);
        assert_eq!(parts.dek_nonce, &nonce[..]);
        assert_eq!(parts.wrapped_dek, &wrapped[..]);
    }

    /// Every way the one file that opens the database can be wrong. None of
    /// them may parse: this build would rather refuse to start than mint a
    /// second DEK over the top of the one the database was built with.
    #[test]
    fn a_blob_that_is_not_exactly_right_is_refused_rather_than_read() {
        let good = encode_blob(&[7u8; 300], &[9u8; NONCE_LEN], &[3u8; KEY_LEN + 16]);

        let mut wrong_magic = good.clone();
        wrong_magic[7] = b'\x02';
        let mut trailing = good.clone();
        trailing.push(0);
        let mut lying_length = good.clone();
        lying_length[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        let mut zero_length = good.clone();
        zero_length[8..12].copy_from_slice(&0u32.to_le_bytes());

        let broken: Vec<Vec<u8>> = vec![
            Vec::new(),
            b"SOULKEY".to_vec(),
            wrong_magic,
            trailing,
            lying_length,
            zero_length,
            good[..good.len() - 1].to_vec(),
            good[..BLOB_MAGIC.len() + 4].to_vec(),
            good[..BLOB_MAGIC.len() + 4 + 300 + 4].to_vec(),
        ];
        for (index, bytes) in broken.iter().enumerate() {
            let refusal = decode_blob(bytes);
            assert!(
                refusal.is_err(),
                "case {index} parsed a blob it should have refused",
            );
        }

        assert!(decode_blob(&good).is_ok(), "the control case must parse");
    }

    #[test]
    fn the_database_key_travels_wrapped_under_the_kek_and_nowhere_else() {
        let kek = SecretKey::from_bytes([1u8; KEY_LEN]);
        let dek = SecretKey::from_bytes([2u8; KEY_LEN]);
        let (nonce, wrapped) = wrap_dek(&kek, &dek).expect("wrap");

        assert_ne!(
            &wrapped[..KEY_LEN],
            dek.expose(),
            "the wrapped DEK repeats the DEK",
        );
        assert_eq!(
            unwrap_dek(&kek, &nonce, &wrapped).expect("unwrap").expose(),
            dek.expose(),
        );
        assert!(
            unwrap_dek(&SecretKey::from_bytes([4u8; KEY_LEN]), &nonce, &wrapped).is_err(),
            "another KEK unwrapped the database key",
        );

        let mut altered = wrapped.clone();
        altered[0] ^= 0xff;
        assert!(unwrap_dek(&kek, &nonce, &altered).is_err());
        assert!(unwrap_dek(&kek, &nonce[..NONCE_LEN - 1], &wrapped).is_err());
    }

    /// `wrap_dek` binds the role into the AEAD tag, so a wrapped content key —
    /// the other thing this KEK wraps, over in `content_keys` — cannot be
    /// pushed into the blob and unwrapped as a database key.
    #[test]
    fn a_key_wrapped_for_some_other_purpose_does_not_unwrap_as_the_database_key() {
        let kek = SecretKey::from_bytes([1u8; KEY_LEN]);
        let dek = SecretKey::from_bytes([2u8; KEY_LEN]);
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let elsewhere = XChaCha20Poly1305::new(Key::from_slice(kek.expose()))
            .encrypt(
                &nonce,
                Payload {
                    msg: dek.expose(),
                    aad: b"content-key|0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4001",
                },
            )
            .expect("wrap under another context");
        assert!(unwrap_dek(&kek, &nonce, &elsewhere).is_err());
    }

    // ------------------------------------------------------- the provider ---

    #[test]
    fn the_provider_keeps_the_path_it_was_given() {
        let provider = DpapiKeyProvider::new("C:/Users/example/AppData/Local/Soul/keys.dpapi");
        assert!(provider.blob_path().ends_with("keys.dpapi"));
        assert!(provider.describe().contains("dpapi"));
        assert_eq!(DpapiKeyProvider::is_available(), cfg!(windows));
    }

    /// Windows: the whole chain, through the real `CryptProtectData`.
    ///
    /// This is the test that says the provider is no longer a skeleton. It
    /// runs on `windows-latest` under `cargo test --workspace --all-targets`;
    /// there is nowhere else it can run, and a version of it with the syscall
    /// faked out would be a test of the fake.
    #[cfg(windows)]
    #[test]
    fn on_windows_the_keys_are_created_once_and_recovered_every_time_after() {
        let directory = scratch();
        let path = directory.path().join("keys.dpapi");
        let provider = DpapiKeyProvider::new(&path);

        assert!(!path.exists(), "nothing exists before the first call");
        let dek = provider.database_key().expect("first DEK");
        let kek = provider.key_encryption_key().expect("first KEK");
        assert!(
            path.is_file(),
            "the first call has to leave the blob behind"
        );
        assert_ne!(
            dek.expose(),
            kek.expose(),
            "the two root secrets must not be the same key",
        );

        // A second provider on the same path is what the next launch is.
        let next_launch = DpapiKeyProvider::new(&path);
        assert_eq!(
            next_launch.database_key().expect("DEK").to_hex(),
            dek.to_hex()
        );
        assert_eq!(
            next_launch.key_encryption_key().expect("KEK").to_hex(),
            kek.to_hex()
        );

        // And the file it read holds neither of them in the clear.
        let bytes = std::fs::read(&path).expect("the blob");
        for secret in [dek.expose(), kek.expose()] {
            assert!(
                !bytes.windows(KEY_LEN).any(|window| window == &secret[..]),
                "the key blob contains a root secret in the clear",
            );
        }
        assert!(!directory.path().join("keys.partial").exists());
    }

    /// Windows: a blob that will not parse stops the store rather than being
    /// replaced. Replacing it would mint a new DEK and lose the database.
    #[cfg(windows)]
    #[test]
    fn on_windows_an_unreadable_blob_is_reported_and_left_alone() {
        let directory = scratch();
        let path = directory.path().join("keys.dpapi");
        std::fs::write(&path, b"this is not a key blob").expect("write");

        let provider = DpapiKeyProvider::new(&path);
        assert!(matches!(
            provider.database_key(),
            Err(KeyError::Corrupt { .. })
        ));
        assert_eq!(
            &std::fs::read(&path).expect("still there")[..],
            &b"this is not a key blob"[..],
        );
    }

    /// Off Windows the provider refuses, and — the half worth pinning — it
    /// writes nothing while refusing. A provider that left a file behind on a
    /// platform it cannot serve would be one that had invented key material
    /// and then declined to use it.
    #[cfg(not(windows))]
    #[test]
    fn off_windows_the_provider_refuses_and_leaves_no_key_material_behind() {
        let directory = scratch();
        let path = directory.path().join("keys.dpapi");
        let provider = DpapiKeyProvider::new(&path);

        assert!(matches!(
            provider.database_key(),
            Err(KeyError::Unsupported(_))
        ));
        assert!(matches!(
            provider.key_encryption_key(),
            Err(KeyError::Unsupported(_))
        ));
        assert!(!path.exists(), "a refusal wrote a key blob");
        assert_eq!(
            std::fs::read_dir(directory.path())
                .expect("read the directory")
                .count(),
            0,
        );
    }
}
