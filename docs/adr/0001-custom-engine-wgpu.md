# 0001 — Custom engine on wgpu + winit
- Status: Accepted
- Date: 2026-10-22
- Confidence: 85%
- Decided by: Adrian (interview)

## Context
Terra must be heavily optimized for voxel rendering (60 fps @ 16 chunks on an Intel iGPU)
and run on Linux now and Windows later. General-purpose engines aren't built around
voxel workloads (millions of quads, streaming chunk meshes, custom culling/LOD).

## Decision
Build our own engine on **wgpu** (graphics abstraction: Vulkan on Linux, DX12/Vulkan on
Windows) and **winit** (windowing/input). Shaders are written in WGSL. Math uses `glam`.

## Consequences
- Full control of the voxel pipeline: vertex packing, buffer suballocation, indirect draws, LOD.
- More foundational work up front (window, input, UI, asset loading).
- wgpu limits some low-level tricks (e.g. no mesh shaders yet). Acceptable for our targets.
- We must track wgpu/winit releases (breaking changes are common). Upgrades are deliberate tasks.

## Alternatives considered
- **Bevy**: fast start, but voxel rendering fights its renderer, heavy builds, frequent API churn.
- **Raw Vulkan (ash)**: highest ceiling, far more code, and Windows/driver burden. wgpu is enough.
