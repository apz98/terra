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

1. **Format before checking.** Run `cargo fmt` after writing code, then the gates
   (`cargo fmt --check`, clippy, tests). Don't hand-wrap lines and hope.
2. **Grep for stale references when finishing a task.** Search docs for the task ID and
   anything renamed or moved (`grep -rn "T-0004" docs/ README.md CLAUDE.md`). Plans written
   early often name the wrong task or an old name.
3. **`ls` output may be swallowed in this shell** (aliased). Use `command ls -A` or the
   dedicated file tools.
4. **Verify, don't assume.** Look up crate versions on crates.io and toolchain versions
   with `rustup check` at task time; confirm a command's output before reporting it.
5. **Things outside the AC need a callout.** If you make a choice the task didn't ask
   for, say so in the report and the commit message, and say how to undo it.

## Log (newest first)

### 2026-10-22 — T-0003 Workspace scaffold
- **What happened:** `cargo fmt --check` failed on a long `const` line I wrote by hand.
  **Cause:** ran the check before `cargo fmt`. **Rule:** 1.
- **What happened:** `docs/VERSIONING.md` said `xtask bump` is built in T-0003; it's T-0004.
  **Cause:** the baseline docs were written before the task list was final, and nobody
  cross-checked. **Rule:** 2. (Candidate for automation in `check-docs`: task IDs
  mentioned in docs must exist in TASKS.md or the archive.)
- **What happened:** a plain `ls -la` printed nothing. **Cause:** shell alias. **Rule:** 3.
- **What happened:** I set dev `opt-level = 1` for Terra's own code, which the task didn't
  specify. Fine, but it needed flagging. **Rule:** 5.
- **Note:** pinning an exact toolchain (`1.99.1`) makes rustup auto-download it on the
  first build in a fresh checkout or CI. That's expected, not an error.
