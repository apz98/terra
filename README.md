# Terra

A Minecraft-style voxel sandbox written in Rust: creative, survival, and multiplayer
with friends over LAN or the internet, built on a custom wgpu engine and optimized hard.

> Repository: https://github.com/apz98/terra
>
> **Status:** pre-code. Documentation baseline `v0.0.0`. See [`docs/STATUS.md`](docs/STATUS.md)
> and the [roadmap](docs/ROADMAP.md).

## Goals

- Feels like modern Minecraft (1.20+ gameplay, 1.18+ tall worlds, 16×16 pixel art)
- Multiplayer first: single-player runs an integrated server; friends join via direct IP / dedicated server
- 60 fps at 16-chunk render distance on an Intel iGPU laptop, with LOD for more
- Moddable: data packs, resource packs, sandboxed WASM plugins
- Linux first, Windows later

More in [`docs/VISION.md`](docs/VISION.md).

## Building

_Coming in M0 (T-0003, T-0007)._ Requires the Rust toolchain pinned in
`rust-toolchain.toml` and a Vulkan-capable GPU driver.

## Documentation

| Doc | What it's for |
|---|---|
| [STATUS](docs/STATUS.md) | Where the project is right now |
| [ROADMAP](docs/ROADMAP.md) | Milestones M0–M14 |
| [TASKS](docs/TASKS.md) | Ordered, actionable tasks |
| [ARCHITECTURE](docs/ARCHITECTURE.md) | Crates, threads, data model |
| [PERFORMANCE](docs/PERFORMANCE.md) | Targets, budgets, benchmark log |
| [WORKFLOW](docs/WORKFLOW.md) | How development is done, step by step |
| [VERSIONING](docs/VERSIONING.md) | Per-crate SemVer + Conventional Commits |
| [ADRs](docs/adr/README.md) | Architecture decisions |

Development is driven by Claude Code: run `/next` to resume work (see [`CLAUDE.md`](CLAUDE.md)).

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT)
at your option. Terra is not affiliated with Mojang or Microsoft and contains no Minecraft assets.
