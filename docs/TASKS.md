# Tasks

Ordered. Take the **first unblocked `todo`**. Status: `todo` · `in progress` ·
`blocked (reason)` · `done (version, commit)`. IDs: `T-<milestone><nn>` (M0 → T-00nn,
M1 → T-01nn …). Finished milestones move to `docs/tasks/archive/` (M0: `archive/M0.md`).

Each task lists the crates it touches, its acceptance criteria (AC), and the docs to update.

---

## M1 — First Light

Goal and exit criteria: `ROADMAP.md` M1. A window showing a textured 16³ section you can
fly around, ≥ 60 fps on the reference machine, baseline numbers recorded.

### T-0109 Fix `/code-review` findings on M0 (`2b2725b..bc4ab5c`) — `done (0.1.1)`
Review of T-0010, T-0006, T-0007, T-0008 (2026-10-22): 8 findings, all checked against
the code. First in M1 because finding 1 can make CI report a good `main` commit as
`cancelled`, and findings 3–4 can make `wait-ci` report red CI that isn't red.
- **Crates:** `terra-xtask`, `terra-core`, `terra`; also `.github/workflows/ci.yml`.
1. **CI concurrency:** a group keeps one running and one *pending* run, and a new run
   cancels the pending one even with `cancel-in-progress: false`. Use a per-commit group
   on push (`ci-${{ github.event_name == 'pull_request' && github.ref || github.sha }}`)
   and fix the comment.
2. **Crash path always visible:** the hook prints the report path (or the write error) to
   stderr even when logging is on, since `TERRA_LOG=off` or `terra_core=off` hides the
   `tracing::error!`.
3. **`wait-ci` retries transient `gh` failures:** a failed `gh run list` is retried on the
   next poll (log it, give up after N consecutive failures) instead of ending the wait.
4. **`wait-ci` "never appeared":** apply the `appear` limit only until a run has been seen.
   An empty result after that is treated like finding 3 (retry), not "was it pushed?".
5. **Gate `profile-tracy`:** `cargo xtask ci` (and so CI) runs `cargo clippy -p terra
   --features profile-tracy -- -D warnings`. Replaces the "check by hand" note in LESSONS.
6. **Closing fences:** a closing fence may be indented at most 3 spaces more than its
   opening fence (CommonMark's rule, relative to the container), so an indented fence
   line inside a code block doesn't close it.
7. **Crash dir fallback:** don't use the shared `/tmp/terra`. Recommend (70%) creating a
   per-user dir (`0700`, refuse one owned by someone else) or falling back to the working
   directory; decide in-task and update the rustdoc and ARCHITECTURE §8.
8. **`TERRA_LOG` keeps valid directives:** filter with `parse_lossy(value)`; keep the strict
   `parse` only to produce the warning. ARCHITECTURE §8 says what an invalid value does.
- Cleanups while there: one short-SHA helper in `wait_ci.rs` (`&sha[..10]` appears twice).
- **AC:** a unit test per code finding (2–8; 3–4 with the injected `gh`); first CI run on the
  new concurrency group green; `cargo xtask ci` green.
- **Docs:** ARCHITECTURE §8, WORKFLOW step 5 (gates), LESSONS (drop the manual Tracy note),
  CHANGELOG, STATUS.

### T-0108 License and advisory checks (`cargo-deny`) — `done (0.1.2)`
First in M1 (Claude, 70%): wgpu and winit add well over a hundred crates, and ADR 0010
says licenses are checked with `cargo-deny`. Adding the check before they arrive is cheap;
afterwards it means auditing everything at once.
- `deny.toml`: allow MIT, Apache-2.0, BSD-2/3-Clause, Zlib, ISC, Unicode-3.0, CC0-1.0
  (add others only with a reason in a comment); deny copyleft; advisories (RustSec)
  deny vulnerabilities and unmaintained crates, with each ignore justified in a comment;
  bans: warn on duplicate versions; sources: crates.io only.
- CI runs `cargo deny check` (install a pinned version; action pinned by SHA). `cargo
  xtask ci` runs it too, and says how to install it if it's missing.
- **AC:** `cargo deny check` green locally and in CI; a test or dry run shows a GPL crate
  is rejected.
- **Docs:** README (dev tools), WORKFLOW step 3 + step 5, CHANGELOG. (ADRs are immutable, so
  ADR 0010's "added later" stays; CHANGELOG records that it's now enforced.)

### T-0101 Window and GPU surface (`terra-client`, `terra-render`) — `done (0.2.0)`
- **Crates:** new `terra-render` and `terra-client` (lib). `terra` calls `terra_client::run()`.
- winit `ApplicationHandler`: create the window in `resumed` (title `Terra <version>`,
  1280×720 logical, resizable). Wayland natively, X11 as winit's fallback.
- `terra-render`: instance, adapter (high performance; `WGPU_BACKEND` and
  `WGPU_ADAPTER_NAME` env overrides), device, queue, surface. Log the adapter name,
  backend, device type and driver at `info`.
- Surface: sRGB format; present mode from a pure function
  `choose_present_mode(vsync, supported)` (on → `Fifo`; off → `Mailbox`, else
  `Immediate`, else `Fifo`). Reconfigure on resize (skip 0×0 while minimized) and on
  `Lost`/`Outdated`; skip a frame on `Timeout`.
- Clear to a sky blue every frame. `F9` toggles vsync (a temporary debug binding until the
  M4 settings menu). Close cleanly on `CloseRequested`.
- **GPU tests:** a headless test renders the clear colour into an offscreen texture and
  reads it back. CI runs it on lavapipe with `TERRA_REQUIRE_GPU=1`, so a missing adapter
  fails there; locally, without that variable, it skips with a message.
- **AC:** `cargo run -p terra` opens a sky-blue window on the reference machine (Wayland),
  resizes and closes without wgpu validation errors, and logs the adapter; unit tests for
  `choose_present_mode`; the GPU test passes in CI.
- **Docs:** ARCHITECTURE §2 (two crates), §3 (main thread), §5; README (run, controls);
  `ci.yml` env; CHANGELOG.

### T-0110 Fix `/code-review` findings on T-0101 (`811d5b2..ec594f2`) — `done (0.3.0)`
Review of T-0101 (2026-10-23): 10 findings, each checked against wgpu 30.0.1 and winit
0.30.13 sources. 7 to fix (two of them with a different cause than reported), 3 need no
change. Goes ahead of T-0102 because T-0102 builds its frame loop on this code, and
findings 1–5 are a 100 % CPU spin, a frozen window and three panics.
- **Crates:** `terra-render`, `terra-client`.
1. **Redraw spin:** a skipped frame requests the next redraw at once, so a minimized
   (0×0) window spins at 100 % CPU. `acquire` says whether to redraw now or wait. While
   waiting, the client retries on a short timer (`ControlFlow::WaitUntil`), and on
   `Resized`/`Occluded(false)`. It can't rely on events alone: winit has no `Occluded`
   on Wayland.
2. **Device lost:** the callback only logs. A lost device makes `get_current_texture` return
   `Validation`: wgpu drops `DeviceError::Lost` without telling us. The code then logs
   "validation error" every frame and the window freezes. Record the loss in `Gpu`, and
   turn `Validation` into `GpuError::DeviceLost` (with the callback's reason and
   message). The client exits cleanly with that error. Recreating the device is later work
   (ARCHITECTURE §5).
3. **Recreated surface:** after `Lost`, query the capabilities again and pick the format and
   present mode again, so the F9 toggle and `configure` don't use the old surface's lists.
4. **Recreation fails:** return `GpuError::CreateSurface` instead of configuring the lost
   surface, which panics with `InvalidSurface`.
5. **Size clamp:** clamp the surface to `device.limits().max_texture_dimension_2d`
   (above it, `configure` panics with `TooLarge`), and warn when it does.
6. **Suboptimal:** reconfigure once, at the window's current size. If the next frame is
   suboptimal too, keep presenting until the size changes.
7. **Outdated:** reconfigure at the window's *current* size (the caller passes a closure; on
   X11 the size comes from the server, so it isn't read every frame), not the cached one.
   If acquiring fails twice in a row, wait (finding 1) instead of retrying at once.
- **No change:** finding 8 ("the `Validation` arm is dead code") is wrong: that arm is
  where a device loss ends up (finding 2). Finding 9 (unused `format()`/`present_mode()`) is
  true, but T-0105's pipeline and T-0106's overlay need them. Finding 10 (no profiling
  scopes or baseline) is already in T-0102's AC and T-0107.
- **AC:** unit tests for the size clamp and for the retry/wait decision; a GPU test where
  `device.destroy()` makes `Gpu` report the loss; checked by hand on the reference
  machine: resize, F9 and close still work, with no warnings in the log. CPU use while the
  window is hidden is checked on X11 if one is available, otherwise reported as not checked.
  `cargo xtask ci` green.
- **Docs:** ARCHITECTURE §3 and §5, LESSONS, CHANGELOG, STATUS.

### T-0102 Frame loop, timing and input — `todo`
- **Crates:** `terra-client`.
- `FrameTimer`: delta time (clamped, e.g. ≤ 250 ms after a stall), fps and average/max
  frame time over the last second in a fixed-size ring buffer (no per-frame allocation).
- `InputState`: physical keys (`KeyCode`) and mouse buttons held / pressed this frame /
  released this frame; raw mouse delta from `DeviceEvent::MouseMotion`. Focus loss clears
  held keys.
- Cursor grab: click grabs (`Locked`, falling back to `Confined`) and hides the cursor;
  `Escape` and focus loss release it.
- `profiling::finish_frame!()` once per frame; scopes around update and render. Fps and
  frame time logged once a second at `debug` until the overlay (T-0106).
- **AC:** unit tests for `FrameTimer` (synthetic durations → fps/avg/max, clamping) and
  `InputState` transitions; grab/release checked by hand on Wayland.
- **Docs:** README (controls), ARCHITECTURE §3, CHANGELOG.

### T-0103 Fly camera — `todo`
- **Crates:** `terra-core` (re-exports `glam`), `terra-client`, `terra-render`.
- Camera position in `DVec3` (world space); the view matrix is built camera-relative in
  `f32`, so precision holds far from the origin. Yaw/pitch, pitch clamped to ±89.9°.
- Mouse look; `W A S D` move horizontally relative to yaw; `Space` up; `Left Shift` down;
  `Left Ctrl` boosts speed. Speeds and sensitivity are named constants in one place (Q3
  decides final game feel in M4).
- Reverse-Z infinite perspective (`perspective_infinite_reverse_rh`), 70° vertical FOV,
  depth compare `Greater`, depth cleared to 0. Camera uniform buffer in `terra-render`.
- **AC:** unit tests: projection maps the near plane to depth 1 and far points toward 0;
  movement is relative to yaw; pitch clamps; the same total movement at 30 and 240 steps
  per second (frame-rate independence).
- **Docs:** README (controls), ARCHITECTURE §4 (coordinates) and §5, CHANGELOG.

### T-0104 Placeholder textures: procedural, baked to PNG assets — `todo`
Q2 (interview 2026-10-22): textures are generated by code, then kept as real PNG asset
files that the game loads and that can be refined by hand later.
- **Crates:** `terra-xtask` (generator), `terra-render` (texture array), `terra-client`.
- `cargo xtask gen-textures`: deterministic (fixed seed per texture) noise + palette
  generator writing 16×16 RGBA PNGs to `assets/textures/block/`: stone, dirt, grass top,
  grass side, sand, gravel, log side/top, planks, leaves. Our own file names, not
  Minecraft's (CLAUDE.md rule 11). It **refuses to overwrite** an existing PNG without
  `--force` (LESSONS rule 7), because committed PNGs are the art from then on.
- `assets/CREDITS.md` (ADR 0010): original work, how it was made, license.
- The game embeds the PNGs (`include_bytes!`) for M1; `terra-data` loads them from the
  built-in resource pack in M2. Decoded into a 2D texture array, `Rgba8UnormSrgb`, with a
  CPU-built mip chain; nearest magnification and minification (mip filter chosen by eye
  at the playtest).
- **AC:** generator determinism test (same input → identical bytes); a test that every
  shipped PNG is 16×16 RGBA; mip chain test (sizes 16→1, mean colour preserved); GPU
  test: the array has one layer per texture.
- **Docs:** `assets/CREDITS.md` (per ADR 0010), README (`gen-textures`), ARCHITECTURE §5
  and §7, CHANGELOG.

### T-0105 One hardcoded section: mesh, shader, depth — `todo`
- **Crates:** new `terra-mesh` (pure CPU, created now rather than M2 so meshing is
  benchmarked from the first version; flag in ARCHITECTURE), `terra-render`, `terra-client`.
- Input for now: a dense 16³ array of block ids with a per-face texture-layer lookup
  (`terra-world` palette storage replaces it in M2). Content: a small hand-built hill
  (stone, dirt, grass, a tree, an overhang and a hole) that exercises culling.
- Naive face culling (faces next to air only). Packed vertex: position (5 bits × 3),
  face (3 bits), corner, texture layer, in ≤ 8 bytes; unpacked in WGSL.
- Shader: sample the texture array, fixed per-face shading (no lighting yet), back-face
  culling, `Depth32Float` reverse-Z depth buffer recreated on resize.
- Criterion benchmark for meshing one section → first PERFORMANCE.md entry against the
  ≤ 100 µs budget.
- **AC:** unit tests: one block → 6 quads, two adjacent → 10, a full section → 6 × 256
  quads, vertex pack/unpack round-trip; benchmark recorded; checked by hand: textured
  section visible and correctly depth-sorted from every side.
- **Docs:** ARCHITECTURE §2 (`terra-mesh` now M1) and §5, PERFORMANCE, CHANGELOG.

### T-0106 Debug overlay — `todo`
- **Crates:** `terra-client`, `terra-render`.
- Decide UI text tech (egui vs a custom bitmap-font renderer) → ADR, considering M4 menus
  and HUD too (OPEN_QUESTIONS technical list).
- `F3` toggles an overlay: fps, frame time (avg/max over 1 s), position (block, section and
  chunk coords), facing, adapter + backend, vsync state, game version.
- **AC:** ADR accepted; overlay readable on the reference machine; its CPU/GPU cost
  measured (with vs without) and recorded in PERFORMANCE.md.
- **Docs:** ADR, OPEN_QUESTIONS (remove the UI item), README (controls), ARCHITECTURE §8,
  CHANGELOG.

### T-0107 M1 wrap-up — `todo`
- Baseline numbers on the reference machine (fps with vsync off, frame time p50/p99,
  RSS, startup time) into PERFORMANCE.md; verify the ≥ 60 fps exit criterion.
- Playtest with Adrian; record feedback in INTERVIEW.md.
- Milestone wrap-up (WORKFLOW): archive M1, expand M2, `/code-review` reminder.

---

## M2 — Terrain (sketch, expanded during T-0107)

- `terra-world`: block-state registry types, `ChunkPos`/`SectionPos`, palette sections.
- `terra-data`: built-in data pack + resource pack (blocks, textures) replaces M1's embedded list.
- `terra-worldgen`: 2D-noise heightmap terrain, deterministic per seed (golden test).
- `terra-mesh`: binary greedy meshing + per-vertex AO, against the M1 benchmark.
- Worker pool (ADR: rayon vs custom priority pool); load/unload by render distance.
- Shared GPU buffers + indirect draws; frustum culling; perf checkpoint 60 fps @ 16 chunks.
