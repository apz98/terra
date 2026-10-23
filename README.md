# Terra

[![CI](https://github.com/apz98/terra/actions/workflows/ci.yml/badge.svg)](https://github.com/apz98/terra/actions/workflows/ci.yml)

A Minecraft-style voxel sandbox written in Rust: creative, survival, and multiplayer
with friends over LAN or the internet, built on a custom wgpu engine and optimized hard.

> Repository: https://github.com/apz98/terra
>
> **Status:** M0 Foundations, workspace scaffolded (game version `0.0.0`). See
> [`docs/STATUS.md`](docs/STATUS.md) and the [roadmap](docs/ROADMAP.md).

## Goals

- Feels like modern Minecraft (1.20+ gameplay, 1.18+ tall worlds, 16×16 pixel art)
- Multiplayer first: single-player runs an integrated server; friends join via direct IP / dedicated server
- 60 fps at 16-chunk render distance on an Intel iGPU laptop, with LOD for more
- Moddable: data packs, resource packs, sandboxed WASM plugins
- Linux first, Windows later

More in [`docs/VISION.md`](docs/VISION.md).

## Building

Install Rust with [rustup](https://rustup.rs). The toolchain is pinned in
`rust-toolchain.toml` (Rust 1.99.1 with rustfmt and clippy) and rustup installs it
automatically on the first build.

```sh
cargo run -p terra                 # run the game (currently prints its version)
cargo build --workspace            # build everything
cargo test --workspace             # run all tests
cargo xtask ci                     # every gate: fmt, clippy, tests, rustdoc, check-docs (CI runs the same)
cargo xtask check-docs             # docs agree with the code (versions, tasks, changelog)
cargo xtask bump terra-core minor --dry-run   # version bump with cascade (docs/VERSIONING.md)
```

Dev builds optimize dependencies (`opt-level = 3`) and Terra's own code lightly
(`opt-level = 1`) so the game stays playable. Release builds use fat LTO.
System packages for the graphics stack are documented in T-0007 (a Vulkan-capable GPU driver is required).

## Documentation

| Doc | What it's for |
|---|---|
| [STATUS](docs/STATUS.md) | Where the project is right now |
| [LESSONS](docs/LESSONS.md) | Mistakes made once, turned into rules |
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
