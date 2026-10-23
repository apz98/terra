# Versioning

Decided in the interview (2026-10-22) and [ADR 0004](adr/0004-versioning.md).

## Summary

- **Every crate has its own SemVer version**, starting at `0.0.0`.
- **Bumps are driven by Conventional Commit types**, applied to each crate the commit changes.
- **The `terra` crate's version is the game version** players see. It bumps
  whenever any crate it ships changes.
- **The baseline:** the initial docs commit is game version `0.0.0` (tag `v0.0.0`).

## Bump rules (pre-1.0, which is where we are)

| Commit type touching a crate's code | Bump for that crate |
|---|---|
| Breaking change (`feat!`, `fix!`, `BREAKING CHANGE:` footer) | minor `0.x.0` |
| `feat` | minor `0.x.0` |
| `fix`, `perf`, `refactor` (behaviour-affecting) | patch `0.0.x` |
| `docs`, `test`, `ci`, `build`, `chore`, `style` only | **no bump** |

After `1.0.0`: breaking → major, feat → minor, fix/perf → patch.

Notes:
- Creating a new crate (scaffold) is `chore` and starts it at `0.0.0`. Its first `feat` makes it `0.1.0`.
- Pre-1.0, `feat` and breaking changes both bump the minor. That's intended, and versions like `0.37.0` are normal.
- If one commit has several types, the highest bump wins per crate.
- Docs-only commits don't bump. Their CHANGELOG lines go under `## [Unreleased]`
  and move under the next version heading when a bump happens.

## Cascade to the game version

`terra` (the game binary) and `terra-server` ship other crates. When a crate
they depend on (directly or transitively) bumps, they bump too, by the **highest** level
among the changed crates. Example: `terra-render` gets a `feat` →
`terra-render 0.3.0 → 0.4.0`, `terra-client` (depends on it) `0.5.2 → 0.6.0`,
`terra 0.8.1 → 0.9.0`. `terra-server` doesn't depend on render, so it doesn't bump.

Internal tool crates (`terra-xtask`) follow the same rules but never cascade.

## Mechanics

- Versions live in each crate's own `Cargo.toml` (no `version.workspace`).
- Internal deps use `path` plus a `version` requirement; `cargo xtask bump` keeps them in sync.
- `cargo xtask bump <crate> <major|minor|patch> [<crate> <level> …] [--dry-run] [--allow-1.0]`:
  - bumps each named crate, then cascades to every crate that ships it through
    `[dependencies]` or `[build-dependencies]` (including `[target.*]` tables and renamed
    deps). `[dev-dependencies]` don't ship, so they don't cascade, but their requirements
    are still updated. Membership and the dependency graph come from `cargo metadata`,
    so `exclude`, globs and `foo.workspace = true` behave exactly as in a build.
  - **Name every crate a commit changes in one call.** The highest level per crate wins
    and each crate bumps once. Two separate `minor` calls would bump `terra` twice.
  - `terra-xtask` is never bumped by a cascade and never cascades.
  - **Refuses to take any `0.x` crate to `1.0.0`**, directly or through the cascade,
    unless `--allow-1.0` is passed (see the 1.0.0 rule below).
  - **Pre-releases are released, not skipped:** `0.3.0-alpha.1` + patch or minor →
    `0.3.0`; + major → `1.0.0` (or `x.0.0` when minor and patch are already 0).
  - Requirements keep their operator and precision: `0.1.0` → `0.2.0`, `=0.1.0` →
    `=0.2.0`, `~0.1` → `~0.2`. Ranges like `>=0.1, <0.3` are refused: edit them by hand.
    Internal entries in the root `[workspace.dependencies]` are updated too.
  - rewrites only the manifests that change, with `toml_edit` (comments and layout
    survive), runs `cargo update --workspace` so `Cargo.lock` matches, and prints the
    `Bumps:` line for the commit message. If any step fails, the manifests and
    `Cargo.lock` are restored, so it's safe to re-run. `--dry-run` only prints.
- `cargo xtask check-docs` fails if a crate table (the first table under ARCHITECTURE's
  `## 2.` heading and under STATUS's `## Crate versions`), the CHANGELOG heading, or the
  STATUS game version disagrees with the versions `cargo metadata` reports. Versions may
  be written plain or in backticks.
- **Tags:** only the game version is tagged: `v<terra version>` on the commit that
  set it. The tag is annotated with the crate bump list.
- **1.0.0** is a manual decision at the end of the Friends Release milestone (M11).

## CHANGELOG format

```markdown
## [Unreleased]
- docs: …

## terra 0.9.0 — 2026-11-03
Bumps: terra-render 0.3.0 → 0.4.0, terra-client 0.5.2 → 0.6.0, terra 0.8.1 → 0.9.0
### Added
- Frustum culling for chunk sections (T-0207)
### Fixed
### Performance
```

Commits that bump only non-shipping crates (e.g. `terra-xtask`) get their own short
heading: `## terra-xtask 0.2.0 — <date>`.
