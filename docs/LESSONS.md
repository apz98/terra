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
2. **Grep for stale references when finishing a task.** Search docs for anything renamed
   or moved, and for "once it exists"/"arrives in" notes about what you just built.
   Unknown task IDs are caught by `check-docs` automatically (since T-0004).
3. **`ls` output may be swallowed in this shell** (aliased). Use `command ls -A` or the
   dedicated file tools.
4. **Verify, don't assume.** Look up crate versions on crates.io and toolchain versions
   with `rustup check` at task time; confirm a command's output before reporting it.
5. **Things outside the AC need a callout.** If you make a choice the task didn't ask
   for, say so in the report and the commit message, and say how to undo it.
6. **Look before you overwrite.** Before writing a file with `cat >`/`Write`, check that
   it doesn't exist (`git ls-files <path>`, Read). If it does, edit it instead.
7. **Tools enforce documented invariants.** If a doc says "never X" or "X is a manual
   decision" and a tool can do X, the tool refuses without an explicit flag.

## Log (newest first)

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
