# Changelog

All notable changes. Format and bump rules: [`docs/VERSIONING.md`](docs/VERSIONING.md).
Headings are game versions (`terra` crate) unless noted.

## [Unreleased]
- ci: installs the Ubuntu system packages from `.github/ubuntu-packages.txt` and checks
  Vulkan with `vulkaninfo`; runner pinned to `ubuntu-24.04` (T-0007)
- docs: README "Building on Linux" lists the Arch and Ubuntu packages (T-0007)

## terra 0.1.0 — 2026-10-22
Bumps: terra-core 0.0.0 → 0.1.0, terra 0.0.0 → 0.1.0
### Added
- `terra_core::diagnostics`: `init_logging()` installs the `tracing` subscriber (stderr,
  `TERRA_LOG` filter, default `info`; idempotent), and `install_crash_handler()` writes a
  crash report (time, version, OS, thread, message, location, backtrace) to
  `<data dir>/crash-reports/` on panic and logs its path (T-0006)
- `terra` initialises logging and crash reports at startup, and has a `profile-tracy`
  feature that streams `profiling` scopes to Tracy (T-0006)
### Other
- ci: runs on `main` are never cancelled by a newer push, only PR runs are (T-0010)
- ci: GitHub Actions workflow runs `cargo xtask ci` on push and PR to `main`; README badge (T-0005)
- docs: `docs/LESSONS.md` lessons log, wired into CLAUDE.md, `/next` and the WORKFLOW retro step
- chore(workspace): Cargo workspace scaffold with `terra`, `terra-core`, `terra-xtask` at 0.0.0,
  pinned toolchain 1.99.1, shared lints and build profiles (T-0003)
- docs: repository published at https://github.com/apz98/terra (T-0002)

## terra-xtask 0.2.0 — 2026-10-22
Bumps: terra-xtask 0.1.1 → 0.2.0
### Added
- `wait-ci [<commit>]`: waits for the `ci.yml` run of a pushed commit (default `HEAD`)
  and fails on failure, cancellation, timeout, or no run. Replaces the shell loop in
  WORKFLOW step 9, CLAUDE.md rule 7 and the `/next` CI check (T-0010)
### Fixed
Fixes for the `/code-review` findings on T-0009 and T-0005 (T-0010):
- `bump` stops before writing anything if `Cargo.lock` exists but can't be read, and a
  failed rollback still restores every other manifest and `Cargo.lock`
- `check-docs` takes each crate table from its own section (ARCHITECTURE `## 2.`,
  STATUS `## Crate versions`), accepts backticked versions, treats indented fences
  (nested list items) as fences, and reads versions from `cargo metadata` alone

## terra-xtask 0.1.1 — 2026-10-22
Bumps: terra-xtask 0.1.0 → 0.1.1
### Fixed
Fixes for the `/code-review` findings on T-0004 (T-0009):
- `bump` refuses to take a 0.x crate to 1.0.0 (directly or by cascade) without `--allow-1.0`
- `bump` rolls back manifests and `Cargo.lock` if writing or `cargo update` fails, so a
  retry can't bump twice. It writes only the manifests that change.
- `bump` updates internal requirements in the root `[workspace.dependencies]`, and keeps
  requirement operators (`=0.1.0` → `=0.2.0`, `~0.1` → `~0.2`)
- `bump` releases pre-releases instead of skipping them (`0.3.0-alpha.1` + patch → `0.3.0`)
- Workspace members and the dependency graph come from `cargo metadata` (honours `exclude`)
- `check-docs` reads only `.claude/commands/` (not worktrees), doesn't follow symlinks, and
  every check ignores fenced code blocks, matched by CommonMark rules
- `ci` runs `check-docs` first

## terra-xtask 0.1.0 — 2026-10-22
Bumps: terra-xtask 0.0.0 → 0.1.0
### Added
- `cargo xtask bump <crate> <level> […] [--dry-run]`: per-crate SemVer bumps with the
  cascade from VERSIONING.md (highest level wins, dev-deps and `terra-xtask` don't
  cascade), comment-preserving manifest edits, `Cargo.lock` refresh, `Bumps:` line (T-0004)
- `cargo xtask check-docs`: crate tables in ARCHITECTURE and STATUS match the manifests,
  CHANGELOG heading and STATUS game version match `terra`, at most one task in progress,
  every task ID mentioned is defined (T-0004)
- `cargo xtask ci`: fmt, clippy `-D warnings`, tests, rustdoc `-D warnings`, check-docs (T-0004)

## terra 0.0.0 — 2026-10-22
Baseline, documentation only (no crates yet).
### Added
- Kickoff interview record, vision, architecture, roadmap (M0–M14), tasks, workflow,
  versioning, performance targets, open questions.
- ADRs 0001–0011.
- `/next` Claude Code command for resuming development.
- Dual MIT / Apache-2.0 license.
