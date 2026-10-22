# 0004 — Per-crate SemVer driven by Conventional Commits
- Status: Accepted
- Date: 2026-10-22
- Confidence: 85%
- Decided by: Adrian (interview: "per-crate" + "Conventional Commits"; `terra` = game version)

## Context
Adrian wants versions starting at 0.0.0 and bumped appropriately on every commit, with
crates versioned independently and bumps derived from commit types.

## Decision
- Each crate carries its own SemVer version, starting at `0.0.0`.
- Pre-1.0: `feat` or breaking → minor; `fix`/`perf`/behavioural `refactor` → patch;
  docs/test/ci/build/chore/style → no bump.
- Dependents that ship a changed crate (`terra`, `terra-server`, intermediates) bump by
  the highest level among their changed dependencies.
- The `terra` crate version is the player-facing game version; game versions get git
  tags `v<version>`. The docs-only initial commit is the `v0.0.0` baseline.
- 1.0.0 is cut manually at the Friends Release (M11).

Full rules: `docs/VERSIONING.md`.

## Consequences
- Version numbers carry meaning per crate; the game version moves on every shipped change.
- Bookkeeping is non-trivial, so `cargo xtask bump` and `check-docs` automate and verify it (T-0004).
- Pre-1.0 minor numbers will grow large (e.g. 0.40.0). That's expected.

## Alternatives considered
- Single workspace version: simpler, but Adrian chose per-crate.
- `release-plz`/`cargo-release`: built for crates.io publishing and PR flows. A small xtask
  fits our commit-by-commit, unpublished workflow better. Revisit if we publish crates.
