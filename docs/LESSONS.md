# Lessons — so mistakes happen once

Adrian clears Claude's context often (`/clear`). Anything learned in a session is lost
unless it is written here. This file is how Claude gets better across sessions.

**Read the Rules at the start of every session** (CLAUDE.md and `/next` require it).

## How this file works

- **When to add a lesson:** something went wrong or surprised us: a failed gate, red CI,
  a wrong assumption, a tool quirk, docs that drifted, a correction from Adrian, or
  time wasted on something that a note would have saved. Add it **in the same commit**
  as the task it came from (or a `docs:` commit if no task is open).
- **Log entry format:** date, task, *what happened*, *root cause*, *rule*. Keep it to a
  few lines. Be honest; a lesson nobody can act on is useless.
- **Promote:** when a log entry yields a rule worth following every time, add it to the
  Rules list below. If it belongs to a process, also put it into `WORKFLOW.md`,
  `CLAUDE.md` or `/next` so it's enforced where the work happens. If it can be checked
  by a tool (`cargo xtask check-docs`, clippy, CI), automate it. That's the strongest fix.
- **Prune:** delete a rule once it's automated or no longer applies, and say so in the
  log. Keep the Rules list short (≤ ~20) so it actually gets read.

## Rules (read every session)

1. **Format before checking.** Run `cargo fmt` after writing code, then `cargo xtask ci`.
   Don't hand-wrap lines and hope.
2. **Grep for stale references when finishing a task.** Search CLAUDE.md, `.claude/commands/`
   and `docs/` for the *exact* old text of anything renamed, moved or replaced, and for "once it exists"/"arrives in" notes about what you just built.
   Unknown task IDs are caught by `check-docs` automatically (since T-0004).
3. **The shell is zsh with aliases.** `ls` output may be swallowed: use `command ls -A`
   or the file tools. A word starting with `=` is a command lookup (`echo ====` fails),
   unmatched globs are errors (quote `--include='*.md'`), and `/usr/bin/time` and `bc`
   aren't installed: time with `date +%s%N` arithmetic.
4. **Verify, don't assume.** Look up crate versions on crates.io (`curl -A terra-dev`, no
   personal data in headers) and toolchain versions with `rustup check` at task time;
   confirm a command's output before reporting it.
5. **Things outside the AC need a callout.** If you make a choice the task didn't ask
   for, say so in the report and the commit message, and say how to undo it.
6. **Look before you overwrite.** Before writing a file with `cat >`/`Write`, check that
   it doesn't exist (`git ls-files <path>`, Read). If it does, edit it instead.
7. **Tools enforce documented invariants.** If a doc says "never X" or "X is a manual
   decision" and a tool can do X, the tool refuses without an explicit flag.
8. **Wait on CI by commit SHA, not "latest run".** A run appears a few seconds after the
   push; `cargo xtask wait-ci` handles it (WORKFLOW step 9).
9. **Scripted edits stay dumb and checked.** Use the Edit tool for one-off replacements.
   In a Python/sed edit script, no clever expressions (conditionals, joins) inside
   `replace` arguments, and check the file's line count or `git diff --stat` afterwards.

## Log (newest first)

### 2026-10-22 — T-0008 M0 wrap-up
- **What happened:** three shell commands failed on zsh quirks: `echo =====` (zsh expands
  `=word` to a command path), an unquoted `--include=*.md` (unmatched glob is an error),
  and timing with `/usr/bin/time` and `bc` (neither is installed). **Cause:** wrote bash
  habits for a zsh shell. **Rule:** 3 (widened from the `ls` alias to zsh quirks).
- **What happened:** two new task descriptions told future tasks to edit ADR 0010, which
  `docs/adr/README.md` says is never rewritten once accepted. Caught by the stale-reference
  grep before committing. **Rule:** none new; an ADR changes only through a superseding ADR.

### 2026-10-22 — T-0007 Linux build prerequisites
- **What happened:** checking 16 package names on packages.ubuntu.com took more than 3
  minutes and never finished. The Docker daemon wasn't running, so a container test
  wasn't possible either (starting it needs sudo). **Fix:** grep the archive's own index,
  `http://archive.ubuntu.com/ubuntu/dists/<release>/{main,universe}/binary-amd64/Packages.xz`
  (~16 MB, seconds). **Rule:** verify Ubuntu package names against `Packages.xz`, not
  the packages website.
- **Note:** winit and wgpu `dlopen` every system library (x11-dl, x11rb `dl-libxcb`,
  xkbcommon-dl, `wayland-dlopen`, ash `libvulkan.so`, EGL `dynamic`), so building needs
  only a C toolchain and pkg-config. Audio (`cpal`/ALSA) will be the first real build-time
  system dependency. Add it to `.github/ubuntu-packages.txt` and README then.
- **Note:** Rust 1.99 already links with `rust-lld` on x86-64 Linux
  (`cargo rustc -p terra -- --print link-args` shows `-fuse-ld=lld`), so `mold` would add
  little. Measure before adding it.

### 2026-10-22 — T-0006 Diagnostics foundation
- **What happened:** I passed Adrian's email in a `User-Agent` header when querying the
  crates.io API. Instructions say his email goes to no unrelated service unless he
  asks. **Cause:** copied crates.io's "include contact info" advice without thinking.
  **Rule:** use `-A terra-dev` (no personal data) for crates.io API calls.
- **What happened:** `tracing-subscriber` writes ANSI colour codes even when stderr is a
  pipe or file. Only running the binary showed it; the tests passed. **Fix:**
  `.with_ansi(io::stderr().is_terminal())`. **Rule:** WORKFLOW step 5 already says to run
  the binary for anything user-visible. This is why.
- **Note:** global state (subscriber, panic hook) is tested in its own `tests/*.rs` file,
  since each one runs as a separate process. The Tracy backend of `profiling` panics on the
  first scope unless `tracy_client::Client::start()` ran. CI builds default features only,
  so check `cargo clippy -p terra --features profile-tracy` by hand when touching it.
- **Note:** clippy pedantic flags `name.ends_with(".log")` in tests
  (`case_sensitive_file_extension_comparisons`). Use `Path::extension()`.

### 2026-10-22 — T-0010 Fix `/code-review` findings on T-0009 and T-0005
- **What happened:** a Python edit script turned `wait_ci.rs` into two nested copies of
  itself. I had written `s.replace(a, X + Y if False else s.replace(a, Z))`. `if/else`
  binds looser than `+`, so the replacement text was the whole edited file. Rustc caught
  it (`//!` in the middle of a file) and I recovered the inner copy. **Cause:** a
  half-rewritten one-liner left in the script. **Rule:** 9.
- **Note:** clippy (1.99) wants `Duration::from_mins(45)` over `from_secs(45 * 60)`, and
  flags test helpers that always return `Ok` (`unnecessary_wraps`). Use `#[expect(…,
  reason = …)]` when the `Result` is the point.

### 2026-10-22 — T-0005 CI on GitHub Actions
- **What happened:** right after `git push`, `gh run list -L 1` returned no run, so
  `gh run watch` got an empty ID and failed. The WORKFLOW command I had just written had
  the same race. **Cause:** GitHub registers the run a few seconds after the push.
  **Rule:** 8 (WORKFLOW step 9 now polls by commit SHA).
- **What happened:** `/code-review` found CLAUDE.md rule 7 still saying `gh run list -L 1`
  after I changed WORKFLOW. My stale-reference grep looked for "no CI" wording, not the
  old command itself. **Cause:** I grepped for the topic, not the exact text I replaced.
  **Rule:** 2 (now says: grep for the exact old command/text in CLAUDE.md,
  `.claude/commands/` and `docs/`).
- **What happened:** the same review found `cancel-in-progress` cancelling `main` runs, and
  that the fix for the race was a pasted shell loop, not a tool. **Cause:** I copied a
  common PR-oriented workflow snippet without thinking about split commits on `main`.
  **Fix:** T-0010 (`cargo xtask wait-ci`).
- **Note:** first green run took 45 s with a cold cache. GitHub says `ubuntu-latest`
  moves to Ubuntu 26 from 2026-10-19; T-0007 should check package names there.

### 2026-10-22 — T-0009 Fix `/code-review` findings on T-0004
- **What happened:** a Python search-and-replace of `fixture.read(` missed three calls,
  because `cargo fmt` had already split them as `fixture\n.read(`. **Cause:** a textual
  edit after formatting. **Rule:** for mechanical renames across formatted Rust, match
  with a whitespace-tolerant regex (`fixture\s*\.read\(`), then run `cargo fmt` again.
- **Note:** tests now build throwaway workspaces with `tempfile` and run the real
  `cargo metadata`/`cargo update` (offline-safe: only path dependencies). That's about 15
  extra process spawns, and the whole xtask suite still runs in ~0.05 s.

### 2026-10-22 — T-0004 xtask `bump`, `check-docs`, `ci`
- **What happened:** I created `clippy.toml` with a heredoc and overwrote an existing
  one, dropping `allow-expect-in-tests` and `allow-dbg-in-tests`. `git status` showed it
  as `M` rather than `??`, and it was restored before committing. **Cause:** assumed the
  file didn't exist because T-0003's notes didn't mention it. **Rule:** 6.
- **Automated:** the 2026-10-22 T-0003 entry's task-ID drift is now checked by
  `check-docs` (IDs outside fenced code blocks must be defined in TASKS.md or the
  archive). Rule 2 was narrowed to match.
- **What happened:** the `/code-review` reminder went into STATUS in a follow-up commit
  rather than the task commit. **Cause:** I checked the review triggers after committing.
  **Rule:** check the WORKFLOW review triggers in the retro step, before committing.
- **What happened:** `/code-review` found 10 issues (T-0009). The worst, `bump major`
  silently making 0.x → 1.0.0, was a guard I had thought of and dropped as
  over-engineering. **Cause:** I treated a documented invariant ("1.0.0 is a manual
  decision") as something the user would remember. **Rule:** 7.
- **Note:** clippy pedantic rejects `format!` collected into a `String` (`format_collect`)
  and `push_str(&format!(…))` (`format_push_string`). Use `writeln!(s, …)` with
  `std::fmt::Write`.

### 2026-10-22 — T-0003 Workspace scaffold
- **What happened:** `cargo fmt --check` failed on a long `const` line I wrote by hand.
  **Cause:** ran the check before `cargo fmt`. **Rule:** 1.
- **What happened:** `docs/VERSIONING.md` said `xtask bump` is built in T-0003; it's T-0004.
  **Cause:** the baseline docs were written before the task list was final, and nobody
  cross-checked. **Rule:** 2. (Automated in T-0004: `check-docs` verifies task IDs.)
- **What happened:** a plain `ls -la` printed nothing. **Cause:** shell alias. **Rule:** 3.
- **What happened:** I set dev `opt-level = 1` for Terra's own code, which the task didn't
  specify. Fine, but it needed flagging. **Rule:** 5.
- **Note:** pinning an exact toolchain (`1.99.1`) makes rustup auto-download it on the
  first build in a fresh checkout or CI. That's expected, not an error.
