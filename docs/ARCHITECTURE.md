# Architecture

Status: **M1 in progress**. The game opens a window and clears it on the GPU; most sections
are still *(planned)* and become *(implemented)* as tasks land. Keep this file true to the code.

## 1. Big picture

```
┌──────────────── terra (game binary) ────────────────┐      ┌── terra-server (binary) ──┐
│  terra-client  ──►  terra-render (wgpu)             │      │  headless, same server    │
│     │  input, UI, client world mirror, prediction   │      │  core as integrated       │
│     ▼                                               │      └────────────┬──────────────┘
│  terra-net  ◄── in-memory transport ──►  integrated server             │
└─────────────────────────────────────────┬──────────┘                   │
                                          │   QUIC (remote players)      │
                                          └──────────────────────────────┘
   Server core = terra-sim (tick, ECS, rules) + terra-world (blocks, light)
                 + terra-worldgen + terra-storage + terra-data (+ terra-mod)
```

**Single-player is multiplayer with one player.** The client always talks to a
server through `terra-net`. In single-player that server runs on background threads
inside the `terra` process over an in-memory transport, so there are no special cases and
multiplayer bugs show up early ([ADR 0002](adr/0002-client-server.md)).

The **server is authoritative**: it owns the world, simulation, and rules. The client
renders, predicts the local player's movement, and sends intents.

## 2. Crates

Versions here must match each `Cargo.toml` (`cargo xtask check-docs` enforces it).
Crates are created by the task that first needs them, not before. All crates live in
`crates/<name>/`; shared settings (edition, lints, profiles) are in the root `Cargo.toml`.

| Crate | Kind | Version | Status | Responsibility |
|---|---|---|---|---|
| `terra` | bin | 0.4.0 | runs the client; logging, crash reports, `profile-tracy` feature | Game binary. Wires client + integrated server. **Its version is the game version.** |
| `terra-core` | lib | 0.1.2 | `diagnostics` (logging, crash reports) | Shared foundation: coordinates, ids, math re-exports (glam), logging/tracing init, config, errors |
| `terra-xtask` | bin (internal) | 0.3.0 | `bump`, `check-docs`, `ci` (incl. `cargo deny check`), `wait-ci` | Automation: version bumps with cascade, doc-drift checks, all quality gates, waiting on GitHub CI; later asset tooling |
| `terra-render` | lib | 0.2.0 | GPU setup (adapter overrides, device-loss tracking), window surface (vsync, resize, size clamp, lost/outdated recovery, retry/wait), clear | wgpu renderer: device/surface, texture arrays, chunk meshes, shaders (WGSL), sky, LOD |
| `terra-client` | lib | 0.3.0 | window + event loop (waits instead of spinning while hidden), frame timing (`timing`), input state and cursor grab (`input`), sky-blue clear, `F9` vsync toggle | winit app, input, camera, UI/HUD, client world mirror, prediction/interpolation |
| `terra-world` | lib | — | planned (M2) | Block states, chunk columns/sections (palette storage), light, world access API |
| `terra-worldgen` | lib | — | planned (M2) | Noise, density terrain, biomes, caves, ores, features, structures. Deterministic per seed |
| `terra-data` | lib | — | planned (M2) | Registries + data-pack/resource-pack loading (blocks, items, recipes, textures, models) |
| `terra-mesh` | lib | — | planned (M2) | CPU chunk meshing (face culling → binary greedy), AO, packed vertices. No GPU deps, so it's benchable |
| `terra-net` | lib | — | planned (M3) | Protocol, packets, serialization, transports (in-memory now, QUIC in M6) |
| `terra-sim` | lib | — | planned (M3) | Server tick loop (20 TPS), ECS (bevy_ecs), physics/collision, player & game rules |
| `terra-storage` | lib | — | planned (M4) | World save format: region files, compression, versioning/migrations |
| `terra-server` | bin | — | planned (M6) | Dedicated headless server binary + config |
| `terra-audio` | lib | — | planned (M10) | Sound and music playback |
| `terra-mod` | lib | — | planned (M13) | WASM plugin host (wasmtime), mod API (WIT) |

Dependency direction (no cycles; lower layers never depend on higher ones):

```
terra-core ◄─ terra-data ◄─ terra-world ◄─ terra-worldgen
                              ▲    ▲
                 terra-mesh ──┘    └── terra-storage
terra-net ◄─ terra-sim (uses world, worldgen, storage, data)
terra-render ◄─ terra-client (uses net, mesh, world, data)
terra ─► terra-client + terra-sim      terra-server ─► terra-sim
```

## 3. Threads and scheduling *(main thread implemented; the rest planned)*

| Thread | Work |
|---|---|
| Main | winit event loop, input, client update, render submission (wgpu). *Implemented:* `terra_client::run()` owns it: the window is created in `resumed`, each `RedrawRequested` runs one frame and requests the next. A frame is: `FrameTimer::tick` (delta time, clamped to 250 ms; fps and avg/max frame time over the last second from a fixed 8192-sample ring buffer), update (reads `InputState`: physical keys and mouse buttons held / pressed / released this frame, raw `DeviceEvent::MouseMotion` delta), render, `InputState::end_frame`, `profiling::finish_frame!()`, with `update`/`render` profiling scopes. Window and device events only feed `InputState` between frames. A click grabs the cursor (`Locked`, else `Confined`; hidden) and isn't passed on; mouse motion counts only while grabbed; `Escape` and focus loss release it, and focus loss also releases every held key and button. Frame stats are logged at `debug` once a second until the overlay (T-0106). With nothing to draw into (minimized or hidden window, or acquiring fails twice in a row) it waits 100 ms (`ControlFlow::WaitUntil`) or for `Resized`/`Occluded(false)` instead of spinning; a timer, since winit reports no `Occluded` on Wayland. The timer is paused for that wait, so the next frame simulates no time and the gap isn't a frame time. A `GpuError` from the frame loop (device lost, surface can't be recreated) exits with that error |
| Integrated server | Fixed 20 TPS tick (50 ms budget): ECS systems, block updates, chunk tickets |
| Worker pool (rayon) | Worldgen, lighting, meshing, (de)compression, save I/O prep. Jobs are prioritized by distance to the player and cancellable |
| Net I/O (M6) | QUIC runtime (tokio, confined to `terra-net`) |
| Storage I/O | Region file reads/writes off the tick thread |

Threads communicate through channels and immutable snapshots (e.g. `Arc` section
data for meshing). There's no shared mutable world between client and server.

## 4. World model *(planned)* — see [ADR 0009](adr/0009-world-dimensions.md)

- **Coordinates:** `BlockPos(i32,i32,i32)`, `ChunkPos(i32,i32)` (16×16 column),
  `SectionPos(i32,i32,i32)` (16³ cube). World height Y −64…319 = 24 sections per column.
- **Block states:** registered at startup from data packs. A block state id (`u16`
  global, widen if mods demand) maps to properties (solid, opaque, light emission, model, …).
- **Section storage:** palette + bit-packed indices (1–8 bits/entry → direct `u16`),
  with a single-value fast path (all-air sections cost almost nothing).
- **Light:** per section, 4-bit sky light + 4-bit block light (nibble arrays),
  BFS propagation on the worker pool.
- **Chunk lifecycle:** `Requested → Generating → Lighting → Ready → (Meshing on client)`,
  driven by load tickets around players (render/simulation distance).

## 5. Rendering *(device and surface implemented; the rest planned)*

- wgpu (Vulkan on Linux, DX12/Vulkan on Windows), WGSL shaders, reverse-Z depth.
- *Implemented (T-0101):* `terra_render::Gpu` holds the instance, adapter, device and queue.
  The adapter is the highest-performance one wgpu offers (discrete before integrated,
  software last); `WGPU_BACKEND` limits the backends and `WGPU_ADAPTER_NAME` picks one by
  name (an error lists the alternatives). The adapter, backend, device type and driver are
  logged at `info`. wgpu validation errors keep wgpu's default handler, which panics, so
  they end in a crash report. A lost device is the exception: wgpu drops the errors it
  causes and calls the device-lost callback, which `Gpu::lost()` records. The client then
  stops with `GpuError::DeviceLost`. Recreating the device and every GPU resource is
  later work. `Gpu::headless()` serves GPU tests.
- *Implemented:* `WindowSurface` uses the first sRGB format and a present mode from
  `choose_present_mode(vsync, supported)` (on → `Fifo`; off → `Mailbox`, else `Immediate`,
  else `Fifo`). Its size is the window's, clamped to the device's
  `max_texture_dimension_2d` (8192 with default limits). `acquire` returns a frame,
  `Retry` or `Wait`. It reconfigures on resize (a 0×0 minimized window waits), on
  `Outdated` at the window's current size, and once after a `Suboptimal` frame (not again
  until the size changes). On `Lost` it recreates the surface and picks the format and
  present mode again. `Timeout` retries. A second failure in a row, or `Occluded`, waits.
  A `Validation` status means the device is lost (see above). On Wayland the client calls `pre_present_notify` only with
  vsync on: it aligns redraws to the compositor's frame callbacks, which would cap vsync-off
  frame rates.
- Block textures in a **2D texture array** (16×16, mipmapped, nearest filtering).
- Meshes are built on workers by `terra-mesh`. Vertices are packed (≈8 bytes/quad-vertex or
  vertex-pulling from a storage buffer), allocated in large shared GPU buffers,
  and drawn with indirect draws. Per-section frustum culling first, occlusion culling later.
- Opaque → cutout (leaves) → translucent (water/glass, sorted) passes.
- Later: LOD for far terrain (M12), shadows/reflections/volumetrics as quality tiers.

## 6. Networking *(planned)* — see [ADR 0007](adr/0007-own-protocol-and-formats.md), [ADR 0008](adr/0008-identity.md), [ADR 0011](adr/0011-transport-quic.md)

- Versioned binary protocol (protocol number checked at handshake).
- Transport trait with an in-memory implementation (M3) and QUIC (M6).
- Chunk data compressed (zstd/lz4, decided in-task) and streamed by priority.
- Client-side prediction for the local player, interpolation for other entities.

## 7. Content & modding *(planned)* — see [ADR 0006](adr/0006-modding.md)

- All content is namespaced (`terra:stone`). Vanilla content ships as the built-in
  `terra` data pack + resource pack. There's no hardcoded block list in code.
- Data packs: JSON. Config files (settings, server): TOML.

## 8. Diagnostics

- `tracing` everywhere. Binaries call `terra_core::diagnostics::init_logging()` first: one
  global subscriber on stderr, filter from `TERRA_LOG` (`EnvFilter` syntax, default `info`;
  invalid directives are dropped with one warning naming them, the valid ones still apply,
  and `info` is used if none is left), `pretty` format in debug builds,
  compact in release, colours only on a terminal. `log` events (wgpu, winit) are forwarded.
  Libraries never install subscribers or hooks.
- Crash reports: `install_crash_handler(version, dir)` sets a panic hook that writes
  `crash-<UTC time>.log` (time, game version, OS, thread, message, location, full
  backtrace) into `<data dir>/crash-reports/`, prints the path (or the write error) to
  stderr whatever the log filter, logs it too, then runs the default hook.
  It also runs before the abort in release builds (`panic = "abort"`). Data dir:
  etcetera's native app strategy for "Terra" (`$XDG_DATA_HOME/terra`, by default
  `~/.local/share/terra`, on Linux; `%APPDATA%\Terra\data` on Windows); saves and identity will live there too. Without a
  home directory, crash reports go to `./crash-reports/` (never a shared dir like `/tmp`).
- `profiling` crate scopes in hot paths (`profiling::scope!`, `#[profiling::function]`);
  they compile to nothing by default. `cargo run -p terra --features profile-tracy`
  streams them to [Tracy](https://github.com/wolfpld/tracy); `main` starts the Tracy client.
- F3-style debug overlay on the client (M1+).

## 9. Glossary

**Column/chunk**: 16×16×384 vertical slice. **Section**: 16³ cube within a column.
**Tick**: one 50 ms server simulation step. **Ticket**: a reason to keep a chunk
loaded. **Data pack**: content definitions (JSON). **Resource pack**: textures,
models, sounds. **Integrated server**: server running inside the game process.
