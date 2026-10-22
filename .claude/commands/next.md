---
description: Resume Terra development — load state, recommend the next task, and start it
argument-hint: "[optional task id, e.g. T-0003, or a note]"
---

You are resuming work on **Terra** after a context reset. Follow this exactly.

## 1. Load state (read, don't skim)
- `docs/STATUS.md`, `docs/TASKS.md`, `docs/OPEN_QUESTIONS.md`
- `docs/WORKFLOW.md` (the task loop + definition of done) and `docs/VERSIONING.md`
- `git status`, `git log --oneline -10`, and if a remote exists `gh run list -L 3`

## 2. Sanity checks
- Working tree dirty? Figure out whether it is unfinished work from the last session
  (check STATUS "In progress") and either finish it or report it. Never discard it silently.
- Last CI run red? Fixing it is the next task, ahead of everything else.
- STATUS.md disagreeing with git/Cargo versions? Fix the docs first (`docs:` commit).

## 3. Review reminders
Check the "Reviews" section of STATUS.md against the triggers in `docs/WORKFLOW.md`.
If a review is due, open your report with a clear reminder, e.g.
> ⚠️ Review due: run `/code-review` (milestone M1 completed since last review).

## 4. Report (short)
- Where we are: milestone, game version, last completed task.
- **Next task**: ID + title + one-line goal, and your recommendation with a
  confidence score (e.g. "Recommend starting T-0104 now (90%)"). If you believe a
  different order is better, say so with confidence and why.
- Any open question that blocks it → **interview Adrian now** with AskUserQuestion
  (recommended option first, with confidence), record the answers in
  `docs/INTERVIEW.md`, and update `docs/OPEN_QUESTIONS.md`.

## 5. Start working
Unless blocked on Adrian's input, **begin the task immediately**. Don't wait for a
"go ahead". Follow the task loop in `docs/WORKFLOW.md` through to a pushed,
green, docs-synced, version-bumped commit. Then update STATUS.md and say what's next.

If an argument was given, treat it as the requested task or focus: $ARGUMENTS
