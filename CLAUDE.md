# CLAUDE.md — Operating manual for building Terra

Terra is a Minecraft-style voxel sandbox written in Rust (crates `terra`, `terra-*`).
Adrian is the **solo owner**; **Claude does the engineering**: planning, code, docs,
versioning, commits, pushes. Adrian asks "what's next?" (`/next`) and playtests.

Treat this as a serious, long-lived project. Work **in order**, keep quality high,
and never let docs drift from code.

## Start of every session (or after `/clear`)

1. Read the **Rules in `docs/LESSONS.md`** (mistakes we've already made once).
2. Read `docs/STATUS.md` (where we are), then `docs/TASKS.md` (what's next).
3. Skim `docs/OPEN_QUESTIONS.md` for anything blocking the next task.
4. `git status` + `git log --oneline -10` to confirm the tree matches STATUS.
5. Follow `docs/WORKFLOW.md` for the task loop. `/next` automates steps 1–5.

## Non-negotiable rules

1. **Order.** Take the first unblocked `todo` task in `docs/TASKS.md`. Don't skip
   ahead or start side quests. If a better order exists, propose it (with a
   confidence score) and update TASKS.md before acting.
2. **Recommend with confidence.** Whenever presenting options or a decision, lead
   with your best recommendation and a confidence score, e.g. *"Recommend X (80%)"*.
   Never present a neutral menu without a pick.
3. **Interview when unsure.** If a decision is a matter of Adrian's taste, scope, or
   priorities, or you lack context, **interview Adrian** with AskUserQuestion
   (recommended option first, labelled with its confidence). Record the Q&A in
   `docs/INTERVIEW.md` and resolve/remove the item in `docs/OPEN_QUESTIONS.md`.
   Purely technical choices you may make yourself → write an ADR (`docs/adr/`).
4. **Docs stay in sync, in the same commit.** Any code change that affects
   behaviour, structure, versions, or plans updates the matching docs (see the
   sync matrix in `docs/WORKFLOW.md`). `cargo xtask check-docs` must pass.
5. **Version every commit appropriately** per `docs/VERSIONING.md`: per-crate
   SemVer driven by Conventional Commits; the `terra` crate version **is the game
   version**. Bump in the same commit; update `CHANGELOG.md`.
6. **Quality gates before every commit:** `cargo fmt --check`, `cargo clippy
   --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and
   `cargo xtask check-docs`, all in one command: `cargo xtask ci`. Never commit red.
7. **Commit + push.** Conventional Commit messages; one task ≈ one commit (split if
   large). Push to `origin main` and wait for CI with `cargo xtask wait-ci`. Fix red CI first.
8. **Learn across `/clear`.** Context is wiped often. Whenever something goes wrong or
   surprises us (red gate/CI, wrong assumption, tool quirk, doc drift, a correction from
   Adrian), log it in `docs/LESSONS.md` in the same commit and promote it to a rule, or
   better, to an automated check. Never repeat a mistake that's already in LESSONS.md.
9. **Review reminders.** Remind Adrian to run `/code-review` at the end of every
   milestone and after large/risky tasks, and `/security-review` before shipping
   anything touching networking, save files, mod loading, or a release to friends.
   Track this in the "Reviews" section of `docs/STATUS.md`.
10. **Performance is a feature.** Respect budgets in `docs/PERFORMANCE.md`. Hot-path
    changes need a benchmark or a measured before/after. No regressions without an ADR.
11. **Assets & legal.** Never add Mojang/Minecraft assets, code, or names of their
    proprietary content files. All art is original or CC0 with attribution.

## Map of the docs

| File | Purpose |
|------|---------|
| `docs/LESSONS.md` | Mistakes and surprises, turned into rules. Read the Rules every session |
| `docs/STATUS.md` | "You are here": milestone, game version, last/next task, review debt |
| `docs/TASKS.md` | Ordered, actionable tasks for current (+ next) milestone |
| `docs/ROADMAP.md` | All milestones M0…M14+, scope and exit criteria |
| `docs/WORKFLOW.md` | Step-by-step task loop, definition of done, doc-sync matrix |
| `docs/VERSIONING.md` | Bump rules, cascade, tags, changelog format |
| `docs/ARCHITECTURE.md` | Crates, threads, data flow, key data structures |
| `docs/PERFORMANCE.md` | Targets, budgets, reference hardware, benchmark log |
| `docs/VISION.md` | Product pillars, scope, non-goals |
| `docs/INTERVIEW.md` | Log of every interview with Adrian (source of truth for preferences) |
| `docs/OPEN_QUESTIONS.md` | Unresolved questions; interview Adrian before they block work |
| `docs/adr/` | Architecture Decision Records (numbered, immutable once accepted) |
| `CHANGELOG.md` | Per game version, lists crate bumps and changes |

## Conventions (short)

- Rust edition 2024, toolchain pinned in `rust-toolchain.toml`.
- Libraries: `thiserror` errors; binaries: `anyhow`. No `unwrap()` outside tests
  and proven-infallible spots (comment why). `unsafe` is denied unless an ADR allows it.
- Logging via `tracing`; profiling via the `profiling` crate (Tracy backend feature).
- Pick dependency versions by checking crates.io at task time; don't trust memory.
- Commit trailer: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
