//! `cargo xtask wait-ci [<commit>]`: waits for the CI run of a pushed commit and fails
//! unless it passes (`docs/WORKFLOW.md` step 9).
//!
//! GitHub registers a run a few seconds after the push, so "the latest run" right after
//! `git push` is usually the previous commit's (`docs/LESSONS.md`, rule 8). This looks
//! up the run of `ci.yml` for one exact commit instead, and waits for it to appear.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};

/// The workflow file that runs `cargo xtask ci` (`.github/workflows/ci.yml`).
const WORKFLOW: &str = "ci.yml";

/// How long to wait, and how often to ask.
#[derive(Clone, Copy)]
struct Limits {
    /// For the run to show up after the push.
    appear: Duration,
    /// For the run to finish, counted from the start (the job's own timeout is 30 min).
    finish: Duration,
    /// Between two `gh` calls.
    poll: Duration,
}

const LIMITS: Limits = Limits {
    appear: Duration::from_secs(120),
    finish: Duration::from_mins(45),
    poll: Duration::from_secs(5),
};

/// One workflow run, as `gh run list` reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Run {
    id: u64,
    /// `queued`, `in_progress`, `completed`, …
    status: String,
    /// Empty until `completed`, then `success`, `failure`, `cancelled`, …
    conclusion: String,
    url: String,
}

/// What the wait loop needs from outside, so tests can fake GitHub and the clock.
trait Env {
    /// The newest `ci.yml` run for commit `sha`, if GitHub has one yet.
    fn latest_run(&mut self, sha: &str) -> Result<Option<Run>>;
    /// Time since the wait started.
    fn elapsed(&self) -> Duration;
    fn sleep(&mut self, duration: Duration);
}

/// The real thing: the `gh` CLI and the wall clock.
struct Gh {
    root: PathBuf,
    start: Instant,
}

impl Env for Gh {
    fn latest_run(&mut self, sha: &str) -> Result<Option<Run>> {
        let output = Command::new("gh")
            .args(["run", "list", "--workflow", WORKFLOW, "--commit", sha])
            .args(["--limit", "1", "--json", "databaseId,status,conclusion,url"])
            .args([
                "--jq",
                ".[] | [.databaseId, .status, .conclusion, .url] | @tsv",
            ])
            .current_dir(&self.root)
            .output()
            .context("running `gh run list` (is the GitHub CLI installed?)")?;
        ensure!(
            output.status.success(),
            "`gh run list` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        parse_run(&String::from_utf8_lossy(&output.stdout))
    }

    fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    fn sleep(&mut self, duration: Duration) {
        thread::sleep(duration);
    }
}

/// Parses the `@tsv` line printed by `gh run list` (no line: no run yet).
fn parse_run(output: &str) -> Result<Option<Run>> {
    let Some(line) = output.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let fields: Vec<&str> = line.split('\t').collect();
    let [id, status, conclusion, url] = fields.as_slice() else {
        bail!("unexpected `gh run list` output: {line:?}");
    };
    Ok(Some(Run {
        id: id
            .parse()
            .with_context(|| format!("unexpected run id in {line:?}"))?,
        status: (*status).to_owned(),
        conclusion: (*conclusion).to_owned(),
        url: (*url).to_owned(),
    }))
}

/// Polls until the run for `sha` completes. `Ok` only if it succeeded; an error if it
/// failed, was cancelled, never appeared, or didn't finish in time.
fn wait(env: &mut impl Env, sha: &str, limits: Limits) -> Result<Run> {
    let short = &sha[..sha.len().min(10)];
    let mut last_status = String::new();
    loop {
        let elapsed = env.elapsed();
        match env.latest_run(sha)? {
            None => ensure!(
                elapsed < limits.appear,
                "no `{WORKFLOW}` run for {short} after {}s. Was it pushed to a branch CI \
                 runs on (`main`, or a PR to it)?",
                limits.appear.as_secs()
            ),
            Some(run) if run.status == "completed" => {
                ensure!(
                    run.conclusion == "success",
                    "CI {} for {short}: {}\n  logs: gh run view {} --log-failed",
                    if run.conclusion.is_empty() {
                        "ended"
                    } else {
                        &run.conclusion
                    },
                    run.url,
                    run.id
                );
                return Ok(run);
            }
            Some(run) => {
                if run.status != last_status {
                    println!("wait-ci: {short} {} ({})", run.status, run.url);
                    last_status.clone_from(&run.status);
                }
                ensure!(
                    elapsed < limits.finish,
                    "CI for {short} still {} after {} min: {}",
                    run.status,
                    limits.finish.as_secs() / 60,
                    run.url
                );
            }
        }
        env.sleep(limits.poll);
    }
}

/// The full SHA of `commit` (any revision `git` understands; default `HEAD`).
fn resolve(root: &Path, commit: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "--end-of-options"])
        .arg(format!("{commit}^{{commit}}"))
        .current_dir(root)
        .output()
        .context("running `git rev-parse`")?;
    ensure!(output.status.success(), "`{commit}` is not a commit");
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Runs `cargo xtask wait-ci [<commit>]`.
pub(crate) fn run(root: &Path, args: &[String]) -> Result<()> {
    let commit = match args {
        [] => "HEAD",
        [commit] if !commit.starts_with('-') => commit.as_str(),
        _ => bail!("expected at most one commit, got {args:?}"),
    };
    let sha = resolve(root, commit)?;
    let mut gh = Gh {
        root: root.to_path_buf(),
        start: Instant::now(),
    };
    let run = wait(&mut gh, &sha, LIMITS)?;
    println!("wait-ci: CI passed for {} ({})", &sha[..10], run.url);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    /// Answers `gh` calls from a script and keeps a fake clock that only `sleep` moves.
    struct Fake {
        answers: VecDeque<Result<Option<Run>>>,
        now: Duration,
        calls: usize,
    }

    impl Fake {
        fn new(answers: impl IntoIterator<Item = Result<Option<Run>>>) -> Self {
            Self {
                answers: answers.into_iter().collect(),
                now: Duration::ZERO,
                calls: 0,
            }
        }
    }

    impl Env for Fake {
        fn latest_run(&mut self, sha: &str) -> Result<Option<Run>> {
            assert_eq!(sha, SHA);
            self.calls += 1;
            // Past the end of the script, the last answer repeats.
            match self.answers.len() {
                0 => panic!("no scripted answer"),
                1 => match &self.answers[0] {
                    Ok(run) => Ok(run.clone()),
                    Err(error) => bail!("{error}"),
                },
                _ => self.answers.pop_front().unwrap(),
            }
        }

        fn elapsed(&self) -> Duration {
            self.now
        }

        fn sleep(&mut self, duration: Duration) {
            self.now += duration;
        }
    }

    const TEST_LIMITS: Limits = Limits {
        appear: Duration::from_secs(20),
        finish: Duration::from_secs(60),
        poll: Duration::from_secs(5),
    };

    /// `gh` reporting run 7 with this status and conclusion.
    #[expect(
        clippy::unnecessary_wraps,
        reason = "reads as one scripted `gh` answer"
    )]
    fn run(status: &str, conclusion: &str) -> Result<Option<Run>> {
        Ok(Some(Run {
            id: 7,
            status: status.to_owned(),
            conclusion: conclusion.to_owned(),
            url: "https://github.com/o/r/actions/runs/7".to_owned(),
        }))
    }

    fn wait_for(fake: &mut Fake) -> Result<Run> {
        wait(fake, SHA, TEST_LIMITS)
    }

    #[test]
    fn waits_for_the_run_to_appear_and_finish() {
        let mut fake = Fake::new([
            Ok(None),
            Ok(None),
            run("queued", ""),
            run("in_progress", ""),
            run("in_progress", ""),
            run("completed", "success"),
        ]);
        let result = wait_for(&mut fake).unwrap();
        assert_eq!(result.id, 7);
        assert_eq!(fake.calls, 6);
        assert_eq!(fake.now, Duration::from_secs(25));
    }

    #[test]
    fn a_failed_or_cancelled_run_is_an_error() {
        for conclusion in ["failure", "cancelled", "timed_out", ""] {
            let mut fake = Fake::new([run("in_progress", ""), run("completed", conclusion)]);
            let err = wait_for(&mut fake).unwrap_err().to_string();
            assert!(err.contains("actions/runs/7"), "{err}");
            assert!(err.contains("gh run view 7 --log-failed"), "{err}");
            if !conclusion.is_empty() {
                assert!(err.contains(conclusion), "{err}");
            }
        }
    }

    #[test]
    fn no_run_is_an_error_after_the_appear_limit() {
        let mut fake = Fake::new([Ok(None)]);
        let err = wait_for(&mut fake).unwrap_err().to_string();
        assert!(err.contains("no `ci.yml` run for 0123456789"), "{err}");
        // Asked at 0, 5, 10, 15 and 20 s; gave up at the 20 s limit.
        assert_eq!(fake.calls, 5);
    }

    #[test]
    fn a_run_that_never_finishes_times_out() {
        let mut fake = Fake::new([run("in_progress", "")]);
        let err = wait_for(&mut fake).unwrap_err().to_string();
        assert!(err.contains("still in_progress after 1 min"), "{err}");
        assert_eq!(fake.now, TEST_LIMITS.finish);
    }

    #[test]
    fn gh_errors_stop_the_wait() {
        let mut fake = Fake::new([Ok(None), Err(anyhow::anyhow!("gh: not logged in"))]);
        let err = wait_for(&mut fake).unwrap_err().to_string();
        assert!(err.contains("not logged in"), "{err}");
        assert_eq!(fake.calls, 2);
    }

    #[test]
    fn parses_gh_tsv_output() {
        assert_eq!(parse_run("").unwrap(), None);
        assert_eq!(parse_run("\n").unwrap(), None);
        assert_eq!(
            parse_run("39883425366\tcompleted\tsuccess\thttps://x/runs/39883425366\n").unwrap(),
            Some(Run {
                id: 39_883_425_366,
                status: "completed".to_owned(),
                conclusion: "success".to_owned(),
                url: "https://x/runs/39883425366".to_owned(),
            })
        );
        assert_eq!(
            parse_run("1\tqueued\t\thttps://x/runs/1")
                .unwrap()
                .unwrap()
                .conclusion,
            ""
        );
        assert!(parse_run("1\tqueued").is_err());
        assert!(parse_run("abc\tqueued\t\turl").is_err());
    }
}
