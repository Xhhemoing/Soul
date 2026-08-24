//! What this process's sockets are actually connected to, while it runs.
//!
//! AC-21 asks two different questions and they need two different answers.
//! "No business domain in the source" is a grep, and `xtask e0-audit` is that
//! grep. "Zero non-loopback connections" is not: a program can reach the
//! network without a URL literal anywhere near it — through a dependency, a
//! DNS lookup, a crash reporter someone linked in — and the only honest way to
//! answer it is to look at the sockets the kernel says this process holds.
//!
//! So this module reads `/proc`. It is deliberately an *observation* rather
//! than an interception: nothing here can prevent a connection, and nothing
//! about the rest of Soul changes because it is running. What it produces is
//! evidence, which is what an acceptance criterion about network behaviour has
//! to rest on.
//!
//! ## Sampling, not a snapshot
//!
//! A single reading at the end of a run would miss a connection that opened
//! and closed in the middle of it — which is the shape almost every real leak
//! has. [`Watch`] therefore samples on a background thread for as long as the
//! flow runs and accumulates every distinct non-loopback peer it ever saw.
//!
//! ## Windows
//!
//! There is no `/proc`, and reading the Windows TCP table means `unsafe`
//! bindings, which this crate forbids. The observation is not silently skipped
//! and it is not faked: [`Support::Unsupported`] says so by name, and
//! `scripts/install-smoke.ps1` makes the same observation on Windows with
//! `Get-NetTCPConnection -OwningProcess`, where PowerShell has the API and this
//! crate does not.

use std::collections::BTreeSet;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// How often the watcher looks. Short enough that a request-and-close inside
/// one HTTP round trip is still seen several times.
pub const SAMPLE_INTERVAL: Duration = Duration::from_millis(5);

/// The `/proc/net` tables a connected socket can appear in.
#[cfg(target_os = "linux")]
const TABLES: &[(&str, &str)] = &[
    ("tcp", "/proc/net/tcp"),
    ("tcp6", "/proc/net/tcp6"),
    ("udp", "/proc/net/udp"),
    ("udp6", "/proc/net/udp6"),
];

/// One socket of this process that has a peer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Peer {
    pub protocol: String,
    pub address: String,
    pub port: u16,
}

impl fmt::Display for Peer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}:{}", self.protocol, self.address, self.port)
    }
}

/// Whether the platform let us look.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "detail")]
pub enum Support {
    Observed,
    /// Named rather than treated as "nothing found". A check that cannot run
    /// must not read as a check that passed.
    Unsupported(String),
}

impl Support {
    pub fn is_observed(&self) -> bool {
        matches!(self, Support::Observed)
    }
}

/// One reading of this process's connected sockets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub support: Support,
    pub loopback: Vec<Peer>,
    /// Everything that is not loopback. PRODUCT_LOCK's "zero business egress"
    /// is this list being empty while no E1 endpoint is configured.
    pub non_loopback: Vec<Peer>,
}

impl Observation {
    fn unsupported(reason: &str) -> Observation {
        Observation {
            support: Support::Unsupported(reason.to_owned()),
            loopback: Vec::new(),
            non_loopback: Vec::new(),
        }
    }
}

/// Read this process's connected sockets once.
pub fn observe() -> Observation {
    observe_impl()
}

#[cfg(not(target_os = "linux"))]
fn observe_impl() -> Observation {
    Observation::unsupported(
        "sockets are read through /proc, which this platform does not have; \
         scripts/install-smoke.ps1 observes them on Windows instead",
    )
}

#[cfg(target_os = "linux")]
fn observe_impl() -> Observation {
    let Some(inodes) = own_socket_inodes() else {
        return Observation::unsupported("/proc/self/fd could not be read");
    };

    let mut loopback = Vec::new();
    let mut non_loopback = Vec::new();
    let mut read_any = false;
    for (protocol, path) in TABLES {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        read_any = true;
        for peer in peers_in_table(&text, protocol, &inodes) {
            match peer.address.parse::<IpAddr>().map(|ip| ip.is_loopback()) {
                Ok(true) => loopback.push(peer),
                _ => non_loopback.push(peer),
            }
        }
    }
    if !read_any {
        return Observation::unsupported("no /proc/net table could be read");
    }

    loopback.sort();
    loopback.dedup();
    non_loopback.sort();
    non_loopback.dedup();
    Observation {
        support: Support::Observed,
        loopback,
        non_loopback,
    }
}

/// Socket inodes belonging to this process, from `/proc/self/fd`.
///
/// The inode is what ties a file descriptor this process holds to a row in the
/// shared `/proc/net` tables, which list every socket on the machine. Without
/// it the observation would be about the host and not about Soul.
#[cfg(target_os = "linux")]
fn own_socket_inodes() -> Option<BTreeSet<u64>> {
    let mut inodes = BTreeSet::new();
    for entry in std::fs::read_dir("/proc/self/fd").ok()? {
        let Ok(entry) = entry else { continue };
        let Ok(target) = std::fs::read_link(entry.path()) else {
            continue;
        };
        if let Some(inode) = socket_inode(&target.to_string_lossy()) {
            inodes.insert(inode);
        }
    }
    Some(inodes)
}

/// `socket:[12345]` -> `12345`.
pub fn socket_inode(link_target: &str) -> Option<u64> {
    link_target
        .strip_prefix("socket:[")?
        .strip_suffix(']')?
        .parse()
        .ok()
}

/// Connected sockets in one `/proc/net` table that belong to `inodes`.
///
/// Public so `tests/headless_main_flow.rs` can hand it a table with a known
/// remote address in it: a parser that silently returns nothing would make
/// every AC-21 run green.
pub fn peers_in_table(text: &str, protocol: &str, inodes: &BTreeSet<u64>) -> Vec<Peer> {
    let mut found = Vec::new();
    for line in text.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // sl local_address rem_address st tx:rx tr:when retrnsmt uid timeout inode
        if fields.len() < 10 {
            continue;
        }
        let Ok(inode) = fields[9].parse::<u64>() else {
            continue;
        };
        if !inodes.contains(&inode) {
            continue;
        }
        let Some((address, port)) = parse_address(fields[2]) else {
            continue;
        };
        // An unspecified peer is a socket that is bound or listening, not one
        // that is talking to anybody.
        if port == 0 || address.is_unspecified() {
            continue;
        }
        found.push(Peer {
            protocol: protocol.to_owned(),
            address: address.to_string(),
            port,
        });
    }
    found
}

/// `0100007F:1F90` -> `127.0.0.1:8080`.
///
/// `/proc/net` writes each 32-bit word in host byte order, which is why the
/// bytes come back through `to_le_bytes` rather than being read left to right.
/// An IPv4-mapped IPv6 address is folded down to its IPv4 form, so that
/// `::ffff:127.0.0.1` counts as loopback — it is.
pub fn parse_address(field: &str) -> Option<(IpAddr, u16)> {
    let (address, port) = field.split_once(':')?;
    let port = u16::from_str_radix(port, 16).ok()?;

    let address = match address.len() {
        8 => IpAddr::V4(Ipv4Addr::from(
            u32::from_str_radix(address, 16).ok()?.to_le_bytes(),
        )),
        32 => {
            let mut bytes = [0u8; 16];
            for (index, chunk) in address.as_bytes().chunks(8).enumerate() {
                let word = u32::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
                bytes[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
            }
            let v6 = Ipv6Addr::from(bytes);
            match v6.to_ipv4_mapped() {
                Some(v4) => IpAddr::V4(v4),
                None => IpAddr::V6(v6),
            }
        }
        _ => return None,
    };
    Some((address, port))
}

/// Everything the watcher saw between start and stop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchReport {
    pub support: Support,
    /// How many readings were taken. Zero with `Observed` would mean the
    /// thread never ran, so the count is part of the evidence.
    pub samples: u64,
    /// Every distinct non-loopback peer seen at any point. AC-21 wants this
    /// empty.
    pub non_loopback: Vec<Peer>,
    /// Loopback peers seen. Not a finding — UI-to-core and local models live
    /// here — but a non-zero count is what shows the watcher can see sockets
    /// at all.
    pub loopback: Vec<Peer>,
}

impl WatchReport {
    pub fn is_clean(&self) -> bool {
        self.non_loopback.is_empty()
    }
}

impl fmt::Display for WatchReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.support {
            Support::Unsupported(reason) => write!(f, "sockets not observed: {reason}"),
            Support::Observed => {
                write!(
                    f,
                    "{} sample(s): {} non-loopback peer(s), {} loopback",
                    self.samples,
                    self.non_loopback.len(),
                    self.loopback.len(),
                )?;
                for peer in &self.non_loopback {
                    write!(f, "\n  non-loopback: {peer}")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Default)]
struct WatchState {
    samples: u64,
    non_loopback: BTreeSet<Peer>,
    loopback: BTreeSet<Peer>,
    support: Option<Support>,
}

/// A background thread sampling this process's sockets.
#[derive(Debug)]
pub struct Watch {
    state: Arc<Mutex<WatchState>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Watch {
    /// Start watching. One reading is taken before this returns, so a caller
    /// that starts and immediately stops still gets an answer.
    pub fn start() -> Watch {
        let state = Arc::new(Mutex::new(WatchState::default()));
        let stop = Arc::new(AtomicBool::new(false));
        record(&state, observe());

        let thread = {
            let state = Arc::clone(&state);
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    std::thread::sleep(SAMPLE_INTERVAL);
                    record(&state, observe());
                }
            })
        };

        Watch {
            state,
            stop,
            thread: Some(thread),
        }
    }

    /// Stop, take one last reading, and report everything that was seen.
    pub fn stop(mut self) -> WatchReport {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        record(&self.state, observe());

        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        WatchReport {
            support: state
                .support
                .clone()
                .unwrap_or_else(|| Support::Unsupported("no reading was taken".to_owned())),
            samples: state.samples,
            non_loopback: state.non_loopback.iter().cloned().collect(),
            loopback: state.loopback.iter().cloned().collect(),
        }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn record(state: &Arc<Mutex<WatchState>>, observation: Observation) {
    let mut state = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.samples += 1;
    state.non_loopback.extend(observation.non_loopback);
    state.loopback.extend(observation.loopback);
    // Once a reading succeeded, a later failure does not turn the run into an
    // unobserved one; the peers already seen are still evidence.
    if !matches!(state.support, Some(Support::Observed)) {
        state.support = Some(observation.support);
    }
}
