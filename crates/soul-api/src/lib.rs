//! Authenticated loopback adapter; default startup requires a synthetic runtime.
//! The optional finite S1 host adds no client or arbitrary invoke surface.
#![cfg_attr(not(windows), forbid(unsafe_code))]
#![cfg_attr(windows, deny(unsafe_code))]

mod credentials;
mod deadline_io;
pub mod openapi;
mod openapi_writes;
mod read_activity;
mod transport;
mod writes;

use std::io;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use chacha20poly1305::aead::rand_core::{OsRng, RngCore};
use serde::Serialize;
use soul_runtime::Runtime;
use tokio::sync::watch;
use zeroize::Zeroizing;

/// Maximum response bytes; this bounds the transport, not the core graph scan.
pub const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
pub const MAX_CONNECTIONS: usize = 8;
pub const HEADER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
pub const WRITE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
pub const MAX_HEADER_BYTES: usize = 8192;
pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const BODY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

pub(crate) struct Secret(pub Zeroizing<String>);

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret([redacted])")
    }
}

/// A host-side header value whose diagnostic representation never reveals it.
pub struct Authorization(Zeroizing<String>);

impl Authorization {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl std::fmt::Debug for Authorization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Authorization([redacted])")
    }
}

/// A trusted local host writes this document to its owner-only credential file.
/// It is deliberately not Debug and is never an HTTP response.
#[derive(Serialize)]
pub struct CredentialDocument<'a> {
    pub endpoint: String,
    pub token: &'a str,
}

/// Owns the listening thread and all its in-flight operation tasks.
/// `shutdown` waits for dispatched work without holding a Session guard.
pub struct ApiServer {
    address: SocketAddr,
    secret: Arc<Secret>,
    stop: watch::Sender<bool>,
    thread: Mutex<Option<JoinHandle<()>>>,
    reads: Arc<read_activity::ReadActivity>,
}

impl std::fmt::Debug for ApiServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiServer")
            .field("address", &self.address)
            .finish_non_exhaustive()
    }
}

impl ApiServer {
    /// Explicit finite S1 validation host; unavailable in default builds.
    #[cfg(feature = "local-validation")]
    pub fn start_s1_validation(runtime: Runtime) -> io::Result<Self> {
        Self::start_with_scope(runtime, transport::HostScope::S1Validation)
    }

    pub fn start(runtime: Runtime) -> io::Result<Self> {
        if !runtime.is_synthetic() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "synthetic runtime required",
            ));
        }
        Self::start_with_scope(runtime, transport::HostScope::Synthetic)
    }

    fn start_with_scope(runtime: Runtime, scope: transport::HostScope) -> io::Result<Self> {
        let mut random = Zeroizing::new([0_u8; 32]);
        OsRng.try_fill_bytes(random.as_mut()).map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "credential generation unavailable")
        })?;
        let mut token = Zeroizing::new(String::with_capacity(64));
        use std::fmt::Write;
        for byte in random.iter() {
            write!(&mut *token, "{byte:02x}").expect("writing a String cannot fail");
        }
        let secret = Arc::new(Secret(token));
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let address = listener.local_addr()?;
        listener.set_nonblocking(true)?;
        // Build before spawning, so runtime initialization failures never look
        // like a successfully started listener to the host.
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .max_blocking_threads(MAX_CONNECTIONS)
            .build()?;
        let listener = {
            let _entered = executor.enter();
            tokio::net::TcpListener::from_std(listener)?
        };
        let (stop, receiver) = watch::channel(false);
        let worker_secret = Arc::clone(&secret);
        let reads = Arc::new(read_activity::ReadActivity::default());
        let worker_reads = Arc::clone(&reads);
        let thread = thread::Builder::new()
            .name("soul-local-api".into())
            .spawn(move || {
                executor.block_on(transport::serve(
                    listener,
                    address,
                    runtime,
                    worker_secret,
                    receiver,
                    Arc::clone(&worker_reads),
                    scope,
                ));
                // Connection disconnects can drop their async JoinHandle, but
                // admitted reads keep their own guard. Settle those reads
                // before dropping the executor, including queued workers.
                worker_reads.wait_empty();
            })?;
        Ok(Self {
            address,
            secret,
            stop,
            thread: Mutex::new(Some(thread)),
            reads,
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Safe host metadata used for lifecycle coordination, with no payloads.
    pub fn in_flight_reads(&self) -> usize {
        self.reads.count()
    }

    /// Includes reads and accepted writes; the older name remains compatible.
    pub fn in_flight_operations(&self) -> usize {
        self.reads.count()
    }

    /// Only for the trusted local host, never put this in logs or URLs.
    pub fn authorization(&self) -> Authorization {
        Authorization(Zeroizing::new(format!("Bearer {}", self.secret.0.as_str())))
    }

    pub fn credentials(&self) -> CredentialDocument<'_> {
        CredentialDocument {
            endpoint: format!("http://{}", self.address),
            token: self.secret.0.as_str(),
        }
    }

    /// Creates a new file with owner-only protection before writing any secret.
    pub fn write_credentials_new(&self, path: &std::path::Path) -> io::Result<()> {
        use std::io::Write;
        let bytes = Zeroizing::new(serde_json::to_vec(&self.credentials()).map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "credential encoding unavailable")
        })?);
        let mut file = credentials::create_new(path)?;
        if let Err(error) = file.write_all(&bytes).and_then(|_| file.sync_all()) {
            drop(file);
            let _ = std::fs::remove_file(path);
            return Err(error);
        }
        Ok(())
    }

    pub fn request_stop(&self) {
        let _ = self.stop.send(true);
    }

    pub fn shutdown(&self) {
        self.request_stop();
        if let Some(worker) = self
            .thread
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        {
            let _ = worker.join();
        }
    }
}

impl Drop for ApiServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}
