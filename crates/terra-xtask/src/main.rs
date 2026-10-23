//! Internal automation for the Terra workspace, run as `cargo xtask <command>`.
//!
//! Commands (`bump`, `check-docs`, `ci`) arrive in T-0004; see `docs/WORKFLOW.md`.

use std::process::ExitCode;

const USAGE: &str =
    "usage: cargo xtask <command>\n\nNo commands yet (bump, check-docs and ci arrive in T-0004).";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        None | Some("help" | "-h" | "--help") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown command `{other}`\n\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}
