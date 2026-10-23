# Tasks

Ordered. Take the **first unblocked `todo`**. Status: `todo` · `in progress` ·
`blocked (reason)` · `done (version, commit)`. IDs: `T-<milestone><nn>` (M0 → T-00nn,
M1 → T-01nn …). Finished milestones move to `docs/tasks/archive/`.

Each task lists the crates it touches, its acceptance criteria (AC), and the docs to update.

---

## M0 — Foundations

### T-0001 Documentation baseline — `done (0.0.0)`
Interview, vision, architecture, roadmap, tasks, workflow, versioning, ADRs, `/next`.

### T-0002 Publish to GitHub — `done (0.0.0)`
- https://github.com/apz98/terra (public). Created the public repo `apz98/terra`, description "A Minecraft-style voxel sandbox in Rust",
  topics (rust, voxel, game, wgpu, minecraft-clone); push `main` + tag `v0.0.0`.
- **AC:** `gh repo view apz98/terra` works; remote `origin` set; tag visible.
- **Docs:** README (repo URL), STATUS.

### T-0003 Workspace scaffold — `done (0.0.0)`
- **Crates:** new `terra` (bin), `terra-core` (lib), `terra-xtask` (bin), all `0.0.0`.
- `rust-toolchain.toml` pinned to the current stable (1.99.x) with rustfmt + clippy.
- Root `Cargo.toml`: workspace (resolver 3, edition 2024), `[workspace.dependencies]`,
  `[workspace.lints]` (`unsafe_code = "deny"`, clippy `all` warn + chosen pedantic lints),
  profiles (dev: deps `opt-level = 3`; release: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`).
- `.cargo/config.toml`: alias `xtask = "run -p terra-xtask --"`.
- `rustfmt.toml`, `.gitignore` check, crate-level rustdoc for each crate.
- `terra` binary prints `Terra <version>` using `env!("CARGO_PKG_VERSION")`.
- Commit as `chore(workspace)` (no bump, per VERSIONING.md).
- **AC:** `cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` are clean; `cargo run -p terra` prints `Terra 0.0.0`.
- **Docs:** ARCHITECTURE crate table (versions + status), README build section.

### T-0004 xtask: `bump`, `check-docs`, `ci` — `done (terra-xtask 0.1.0)`
- **Crates:** `terra-xtask`.
- `bump <crate> <major|minor|patch>`: edits the crate version, updates internal dependency
  requirements, cascades to dependents per VERSIONING.md (highest level wins,
  `terra-xtask` never cascades), prints a `Bumps:` line. `--dry-run`.
- `check-docs`: fails if (a) a workspace crate is missing from the ARCHITECTURE crate table
  or its version differs, (b) CHANGELOG lacks a heading for the current `terra` version,
  (c) STATUS.md game version ≠ `terra` version, (d) TASKS.md has more than one `in progress` task,
  (e) a task ID (`T-nnnn`) mentioned in docs doesn't exist in TASKS.md or the archive
  (from LESSONS.md, 2026-10-22).
- `ci`: runs fmt check, clippy `-D warnings`, tests, `cargo doc` with `-D warnings`, check-docs.
- Use a TOML-preserving editor (`toml_edit`) so formatting and comments survive.
- **AC:** unit tests for bump cascade and each check; `cargo xtask ci` green.
- **Docs:** VERSIONING (mechanics), WORKFLOW (commands), ARCHITECTURE.

### T-0009 Fix `/code-review` findings on T-0004 — `done (terra-xtask 0.1.1)`
Review of `2eea2bc` (2026-10-22). Ahead of T-0005 because CI will run this code on every push.
- **Crates:** `terra-xtask`.
1. **1.0 guard:** refuse any bump (direct or cascaded) that takes a `0.x` crate to `1.0.0`
   unless `--allow-1.0` is given (VERSIONING: 1.0.0 is a manual M11 decision).
2. **Atomic bump:** keep the original manifest and `Cargo.lock` text; if writing or
   `cargo update --workspace` fails, restore them so a retry can't double-bump.
3. **Doc discovery:** scan only `.claude/commands/` (not `.claude/worktrees/`), and don't
   follow symlinked directories.
4. **Root `[workspace.dependencies]`:** update internal crates' requirements there too.
5. **Fences:** track the opening fence's character and length; every check (crate tables,
   in-progress count, task definitions, task mentions) ignores fenced blocks.
6. **Discovery via `cargo metadata --no-deps`** (members, `exclude`, manifest paths, the
   dependency graph with kinds, renames and targets); keep `toml_edit` only for edits.
7. **Keep requirement operators:** `=0.1.0` → `=0.2.0`, `~0.1` → `~0.2` (via `semver::VersionReq`).
8. **Pre-releases:** `0.3.0-alpha.1` + patch → `0.3.0` (+ minor → `0.3.0` when patch is 0;
   + major → `1.0.0` when minor and patch are 0), as cargo-release does.
9. **`ci` order:** run check-docs first (it takes milliseconds).
10. **Save only changed manifests.**
- **AC:** a unit test per finding (2 via an injected failing `cargo update`); `cargo xtask ci` green.
- **Docs:** VERSIONING (guard, pre-release rule, operators), README/WORKFLOW (`--allow-1.0`), CHANGELOG, STATUS.

### T-0005 CI on GitHub Actions — `done (no bump)`
- `.github/workflows/ci.yml`: ubuntu-latest, toolchain from `rust-toolchain.toml`,
  `Swatinem/rust-cache`, run `cargo xtask ci`. Trigger on push and PR to `main`.
  Actions pinned by commit SHA; read-only token; stale runs cancelled.
- **AC:** green run on GitHub; README CI badge.
- **Docs:** README, WORKFLOW (CI check step).

### T-0010 Fix `/code-review` findings on T-0009 and T-0005 — `done (terra-xtask 0.2.0)`
Review of `b3d3dff..HEAD` (2026-10-22), 9 findings, all confirmed against the code. Ahead
of T-0006 because two of them can make CI report red on a good commit.
- **Crates:** `terra-xtask`; also `.github/workflows/ci.yml`, CLAUDE.md, WORKFLOW.
1. **Don't cancel `main` runs:** `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`,
   so every `main` commit gets a result and a watched run is never reported as `cancelled`.
2. **CLAUDE.md rule 7** still says `gh run list -L 1` (the race LESSONS rule 8 bans).
   Point it at the new command from item 3.
3. **`cargo xtask wait-ci [<sha>]`** replaces the WORKFLOW shell loop (also review item 9):
   filters by `--workflow ci.yml` and commit SHA (default `HEAD`), polls with a timeout,
   exits non-zero on failure, timeout, or no run. Polling logic unit-tested with an injected `gh`.
4. **Lockfile rollback:** `fs::read(Cargo.lock)` errors other than `NotFound` abort the
   bump before writing; the lock is restored even if a manifest restore fails.
5. **Backticked versions:** crate-table version cells and the STATUS game version accept
   `` `0.1.1` `` (trim backticks like the name cell).
6. **Indented fences:** fences inside nested list items (indent > 3) are treated as
   fences, so their contents are ignored by every check.
7. **Anchor the crate table:** take the table under ARCHITECTURE's `## 2.` heading (and STATUS's
   `## Crate versions`), not the first table with `Crate` and `Version` columns.
8. **check-docs without `toml_edit`:** read versions from `cargo metadata` only; keep
   `Workspace::load` for `bump`.
- **AC:** a unit test per xtask finding (3–8); first CI run on the new workflow green;
  `cargo xtask ci` green.
- **Docs:** WORKFLOW step 9, CLAUDE.md rule 7, `/next` (CI check), README (`wait-ci`),
  VERSIONING if needed, CHANGELOG, STATUS.

### T-0006 Diagnostics foundation in `terra-core` — `done (terra 0.1.0)`
- `tracing` + `tracing-subscriber` init with `TERRA_LOG` env filter, pretty dev output.
- Panic hook that writes a crash log (timestamp, version, backtrace) to the
  platform data dir (`directories` crate) and logs its path.
- `profiling` crate wired with an optional `profile-tracy` feature on `terra`.
- **AC:** tests for log init idempotence; a forced panic in a test writes a crash file.
- **Docs:** ARCHITECTURE §8, README (env vars).

### T-0007 Linux build prerequisites — `done (no bump)`
- Document system packages for wgpu/winit on Arch (vulkan-icd-loader, mesa/vulkan-intel,
  wayland, libxkbcommon, …) and Ubuntu (for CI and friends). Optional: `mold` for faster linking.
- **AC:** README "Building on Linux" section; CI installs the same Ubuntu packages.
- Note: `ubuntu-latest` is migrating to Ubuntu 26 (from 2026-10-19). Check package names
  on the image CI actually uses, or pin `ubuntu-24.04` and say why.
- **Done:** the Ubuntu list lives in `.github/ubuntu-packages.txt` (CI installs it, README
  links it); CI pinned to `ubuntu-24.04`. `mold` skipped: Rust already links with `rust-lld`.

### T-0008 M0 wrap-up — `todo`
- Verify M0 exit criteria; archive M0 tasks; expand M1 tasks; remind Adrian to run `/code-review`.

---

## M1 — First Light (sketch, expanded during T-0008)

- **T-0101** `terra-client` + `terra-render` crates: winit `ApplicationHandler`, window, wgpu
  surface/device (log adapter info), resize, clear color, vsync toggle.
- **T-0102** Frame loop + timing: delta time, fps counter, input state (keyboard/mouse, cursor grab).
- **T-0103** Fly camera: glam, reverse-Z perspective, mouse look, WASD/space/shift, speed boost.
- **T-0104** Placeholder textures: procedural 16×16 textures → 2D texture array with mipmaps. (Q2 decides final art.)
- **T-0105** One hardcoded 16³ section: face-culled mesh, packed vertex format, WGSL shader, depth buffer.
- **T-0106** Debug overlay (fps, frame time, position, adapter). Decide egui vs custom text → ADR.
- **T-0107** M1 wrap-up: baseline perf numbers into PERFORMANCE.md, playtest with Adrian, `/code-review` reminder.
