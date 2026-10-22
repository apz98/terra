# Performance

"Optimized the hell out of it" is a pillar, so performance is designed in, measured,
and protected from regressions.

## Reference machine (Adrian's laptop)

| | |
|---|---|
| CPU | 8 threads (Intel Raptor Lake-P) |
| GPU | Intel UHD Graphics (RPL-U), Mesa 26.2 ANV, Vulkan 1.4 |
| RAM | 7.4 GiB |
| OS | Linux (Omarchy/Arch), Wayland |

If it runs well here, it runs well on friends' machines.

## Targets (agreed 2026-10-22)

| Metric | Target on reference machine |
|---|---|
| Client frame rate | **60 fps sustained** at **16-chunk** render distance (vsync off, 1080p) |
| Frame time p99 | ≤ 20 ms (no hitches when loading chunks) |
| Server tick | ≤ 50 ms worst-case at 20 TPS; ≤ 10 ms median single-player |
| Memory | ≤ 2 GB total process RSS at 16 chunks |
| Chunk throughput | New terrain keeps up with flying at sprint speed at 16 chunks |
| Startup to in-world | ≤ 5 s for a new single-player world |

LOD (M12) extends visible distance far beyond 16 chunks within the same frame budget.

## Budgets per subsystem (initial guesses, refine with data)

| Subsystem | Budget |
|---|---|
| Mesh one 16³ section (typical terrain) | ≤ 100 µs single-threaded |
| Generate one column (M2 heightmap) | ≤ 2 ms; (M7 full 3D) ≤ 10 ms |
| GPU memory per visible section | ≤ 64 KiB average |
| Draw calls | O(1) via indirect / multi-draw, not O(sections) |

## Practices

- Hot paths (meshing, worldgen, lighting, serialization, section access) have
  **criterion benchmarks** in their crate's `benches/`.
- Profile before optimizing (Tracy via `--features profile-tracy`).
- Dev profile compiles dependencies with `opt-level = 3`, so debug builds stay playable.
- Release: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` for the game binaries.
- Avoid per-frame allocations in the render loop; reuse buffers; prefer SoA and
  bit-packing for world data.
- Any regression > 5% on a tracked benchmark needs a fix or an ADR explaining it.

## Benchmark log

Record numbers when a milestone completes or a hot path changes.

| Date | Game version | Benchmark / scenario | Result | Notes |
|---|---|---|---|---|
| — | — | — | — | No code yet |
