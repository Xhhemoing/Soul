//! Headless entry point for CI hosts that have no desktop.
//!
//! Prints the default configuration as JSON and exits 0, after checking that
//! the defaults really are all-off. Exiting 0 with an open capability would
//! turn AC-02 into a formality, so the check is here rather than only in the
//! test.

use soulcore::Config;

fn main() -> std::process::ExitCode {
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
