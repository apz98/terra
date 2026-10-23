//! Internal automation for the Terra workspace, run as `cargo xtask <command>`.
//!
//! - `bump`: version bumps with the cascade from `docs/VERSIONING.md`.
//! - `check-docs`: fails when the docs disagree with the code.
//! - `ci`: every quality gate in one command (`docs/WORKFLOW.md`).

mod bump;
mod check_docs;
mod ci;
mod workspace;

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, ensure};

const USAGE: &str = "\
usage: cargo xtask <command>

commands:
  bump <crate> <major|minor|patch> [<crate> <level> ...] [--dry-run]
      Bump crate versions, cascade to the crates that ship them, update internal
      version requirements and Cargo.lock, and print the `Bumps:` line for the
      commit message. Name every crate a commit changes in one call so the
      highest level wins (docs/VERSIONING.md).
  check-docs
      Fail if the docs disagree with the code: crate tables, CHANGELOG heading,
      STATUS game version, one task in progress, unknown task IDs.
  ci
      Run every gate: fmt, clippy -D warnings, tests, rustdoc -D warnings, check-docs.";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some((command, rest)) = args.split_first() else {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    };
    let result = match command.as_str() {
        "help" | "-h" | "--help" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        "bump" => workspace_root().and_then(|root| bump::run(&root, rest)),
        "check-docs" => no_args(rest)
            .and_then(|()| workspace_root())
            .and_then(|root| check_docs::run(&root)),
        "ci" => no_args(rest)
            .and_then(|()| workspace_root())
            .and_then(|root| ci::run(&root)),
        other => {
            eprintln!("unknown command `{other}`\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn no_args(rest: &[String]) -> Result<()> {
    ensure!(rest.is_empty(), "unexpected arguments: {rest:?}");
    Ok(())
}

/// The workspace root. This crate lives at `<root>/crates/terra-xtask`.
fn workspace_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .context("terra-xtask is not at <root>/crates/terra-xtask")
}

/// The `cargo` that invoked us (set by `cargo run`), so the pinned toolchain is used.
fn cargo() -> OsString {
    env::var_os("CARGO").unwrap_or_else(|| "cargo".into())
}
