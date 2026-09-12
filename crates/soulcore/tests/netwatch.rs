//! The socket observation AC-21 rests on, proved against sockets that exist.
//!
//! Its own test binary, and that is the point. `netwatch` reports the sockets
//! of the *process* it runs in, so a control test that opens one would be
//! visible to `tests/headless_main_flow.rs` running beside it — the main flow
//! would see a peer it did not create and fail the very criterion this file
//! exists to make trustworthy. Cargo gives each `tests/*.rs` its own process,
//! which is the isolation that separation buys.

use std::collections::BTreeSet;
use std::net::{IpAddr, TcpListener, TcpStream, UdpSocket};

use soulcore::netwatch;

/// The control the AC-21 number depends on: with sockets open, the observer
/// reports them, and it puts each on the right side of the loopback line.
/// Without this, "zero non-loopback connections" would be just as true of an
/// observer that reads nothing at all.
///
/// Both halves are one test because the observation is process-wide: run as
/// two tests, each would see the other's sockets and the loopback assertion
/// would be a coin toss.
///
/// The far address is TEST-NET-3 (RFC 5737), which is routed nowhere, and a
/// connected UDP socket sends no packet — `connect` only fixes the peer. So
/// this test reaches no network while producing exactly the `/proc/net` row a
/// real egress would produce.
#[test]
fn the_observer_sees_an_open_socket_and_says_which_side_of_loopback_it_is_on() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let address = listener.local_addr().expect("local address");
    let client = TcpStream::connect(address).expect("connect to ourselves");
    let accepted = listener.accept().expect("accept");

    let observation = netwatch::observe();
    if !observation.support.is_observed() {
        eprintln!("skipped: {:?}", observation.support);
        return;
    }
    assert!(
        observation
            .loopback
            .iter()
            .any(|peer| peer.address == "127.0.0.1"),
        "a live loopback connection was not in {:?}",
        observation.loopback,
    );
    assert!(
        observation.non_loopback.is_empty(),
        "a connection to ourselves is not egress: {:?}",
        observation.non_loopback,
    );

    let far = UdpSocket::bind("0.0.0.0:0").expect("bind");
    if far.connect("203.0.113.10:9").is_err() {
        eprintln!("skipped the second half: no route towards a non-loopback address");
        return;
    }
    let observation = netwatch::observe();
    assert!(
        observation
            .non_loopback
            .iter()
            .any(|peer| peer.address == "203.0.113.10" && peer.port == 9),
        "a non-loopback peer was not reported: {:?}",
        observation.non_loopback,
    );

    drop(client);
    drop(accepted);
}

/// The table parser, against rows whose answers are known. This runs
/// everywhere, including on hosts where `/proc` is unavailable or where the
/// two tests above skip.
#[test]
fn the_table_parser_reports_the_peers_and_only_the_peers() {
    let table = concat!(
        "  sl  local_address rem_address   st tx_queue:rx_queue tr:tm->when retrnsmt   uid  timeout inode\n",
        "   0: 0100007F:1F90 0A7100CB:01BB 01 00000000:00000000 00:00000000 00000000  1000        0 4242 1 ffff 20 0 0 10 -1\n",
        "   1: 0100007F:1F91 0100007F:C000 01 00000000:00000000 00:00000000 00000000  1000        0 4243 1 ffff 20 0 0 10 -1\n",
        "   2: 00000000:1F92 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 4244 1 ffff 20 0 0 10 -1\n",
        "   3: 0100007F:1F93 0A7100CB:01BB 01 00000000:00000000 00:00000000 00000000  1000        0 9999 1 ffff 20 0 0 10 -1\n",
    );
    let ours: BTreeSet<u64> = [4242, 4243, 4244].into_iter().collect();
    let peers = netwatch::peers_in_table(table, "tcp", &ours);

    let described: Vec<String> = peers.iter().map(ToString::to_string).collect();
    assert_eq!(
        described,
        vec!["tcp://203.0.113.10:443", "tcp://127.0.0.1:49152"],
        "row 2 is listening and row 3 belongs to another process",
    );

    assert!(
        netwatch::peers_in_table(table, "tcp", &BTreeSet::new()).is_empty(),
        "with no inodes of our own, nothing on this machine is ours",
    );
}

#[test]
fn a_hex_address_decodes_the_way_proc_writes_it() {
    let (address, port) = netwatch::parse_address("0100007F:1F90").expect("ipv4");
    assert_eq!(address, "127.0.0.1".parse::<IpAddr>().expect("parse"));
    assert_eq!(port, 8080);
    assert!(address.is_loopback());

    let (address, _) =
        netwatch::parse_address("00000000000000000000000001000000:0050").expect("ipv6 loopback");
    assert!(address.is_loopback(), "::1 is loopback: {address}");

    // ::ffff:203.0.113.10 is an IPv4 peer reached through an IPv6 socket, and
    // it must not become loopback or vanish on the way through.
    let (address, port) =
        netwatch::parse_address("0000000000000000FFFF00000A7100CB:01BB").expect("mapped");
    assert_eq!(port, 443);
    assert!(!address.is_loopback());
    assert_eq!(address.to_string(), "203.0.113.10");

    assert!(netwatch::parse_address("nonsense").is_none());
    assert!(netwatch::parse_address("0100007F").is_none());
    assert_eq!(netwatch::socket_inode("socket:[4242]"), Some(4242));
    assert_eq!(netwatch::socket_inode("/dev/null"), None);
}
