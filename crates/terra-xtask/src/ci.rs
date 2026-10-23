//! `cargo xtask ci`: every quality gate from CLAUDE.md, stopping at the first failure.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};

use crate::check_docs;

/// The cargo-deny release CI installs (`.github/workflows/ci.yml`; a test keeps the two
/// in sync). Locally, another version only gets a note.
const CARGO_DENY_VERSION: &str = "0.20.2";

/// One gate.
enum Gate {
    /// In-process and fast, so it runs first: doc drift shows up in milliseconds, not
    /// after a full clippy and test build.
    CheckDocs,
    /// `cargo deny check` (licenses, advisories, bans, sources; see `deny.toml`). It's an
    /// external tool, so a missing install gets an install hint instead of cargo's
    /// "no such command".
    Deny,
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
    Gate::Deny,
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
    // Optional features aren't built by the gate above. `profile-tracy` swaps in a
    // different `profiling` backend, so it gets its own lint pass.
    Gate::Cargo {
        args: &[
            "clippy",
            "-p",
            "terra",
            "--features",
            "profile-tracy",
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
            Gate::Deny => {
                println!("==> cargo deny check");
                cargo_deny_check(root)?;
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

/// Runs `cargo deny check` in `root`, or fails with an install hint if cargo-deny is
/// missing.
fn cargo_deny_check(root: &Path) -> Result<()> {
    let Some(version) = cargo_deny_version()? else {
        bail!(
            "ci: cargo-deny is not installed. Install the version CI uses with\n  \
             cargo install --locked cargo-deny@{CARGO_DENY_VERSION}"
        );
    };
    if version != CARGO_DENY_VERSION {
        println!(
            "note: cargo-deny {version} is installed, CI uses {CARGO_DENY_VERSION}; \
             results may differ"
        );
    }
    let status = Command::new(crate::cargo())
        .args(["deny", "check"])
        .current_dir(root)
        .status()
        .context("running `cargo deny check`")?;
    ensure!(status.success(), "ci: `cargo deny check` failed");
    Ok(())
}

/// The installed cargo-deny's version (`cargo deny --version` prints
/// `cargo-deny 0.20.2`), or `None` if cargo has no `deny` subcommand.
fn cargo_deny_version() -> Result<Option<String>> {
    let output = Command::new(crate::cargo())
        .args(["deny", "--version"])
        .output()
        .context("running `cargo deny --version`")?;
    if !output.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout
        .split_whitespace()
        .nth(1)
        .with_context(|| format!("unexpected `cargo deny --version` output: {stdout:?}"))?;
    Ok(Some(version.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_docs_runs_first() {
        assert!(matches!(GATES.first(), Some(Gate::CheckDocs)));
    }

    #[test]
    fn the_tracy_feature_is_linted() {
        let tracy = GATES.iter().any(|gate| {
            matches!(gate, Gate::Cargo { args, .. }
                if args.starts_with(&["clippy", "-p", "terra", "--features", "profile-tracy"])
                    && args.ends_with(&["-D", "warnings"]))
        });
        assert!(
            tracy,
            "no `cargo clippy -p terra --features profile-tracy` gate"
        );
    }

    #[test]
    fn cargo_deny_is_a_gate() {
        assert!(GATES.iter().any(|gate| matches!(gate, Gate::Deny)));
    }

    /// The repository root (this crate lives at `<root>/crates/terra-xtask`).
    fn repo_root() -> &'static Path {
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
    }

    #[test]
    fn ci_installs_the_pinned_cargo_deny() {
        let path = repo_root().join(".github/workflows/ci.yml");
        let workflow = std::fs::read_to_string(&path).expect("read ci.yml");
        let pin = format!("cargo-deny@{CARGO_DENY_VERSION}");
        assert!(
            workflow.contains(&pin),
            "{} doesn't install `{pin}`; keep it in sync with CARGO_DENY_VERSION",
            path.display()
        );
    }

    /// Runs `cargo deny check licenses` with the repository's `deny.toml` on a throwaway
    /// workspace whose one dependency has `license`.
    fn check_licenses_of_a_dependency(license: &str) -> std::process::Output {
        let dir = tempfile::tempdir().expect("tempdir");
        let write = |path: &str, contents: &str| {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().expect("has a parent")).expect("mkdir");
            std::fs::write(path, contents).expect("write");
        };
        write(
            "Cargo.toml",
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
             license = \"MIT\"\n\n[dependencies]\ndep = { path = \"dep\" }\n",
        );
        write("src/main.rs", "fn main() {}\n");
        write(
            "dep/Cargo.toml",
            &format!(
                "[package]\nname = \"dep\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
                 license = \"{license}\"\n"
            ),
        );
        write("dep/src/lib.rs", "");
        std::fs::copy(repo_root().join("deny.toml"), dir.path().join("deny.toml"))
            .expect("copy deny.toml");
        Command::new(crate::cargo())
            .args(["deny", "check", "licenses"])
            .current_dir(dir.path())
            .output()
            .expect("run cargo deny")
    }

    /// cargo-deny is an external tool. CI installs it (and sets `CI`), so there a missing
    /// install fails the test; locally the test is skipped with a message.
    fn cargo_deny_available() -> bool {
        if cargo_deny_version().expect("cargo runs").is_some() {
            return true;
        }
        assert!(
            std::env::var_os("CI").is_none(),
            "cargo-deny is not installed, but CI must run this test"
        );
        eprintln!("skipping: cargo-deny is not installed");
        false
    }

    #[test]
    fn deny_toml_rejects_gpl_and_accepts_mit() {
        if !cargo_deny_available() {
            return;
        }
        let gpl = check_licenses_of_a_dependency("GPL-3.0-only");
        let stderr = String::from_utf8_lossy(&gpl.stderr);
        assert!(!gpl.status.success(), "a GPL dependency passed:\n{stderr}");
        assert!(
            stderr.contains("GPL-3.0-only"),
            "unexpected output:\n{stderr}"
        );

        // The same workspace with a permissive license passes, so the failure above is
        // the license and not a broken setup.
        let mit = check_licenses_of_a_dependency("MIT");
        assert!(
            mit.status.success(),
            "an MIT dependency failed:\n{}",
            String::from_utf8_lossy(&mit.stderr)
        );
    }
}
