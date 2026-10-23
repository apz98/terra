# Changelog

All notable changes. Format and bump rules: [`docs/VERSIONING.md`](docs/VERSIONING.md).
Headings are game versions (`terra` crate) unless noted.

## [Unreleased]

## terra 0.4.0 — 2026-10-23
Bumps: terra-client 0.2.0 → 0.3.0, terra 0.3.0 → 0.4.0
### Added
- Frame timing and input (T-0102). Clicking into the window grabs the mouse (locked on
  Wayland, confined on X11) and hides the cursor. `Escape` or switching to another window
  releases it. The click that grabs isn't passed on to the game.
- terra-client: `timing::FrameTimer` gives each frame a delta time (at most 250 ms after a
  stall), plus fps and average/max frame time over the last second, from a fixed-size ring
  buffer. `input::InputState` tracks physical keys and mouse buttons held / pressed /
  released this frame, and raw mouse movement (counted only while the mouse is grabbed).
  Losing focus releases every held key and button.
- With `TERRA_LOG=info,terra_client=debug`, fps and frame time are logged once a second.
  `profiling` scopes around update and render, and one frame mark per frame for Tracy.
### Changed
- `F9` is read from the frame's input state, so it takes effect on the next frame.

## terra 0.3.0 — 2026-10-23
Bumps: terra-render 0.1.0 → 0.2.0, terra-client 0.1.0 → 0.2.0, terra 0.2.0 → 0.3.0
### Fixed
Fixes for the `/code-review` findings on T-0101 (T-0110):
- A minimized or hidden window no longer spins a CPU core. When there's nothing to draw
  into, the client retries every 100 ms and on `Resized`/`Occluded(false)`.
- A lost GPU device stops the game with "the GPU device was lost: …". Before, the window
  froze and the log said "validation error" every frame.
- A window larger than the GPU's largest texture (8192 px with default limits) no longer
  crashes the game. The surface is clamped to that size, with a warning.
- Lost surface: the new surface's format and present modes are queried again. If it can't
  be recreated, the game stops with an error instead of panicking on the old surface.
- `Outdated` reconfigures at the window's current size, not the cached one. A
  `Suboptimal` surface is reconfigured once, not every frame.
### Changed
- terra-render (breaking): `WindowSurface::acquire` takes a window-size closure and returns
  `Result<Acquired, GpuError>` (`Frame`, `Retry` or `Wait`). New: `Gpu::lost()` and
  `GpuError::DeviceLost`. A GPU test checks that a destroyed device is reported.
### Other
- docs: LESSONS rule 11, follow every error branch to the next iteration.

## terra 0.2.0 — 2026-10-23
Bumps: terra-render 0.0.0 → 0.1.0, terra-client 0.0.0 → 0.1.0, terra 0.1.2 → 0.2.0
### Added
- The game opens a window: `Terra <version>`, 1280×720 logical, resizable, Wayland with
  X11 as fallback, app id `terra`. It's cleared to sky blue (`#87CEEB`) every frame and
  closes cleanly. `F9` toggles vsync (a debug key until the M4 settings menu) (T-0101).
- New crate `terra-render` (wgpu 30): GPU setup with the highest-performance adapter,
  `WGPU_BACKEND`/`WGPU_ADAPTER_NAME` overrides (a name that matches nothing is an error
  listing the adapters found), the adapter logged at `info`; a window surface with an
  sRGB format, vsync → present mode (`Fifo`; off: `Mailbox`, `Immediate`, `Fifo`),
  resize, and recovery from outdated, suboptimal and lost surfaces.
- New crate `terra-client` (winit 0.30): the window, the event loop and the frame loop.
  `terra` now runs it instead of printing its version.
- A headless GPU test clears an offscreen texture and reads it back. CI runs it on
  lavapipe with `TERRA_REQUIRE_GPU=1`; without a GPU it skips locally.
### Other
- winit is built without `wayland-csd-adwaita` (`-notitle` instead): its title-bar font
  rendering pulls in the unmaintained `ttf-parser` (RUSTSEC-2026-0192), which
  `cargo deny check` rejects. Wayland decorations stay; their title text goes.
- docs: README lists the controls, the GPU environment variables and the optional Vulkan
  validation layer.

## terra 0.1.2 — 2026-10-23
Bumps: terra-core 0.1.1 → 0.1.2, terra 0.1.1 → 0.1.2, terra-xtask 0.2.1 → 0.3.0
### Added
- `deny.toml`: `cargo deny check` enforces the dependency license allow list (MIT,
  Apache-2.0, BSD-2/3-Clause, Zlib, ISC, Unicode-3.0, CC0-1.0; copyleft is rejected),
  RustSec advisories (vulnerable, unmaintained, unsound or yanked crates fail) and
  crates.io-only sources, for Linux and Windows targets with all features. ADR 0010's
  "checked with `cargo-deny`, added later" is now enforced (T-0108).
- xtask `ci`: runs `cargo deny check` after fmt, with an install hint if cargo-deny is
  missing and a note if the installed version differs from CI's (0.20.2). A test checks
  that `deny.toml` rejects a GPL dependency and accepts an MIT one.
- ci: installs cargo-deny 0.20.2 with `taiki-e/install-action` (pinned by SHA).
### Changed
- terra-core: the data directory comes from `etcetera` instead of `directories`, whose
  `dirs-sys` pulls in the MPL-2.0 `option-ext`. Same paths as before
  (`~/.local/share/terra`, `%APPDATA%\Terra\data`).
### Other
- docs: README lists cargo-deny as a dev tool and shows the current milestone.

## terra 0.1.1 — 2026-10-23
Bumps: terra-core 0.1.0 → 0.1.1, terra 0.1.0 → 0.1.1, terra-xtask 0.2.0 → 0.2.1
### Fixed
Fixes for the `/code-review` findings on M0 (T-0109):
- Crash reports: the hook always prints the report's path (or the write error) to stderr,
  so `TERRA_LOG=off` can't hide it. Without a home directory, reports go to
  `./crash-reports/` instead of the shared `/tmp/terra`.
- `TERRA_LOG`: invalid directives are dropped and the valid ones kept (before, one bad
  directive threw away the whole value); one warning names every invalid directive.
- ci: pushes to `main` get one concurrency group per commit. A queued run cancels the
  group's pending run even with `cancel-in-progress: false`, so a third quick push could
  cancel the second (0.1.0 wrongly said `main` runs are never cancelled).
- xtask `wait-ci`: a failed `gh run list` is retried on the next poll, giving up after 6
  failures in a row; a run that vanishes after it was seen counts as such a failure
  instead of "no run, was it pushed?".
- xtask `check-docs`: a closing fence indented 4+ columns more than its opening fence
  doesn't close the code block (CommonMark).
- xtask `ci`: also runs `cargo clippy -p terra --features profile-tracy -- -D warnings`.
### Other
- ci: installs the Ubuntu system packages from `.github/ubuntu-packages.txt` and checks
  Vulkan with `vulkaninfo`; runner pinned to `ubuntu-24.04` (T-0007)
- docs: README "Building on Linux" lists the Arch and Ubuntu packages (T-0007)
- docs: M0 complete; its tasks are archived in `docs/tasks/archive/M0.md`, M1 is expanded
  into tasks (T-0108, T-0101–T-0107), and the M0 perf snapshot is recorded (T-0008)

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
