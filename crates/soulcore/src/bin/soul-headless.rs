//! Soul with no desktop attached.
//!
//! Four jobs, and the exit code is the whole interface for all of them:
//!
//! * `soul-headless` (no arguments) runs the AC-21 main flow — the entire
//!   product against a scratch store, with this process's sockets watched from
//!   start to finish — and exits 0 only if every step proved what it claims.
//!   `scripts/install-smoke.ps1` runs exactly this after an installer has put
//!   Soul on a Windows machine, which is why the report goes to stdout as JSON
//!   and the readable summary goes to stderr: a caller can pipe one and read
//!   the other.
//! * `soul-headless config` prints the shipped defaults, after checking that
//!   they really are all off. Exiting 0 with a capability switched on would
//!   turn AC-02 into a formality, so the check is here and not only in a test.
//! * `soul-headless collect-probe` is the one command that needs a person: it
//!   measures AC-09 and AC-10 against the desktop the operator is switching
//!   between, which no runner can do. It collects only because the caller
//!   passed `--i-consent`, into a scratch store it then deletes. See
//!   [`soulcore::collect_probe`] and `scripts/author-manual-checklist.md`.
//! * `soul-headless e1-watch` is the smoke's complement: the same socket
//!   observation, taken over the one path that builds an HTTP client, against
//!   an endpoint this process is itself running on loopback. See
//!   [`soulcore::e1_watch`].
//!
//! The two instruments are deliberately separate commands rather than steps of
//! the smoke. `smoke` is AC-21, and AC-21 is *the default configuration*: a run
//! that configured an endpoint in order to have something to watch would be
//! measuring a machine nobody ships.

use std::time::Duration;

use soulcore::{collect_probe, e1_watch, headless, Config};

const USAGE: &str = "\
soul-headless — Soul without a desktop

USAGE:
    soul-headless [COMMAND]

COMMANDS:
    smoke     Run the main flow against a scratch store and report what it
              found, including this process's non-loopback connections.
              The default when no command is given.
    config    Print the shipped configuration defaults.

    collect-probe --i-consent [--seconds N]
              Windows, and a person at the keyboard. Two timed phases, one
              with foreground collection off and one with it on, to check
              AC-09 and AC-10 against a real desktop. Collects application
              names into a temporary store for the duration and deletes it
              afterwards; --i-consent is how you say that is what you want.

    e1-watch  Drive the one path that opens a socket — save an endpoint,
              approve a draft, ask for a people summary, replay the approval —
              against an endpoint this process runs on 127.0.0.1, with the
              sockets watched throughout. Reports every peer it saw. Uses a
              temporary store and deletes it afterwards.
";

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("smoke") => smoke(),
        Some("config") => print_config(),
        Some("collect-probe") => probe(&args[1..]),
        Some("e1-watch") => watch_the_endpoint_path(&args[1..]),
        Some("--help") | Some("-h") => {
            print!("{USAGE}");
            std::process::ExitCode::SUCCESS
        }
        Some(other) => {
            eprint!("{USAGE}");
            eprintln!("soul-headless: unknown command `{other}`");
            std::process::ExitCode::FAILURE
        }
    }
}

fn smoke() -> std::process::ExitCode {
    let report = match headless::run() {
        Ok(report) => report,
        Err(error) => {
            eprintln!("soul-headless: the main flow failed at {error}");
            return std::process::ExitCode::FAILURE;
        }
    };

    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("soul-headless: could not serialize the report: {error}");
            return std::process::ExitCode::FAILURE;
        }
    }

    for step in &report.steps {
        eprintln!("  {:<9} {}", step.name, step.detail);
    }
    eprintln!(
        "  {:<9} {} non-loopback connection(s) over {} sample(s){}",
        "egress",
        report.egress.non_loopback_connections,
        report.egress.samples,
        match &report.egress.observation_note {
            Some(note) => format!("; sockets not observed here: {note}"),
            None => String::new(),
        },
    );
    std::process::ExitCode::SUCCESS
}

/// The manual probe. Refuses without `--i-consent`, because it is the one
/// command in this binary that samples what the operator is doing.
fn probe(args: &[String]) -> std::process::ExitCode {
    let mut consented = false;
    let mut phase = collect_probe::DEFAULT_PHASE;
    let mut rest = args.iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "--i-consent" => consented = true,
            "--seconds" => match rest.next().and_then(|value| value.parse::<u64>().ok()) {
                Some(seconds) if seconds > 0 => phase = Duration::from_secs(seconds),
                _ => {
                    eprintln!("soul-headless: --seconds wants a positive whole number");
                    return std::process::ExitCode::FAILURE;
                }
            },
            other => {
                eprint!("{USAGE}");
                eprintln!("soul-headless: collect-probe does not take `{other}`");
                return std::process::ExitCode::FAILURE;
            }
        }
    }

    if !consented {
        eprintln!(
            "soul-headless: collect-probe samples which application is in the \
             foreground. Pass --i-consent to say that is what you want.",
        );
        return std::process::ExitCode::FAILURE;
    }

    let report = match collect_probe::run(phase) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("soul-headless: the collection probe failed at {error}");
            return std::process::ExitCode::FAILURE;
        }
    };

    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("soul-headless: could not serialize the report: {error}");
            return std::process::ExitCode::FAILURE;
        }
    }

    for phase in &report.phases {
        eprintln!("  {:<12} {}", phase.name, phase.verdict);
    }
    eprintln!(
        "  {:<12} {} non-loopback connection(s) over {} sample(s)",
        "egress", report.egress.non_loopback_connections, report.egress.samples,
    );
    std::process::ExitCode::SUCCESS
}

/// The endpoint path, watched. Takes no arguments: everything it needs it
/// starts itself, and there is nothing here for a caller to point somewhere
/// else — an instrument that accepted a URL would be a way to make Soul open a
/// socket to an address of somebody else's choosing.
fn watch_the_endpoint_path(args: &[String]) -> std::process::ExitCode {
    if let Some(unexpected) = args.first() {
        eprint!("{USAGE}");
        eprintln!("soul-headless: e1-watch does not take `{unexpected}`");
        return std::process::ExitCode::FAILURE;
    }

    let report = match e1_watch::run() {
        Ok(report) => report,
        Err(error) => {
            eprintln!("soul-headless: the endpoint watch failed at {error}");
            return std::process::ExitCode::FAILURE;
        }
    };

    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("soul-headless: could not serialize the report: {error}");
            return std::process::ExitCode::FAILURE;
        }
    }

    for step in &report.steps {
        eprintln!("  {:<9} {}", step.name, step.detail);
    }
    eprintln!(
        "  {:<9} {} request(s) reached 127.0.0.1:{}",
        "endpoint",
        report.endpoint.requests.len(),
        report.endpoint.port,
    );
    eprintln!(
        "  {:<9} {} non-loopback and {} unexplained loopback peer(s) over {} sample(s){}",
        "sockets",
        report.sockets.non_loopback_connections,
        report.sockets.unexpected_loopback_peers.len(),
        report.sockets.samples,
        match &report.sockets.observation_note {
            Some(note) => format!("; sockets not observed here: {note}"),
            None => String::new(),
        },
    );
    std::process::ExitCode::SUCCESS
}

fn print_config() -> std::process::ExitCode {
    let config = Config::default();

    let open = config.open_capabilities();
    if !open.is_empty() {
        eprintln!(
            "soul-headless: refusing to start; these are on by default and must not be: {}",
            open.join(", "),
        );
        return std::process::ExitCode::FAILURE;
    }

    match serde_json::to_string_pretty(&config) {
        Ok(json) => {
            println!("{json}");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("soul-headless: could not serialize the configuration: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
