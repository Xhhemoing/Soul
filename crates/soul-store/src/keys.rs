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
//! * [`DpapiKeyProvider`] is the Windows one. Its skeleton is here so the
//!   storage code can already be written against the abstraction; the actual
//!   `CryptProtectData` calls are still outstanding, see the note on the type.

use std::fmt;
use std::path::{Path, PathBuf};

use chacha20poly1305::aead::rand_core::RngCore;
use chacha20poly1305::aead::OsRng;
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

    match std::fs::read(path) {
        Ok(bytes) if bytes.len() == SEED_LEN => Ok(bytes),
        Ok(bytes) => Err(KeyError::Malformed {
            path: path.display().to_string(),
            found: bytes.len(),
            expected: SEED_LEN,
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut seed = vec![0u8; SEED_LEN];
            OsRng.fill_bytes(&mut seed);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| KeyError::Io(e.to_string()))?;
            }
            std::fs::write(path, &seed).map_err(|e| KeyError::Io(e.to_string()))?;
            Ok(seed)
        }
        Err(error) => Err(KeyError::Io(error.to_string())),
    }
}

/// The Windows provider named by `docs/SECURITY.md`.
///
/// Outstanding work, tracked here rather than in a separate note because this
/// is where it lands: both accessors are meant to call `CryptProtectData` and
/// `CryptUnprotectData` against `blob_path`, which needs a Win32 binding crate
/// and therefore an `unsafe` block that this crate currently forbids. Until a
/// work package adds that dependency the provider reports
/// [`KeyError::Unsupported`] rather than silently inventing key material, so a
/// Windows build fails loudly instead of shipping an unprotected KEK.
///
/// Linux CI is unaffected: it uses [`TestKeyProvider`], and this type only has
/// to construct and compile.
#[derive(Debug, Clone)]
pub struct DpapiKeyProvider {
    blob_path: PathBuf,
}

impl DpapiKeyProvider {
    /// `blob_path` is where the DPAPI-protected seed lives, normally
    /// `%LOCALAPPDATA%\Soul\keys.dpapi`.
    pub fn new(blob_path: impl Into<PathBuf>) -> Self {
        DpapiKeyProvider {
            blob_path: blob_path.into(),
        }
    }

    pub fn blob_path(&self) -> &Path {
        &self.blob_path
    }

    #[cfg(windows)]
    fn unprotect(&self, domain: &str) -> KeyResult<SecretKey> {
        let _ = domain;
        Err(KeyError::Unsupported(format!(
            "the DPAPI-protected seed at {} cannot be unwrapped yet: the Win32 binding is not \
             wired up, so refusing to fabricate key material",
            self.blob_path.display(),
        )))
    }

    #[cfg(not(windows))]
    fn unprotect(&self, domain: &str) -> KeyResult<SecretKey> {
        let _ = domain;
        Err(KeyError::Unsupported(format!(
            "DPAPI is a Windows API and this is not Windows; use TestKeyProvider for the seed at {}",
            self.blob_path.display(),
        )))
    }
}

impl KeyProvider for DpapiKeyProvider {
    fn database_key(&self) -> KeyResult<SecretKey> {
        self.unprotect(DEK_DOMAIN)
    }

    fn key_encryption_key(&self) -> KeyResult<SecretKey> {
        self.unprotect(KEK_DOMAIN)
    }

    fn describe(&self) -> String {
        "dpapi-key-provider".into()
    }
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

    #[test]
    fn the_dpapi_provider_constructs_and_refuses_rather_than_inventing_a_key() {
        let provider = DpapiKeyProvider::new("C:/Users/example/AppData/Local/Soul/keys.dpapi");
        assert!(provider.blob_path().ends_with("keys.dpapi"));
        assert!(matches!(
            provider.database_key(),
            Err(KeyError::Unsupported(_))
        ));
        assert!(matches!(
            provider.key_encryption_key(),
            Err(KeyError::Unsupported(_))
        ));
    }
}
