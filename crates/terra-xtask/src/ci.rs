//! `cargo xtask ci`: every quality gate from CLAUDE.md, stopping at the first failure.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, ensure};

use crate::check_docs;

/// One gate.
enum Gate {
    /// In-process and fast, so it runs first: doc drift shows up in milliseconds, not
    /// after a full clippy and test build.
    CheckDocs,
    /// A `cargo` invocation with extra environment variables.
    Cargo {
        args: &'static [&'static str],
        env: &'static [(&'static str, &'static str)],
    },
}

/// The gates, cheapest first.
const GATES: &[Gate] = &[
    Gate::CheckDocs,
    Gate::Cargo {
        args: &["fmt", "--all", "--check"],
        env: &[],
    },
    Gate::Cargo {
        args: &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        env: &[],
    },
    Gate::Cargo {
        args: &["test", "--workspace"],
        env: &[],
    },
    Gate::Cargo {
        args: &["doc", "--workspace", "--no-deps"],
        env: &[("RUSTDOCFLAGS", "-D warnings")],
    },
];

/// Runs `cargo xtask ci`.
pub(crate) fn run(root: &Path) -> Result<()> {
    for gate in GATES {
        match gate {
            Gate::CheckDocs => {
                println!("==> check-docs");
                check_docs::run(root)?;
            }
            Gate::Cargo { args, env } => {
                let command = format!("cargo {}", args.join(" "));
                println!("==> {command}");
                let status = Command::new(crate::cargo())
                    .args(*args)
                    .envs(env.iter().copied())
                    .current_dir(root)
                    .status()
                    .with_context(|| format!("running `{command}`"))?;
                ensure!(status.success(), "ci: `{command}` failed");
            }
        }
    }
    println!("ci: all gates green");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_docs_runs_first() {
        assert!(matches!(GATES.first(), Some(Gate::CheckDocs)));
    }
}
