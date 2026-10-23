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

### T-0004 xtask: `bump`, `check-docs`, `ci` — `todo`
- **Crates:** `terra-xtask`.
- `bump <crate> <major|minor|patch>`: edits the crate version, updates internal dependency
  requirements, cascades to dependents per VERSIONING.md (highest level wins,
  `terra-xtask` never cascades), prints a `Bumps:` line. `--dry-run`.
- `check-docs`: fails if (a) a workspace crate is missing from the ARCHITECTURE crate table
  or its version differs, (b) CHANGELOG lacks a heading for the current `terra` version,
  (c) STATUS.md game version ≠ `terra` version, (d) TASKS.md has more than one `in progress` task.
- `ci`: runs fmt check, clippy `-D warnings`, tests, `cargo doc` with `-D warnings`, check-docs.
- Use a TOML-preserving editor (`toml_edit`) so formatting and comments survive.
- **AC:** unit tests for bump cascade and each check; `cargo xtask ci` green.
- **Docs:** VERSIONING (mechanics), WORKFLOW (commands), ARCHITECTURE.

### T-0005 CI on GitHub Actions — `todo`
- `.github/workflows/ci.yml`: ubuntu-latest, toolchain from `rust-toolchain.toml`,
  `Swatinem/rust-cache`, run `cargo xtask ci`. Trigger on push and PR to `main`.
- **AC:** green run on GitHub; README CI badge.
- **Docs:** README, WORKFLOW (CI check step).

### T-0006 Diagnostics foundation in `terra-core` — `todo`
- `tracing` + `tracing-subscriber` init with `TERRA_LOG` env filter, pretty dev output.
- Panic hook that writes a crash log (timestamp, version, backtrace) to the
  platform data dir (`directories` crate) and logs its path.
- `profiling` crate wired with an optional `profile-tracy` feature on `terra`.
- **AC:** tests for log init idempotence; a forced panic in a test writes a crash file.
- **Docs:** ARCHITECTURE §8, README (env vars).

### T-0007 Linux build prerequisites — `todo`
- Document system packages for wgpu/winit on Arch (vulkan-icd-loader, mesa/vulkan-intel,
  wayland, libxkbcommon, …) and Ubuntu (for CI and friends). Optional: `mold` for faster linking.
- **AC:** README "Building on Linux" section; CI installs the same Ubuntu packages.

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
