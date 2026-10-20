//! Bounded loopback responses for defects MockLlm cannot emit.
use std::{
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub struct RawEndpoint {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    requests: Arc<AtomicUsize>,
    worker: Option<JoinHandle<()>>,
}

impl RawEndpoint {
    pub fn start(body: Option<&str>) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener");
        listener.set_nonblocking(false).unwrap();
        let address = listener.local_addr().unwrap();
        assert!(address.ip().is_loopback());
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(AtomicUsize::new(0));
        let (thread_stop, thread_requests) = (Arc::clone(&stop), Arc::clone(&requests));
        let body = body.map(str::to_owned);
        let worker = thread::spawn(move || {
            loop {
                let (mut stream, peer) = listener.accept().expect("blocking loopback accept");
                if thread_stop.load(Ordering::SeqCst) {
                    break;
                }
                assert!(peer.ip().is_loopback());
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                read_request(&mut stream);
                thread_requests.fetch_add(1, Ordering::SeqCst);
                let (status, payload) = match &body {
                    Some(body) => ("200 OK", body.as_str()),
                    // A complete failing HTTP response avoids client retry of an
                    // ambiguous connection close; it reaches real EgressError.
                    None => ("503 Service Unavailable", "synthetic transport unavailable"),
                };
                let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}", payload.len());
                stream
                    .write_all(response.as_bytes())
                    .expect("complete raw response");
                stream.flush().unwrap();
                stream.shutdown(Shutdown::Both).unwrap();
            }
        });
        Self {
            address,
            stop,
            requests,
            worker: Some(worker),
        }
    }

    pub fn base_url(&self) -> String {
        format!("http://{}", self.address)
    }

    pub fn request_count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}

impl Drop for RawEndpoint {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake a blocking accept without making a counted HTTP request.
        let wake = TcpStream::connect_timeout(&self.address, Duration::from_secs(2))
            .expect("wake fixture accept");
        drop(wake);
        if let Some(worker) = self.worker.take() {
            worker.join().expect("raw endpoint worker");
        }
    }
}

fn read_request(stream: &mut TcpStream) {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut received = Vec::new();
    let mut chunk = [0_u8; 4096];
    let (header_end, length) = loop {
        assert!(received.len() <= 128 * 1024, "bounded synthetic request");
        let count = read_chunk(stream, &mut chunk, deadline);
        assert_ne!(count, 0, "request headers complete before EOF");
        received.extend_from_slice(&chunk[..count]);
        if let Some(end) = received.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
            let headers = std::str::from_utf8(&received[..end]).unwrap();
            let length = headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("Content-Length"))
                .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                .expect("explicit Content-Length");
            assert!(length <= 64 * 1024);
            break (end + 4, length);
        }
    };
    while received.len() < header_end + length {
        let count = read_chunk(stream, &mut chunk, deadline);
        assert_ne!(count, 0, "request body complete before EOF");
        received.extend_from_slice(&chunk[..count]);
    }
    assert_eq!(received.len(), header_end + length);
}

fn read_chunk(stream: &mut TcpStream, chunk: &mut [u8], deadline: Instant) -> usize {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .expect("total request deadline");
    stream
        .set_read_timeout(Some(remaining.min(Duration::from_secs(2))))
        .unwrap();
    stream.read(chunk).expect("bounded complete request read")
}
