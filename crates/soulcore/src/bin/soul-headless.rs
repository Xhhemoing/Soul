//! Soul with no desktop attached.
//!
//! Two jobs, and the exit code is the whole interface for both:
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

use soulcore::{headless, Config};

const USAGE: &str = "\
soul-headless — Soul without a desktop

USAGE:
    soul-headless [COMMAND]

COMMANDS:
    smoke     Run the main flow against a scratch store and report what it
              found, including this process's non-loopback connections.
              The default when no command is given.
    config    Print the shipped configuration defaults.
";

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("smoke") => smoke(),
        Some("config") => print_config(),
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
