# Terra

[![CI](https://github.com/apz98/terra/actions/workflows/ci.yml/badge.svg)](https://github.com/apz98/terra/actions/workflows/ci.yml)

A Minecraft-style voxel sandbox written in Rust: creative, survival, and multiplayer
with friends over LAN or the internet, built on a custom wgpu engine and optimized hard.

> Repository: https://github.com/apz98/terra
>
> **Status:** M1 First Light, in progress. See [`docs/STATUS.md`](docs/STATUS.md) for the
> current game version and the [roadmap](docs/ROADMAP.md).

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

`cargo xtask ci` also needs [cargo-deny](https://github.com/EmbarkStudios/cargo-deny),
which checks dependency licenses, security advisories and sources against `deny.toml`.
Install the version CI uses:

```sh
cargo install --locked cargo-deny@0.20.2
```

```sh
cargo run -p terra                 # run the game (currently prints its version)
cargo run -p terra --features profile-tracy   # same, streaming profiling scopes to Tracy
cargo build --workspace            # build everything
cargo test --workspace             # run all tests
cargo xtask ci                     # every gate: check-docs, fmt, cargo-deny, clippy (+ profile-tracy), tests, rustdoc (CI runs the same)
cargo deny check                   # just the license / advisory / source checks (deny.toml)
cargo xtask check-docs             # docs agree with the code (versions, tasks, changelog)
cargo xtask bump terra-core minor --dry-run   # version bump with cascade (docs/VERSIONING.md)
cargo xtask wait-ci                # after a push: wait for GitHub Actions on HEAD (needs `gh`)
```

Dev builds optimize dependencies (`opt-level = 3`) and Terra's own code lightly
(`opt-level = 1`) so the game stays playable. Release builds use fat LTO.
The `profile-tracy` feature also needs a C++ compiler.

### Building on Linux

Terra renders with Vulkan, so you need a GPU driver with Vulkan support. The windowing
and graphics libraries are loaded when the game starts, not linked at build time. That
means building only needs a C toolchain and `pkg-config`; the rest is needed to run it.

**Arch Linux** (and Arch-based distros such as Omarchy):

```sh
sudo pacman -S --needed base-devel pkgconf vulkan-icd-loader \
    wayland libxkbcommon libxkbcommon-x11 libx11 libxcursor libxrandr libxi vulkan-tools
# plus the Vulkan driver for your GPU, one of:
sudo pacman -S --needed vulkan-intel     # Intel
sudo pacman -S --needed vulkan-radeon    # AMD
sudo pacman -S --needed nvidia-utils     # NVIDIA (proprietary driver)
```

**Ubuntu 24.04 / Debian-based:** the list lives in
[`.github/ubuntu-packages.txt`](.github/ubuntu-packages.txt), with a comment per group.
CI installs exactly that file, so it can't go stale:

```sh
sed 's/#.*//' .github/ubuntu-packages.txt | xargs sudo apt-get install -y --no-install-recommends
```

**Check Vulkan:** `vulkaninfo --summary` should list your GPU under `Devices:`. If it
lists only `llvmpipe`, Vulkan found no hardware driver and Terra will run on the CPU
(very slowly).

**Linking:** Rust already links with its bundled `rust-lld` on x86-64 Linux, which is
fast. `mold` isn't needed for now.

### Environment variables

| Variable | Effect |
|---|---|
| `TERRA_LOG` | Log filter in [`tracing` `EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) syntax, e.g. `debug` or `info,terra_core=trace`. Default `info`. Logs go to stderr. |

If Terra crashes, it writes a crash report to `~/.local/share/terra/crash-reports/`
(Linux; `%APPDATA%\Terra\data\crash-reports\` on Windows) and prints the file's path.

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
