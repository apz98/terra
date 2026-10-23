# Changelog

All notable changes. Format and bump rules: [`docs/VERSIONING.md`](docs/VERSIONING.md).
Headings are game versions (`terra` crate) unless noted.

## [Unreleased]
- docs: `docs/LESSONS.md` lessons log, wired into CLAUDE.md, `/next` and the WORKFLOW retro step
- chore(workspace): Cargo workspace scaffold with `terra`, `terra-core`, `terra-xtask` at 0.0.0,
  pinned toolchain 1.99.1, shared lints and build profiles (T-0003)
- docs: repository published at https://github.com/apz98/terra (T-0002)

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
