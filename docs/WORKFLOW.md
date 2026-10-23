# Workflow — how Claude builds Terra, step by step

This is the loop for every task. `/next` enters it; nothing else should.

## The task loop

1. **Pick.** First unblocked `todo` task in `docs/TASKS.md` (or the one Adrian named).
   Mark it `in progress` in TASKS.md and STATUS.md.
2. **Understand.** Re-read the task's acceptance criteria and the relevant
   `ARCHITECTURE.md` section and ADRs. If something is a matter of taste or scope
   → interview Adrian (recommended option first, with confidence). If it is a
   significant technical choice → draft an ADR before coding.
3. **Check dependencies.** For new crates/deps, look up the current version on
   crates.io (don't rely on memory), check the license (MIT/Apache/BSD/Zlib/CC0
   are fine; avoid GPL in shipped crates) and maintenance status.
4. **Implement** in small steps. Tests alongside code. Hot path? Add/extend a
   criterion benchmark and record numbers in `PERFORMANCE.md`.
5. **Verify.** `cargo xtask ci` (fmt, clippy `-D warnings`, tests, rustdoc
   `-D warnings`, `check-docs`; stops at the first failure). For anything visual
   or interactive, run the game and describe what you saw. If you can't verify something, say so; never claim it works untested.
6. **Sync docs** using the matrix below. Rustdoc for every public item you added.
7. **Version.** Determine the level per changed crate (`VERSIONING.md`), then run
   **one** `cargo xtask bump <crate> <level> [<crate> <level> …]` naming every changed
   crate (try `--dry-run` first; `--allow-1.0` only when Adrian decides on 1.0). Copy its `Bumps:` line into the commit message and
   update `CHANGELOG.md`, the crate tables and STATUS; `check-docs` catches misses.
8. **Retro.** Check the review triggers below and record any review now due in
   STATUS.md. Did anything fail, surprise you, or get corrected by Adrian during this
   task? Log it in `docs/LESSONS.md` (what happened, cause, rule) and promote it to a
   rule, a doc change, or an automated check. Goes in the same commit.
9. **Commit** (Conventional Commit, trailer included), **tag** if the game version
   changed (`v<terra version>`), **push** with tags, and **check CI**: GitHub Actions
   (`.github/workflows/ci.yml`) runs `cargo xtask ci` on every push and PR to `main`.
   Wait for it with `gh run watch $(gh run list -L 1 --json databaseId -q '.[0].databaseId') --exit-status`.
   Red CI is the next task, ahead of everything else.
10. **Close out.** Mark the task `done` in TASKS.md, update STATUS.md (last task,
   next task, versions, review counters). If the milestone is complete, run the
   milestone wrap-up below.
11. **Report** to Adrian: what changed, how it was verified, the new version, lessons
    added, any review reminder, and the recommended next task with a confidence score.

## Definition of done

- [ ] Acceptance criteria in TASKS.md all met and verified
- [ ] fmt, clippy (`-D warnings`), tests, `check-docs` green locally
- [ ] Public API documented; architecture/ADRs updated if structure changed
- [ ] Versions bumped and CHANGELOG entry written
- [ ] STATUS.md and TASKS.md updated
- [ ] Retro done: new lessons (if any) in `LESSONS.md`
- [ ] Committed, pushed, CI green

## Doc-sync matrix

| If you change… | …update |
|---|---|
| Add/remove/rename a crate | `ARCHITECTURE.md` crate table, root `Cargo.toml`, `README.md` |
| Any crate version | `CHANGELOG.md`, `ARCHITECTURE.md` crate table, `STATUS.md` (game version) |
| Threading, data flow, storage layout, protocol | `ARCHITECTURE.md` (+ ADR if it's a decision) |
| A significant technical choice | New ADR in `docs/adr/`, linked from `adr/README.md` |
| Task finished / added / reordered | `TASKS.md`, `STATUS.md` |
| Milestone scope or order | `ROADMAP.md` (+ interview Adrian if it changes priorities) |
| Performance numbers or budgets | `PERFORMANCE.md` |
| Build steps, system deps, controls, how to play | `README.md` |
| An answer from Adrian | `INTERVIEW.md`, `OPEN_QUESTIONS.md`, and whatever doc it affects |
| A new uncertainty | `OPEN_QUESTIONS.md` |
| A mistake, surprise, or correction | `LESSONS.md` (log + rule; automate if possible) |

## Review triggers (remind Adrian; don't run them yourself)

- **`/code-review`**: end of every milestone; after any task touching >~500 lines
  or core data structures (chunk storage, meshing, tick loop, protocol); before
  the Friends Release.
- **`/security-review`**: before merging/shipping anything that touches
  networking, authentication, save-file parsing, mod/data-pack loading, or any
  build given to friends.

Record due/done reviews in STATUS.md "Reviews". When Adrian runs one, turn the
findings into tasks (in TASKS.md, ahead of new feature work if severe).

## Milestone wrap-up

1. All tasks of the milestone `done`; exit criteria in ROADMAP.md verified.
2. Record the game version that completed it in ROADMAP.md and PERFORMANCE.md (perf snapshot).
3. Move the milestone's tasks from TASKS.md to `docs/tasks/archive/M<n>.md`.
4. Expand the next milestone into detailed tasks in TASKS.md; sketch the one after.
5. Remind Adrian: `/code-review` (and `/security-review` if triggers apply).
6. Ask Adrian to playtest when there is something playable, and interview them
   about what they'd like next if the roadmap leaves room.

## Commit message format

```
<type>(<scope>): <summary in imperative, ≤72 chars>

<why, what, notable details>
Bumps: terra-core 0.2.0 → 0.3.0, terra 0.4.1 → 0.5.0   (omit if none)
Task: T-0105

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Types: `feat`, `fix`, `perf`, `refactor`, `test`, `docs`, `build`, `ci`, `chore`.
Scope is the crate without the `terra-` prefix (`core`, `render`, …), `workspace`, or `docs`.
