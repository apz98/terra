//! `cargo xtask ci`: every quality gate from CLAUDE.md, in order, stopping at the first failure.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, ensure};

use crate::check_docs;

/// One `cargo` invocation.
struct Step {
    args: &'static [&'static str],
    env: &'static [(&'static str, &'static str)],
}

const STEPS: &[Step] = &[
    Step {
        args: &["fmt", "--all", "--check"],
        env: &[],
    },
    Step {
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
    Step {
        args: &["test", "--workspace"],
        env: &[],
    },
    Step {
        args: &["doc", "--workspace", "--no-deps"],
        env: &[("RUSTDOCFLAGS", "-D warnings")],
    },
];

/// Runs `cargo xtask ci`.
pub(crate) fn run(root: &Path) -> Result<()> {
    for step in STEPS {
        let command = format!("cargo {}", step.args.join(" "));
        println!("==> {command}");
        let status = Command::new(crate::cargo())
            .args(step.args)
            .envs(step.env.iter().copied())
            .current_dir(root)
            .status()
            .with_context(|| format!("running `{command}`"))?;
        ensure!(status.success(), "ci: `{command}` failed");
    }
    println!("==> check-docs");
    check_docs::run(root)?;
    println!("ci: all gates green");
    Ok(())
}
