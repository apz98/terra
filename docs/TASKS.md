# Tasks

Ordered. Take the **first unblocked `todo`**. Status: `todo` · `in progress` ·
`blocked (reason)` · `done (version, commit)`. IDs: `T-<milestone><nn>` (M0 → T-00nn,
M1 → T-01nn …). Finished milestones move to `docs/tasks/archive/` (M0: `archive/M0.md`).

Each task lists the crates it touches, its acceptance criteria (AC), and the docs to update.

---

## M1 — First Light

Goal and exit criteria: `ROADMAP.md` M1. A window showing a textured 16³ section you can
fly around, ≥ 60 fps on the reference machine, baseline numbers recorded.

### T-0108 License and advisory checks (`cargo-deny`) — `todo`
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

### T-0101 Window and GPU surface (`terra-client`, `terra-render`) — `todo`
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
