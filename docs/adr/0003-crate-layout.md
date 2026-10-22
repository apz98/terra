# 0003 — Workspace and `terra-*` crate layout
- Status: Accepted
- Date: 2026-10-22
- Confidence: 80%
- Decided by: Adrian (crate naming `terra`, `terra-*`), Claude (the split)

## Context
Adrian wants crates named `terra` and `terra-*`. A good split keeps compile times low
(parallel, incremental), enforces layering, and makes hot paths benchmarkable without a GPU.

## Decision
A Cargo workspace with crates under `crates/<name>/`. The full planned list and
dependency direction live in `ARCHITECTURE.md` §2. Key rules:
- `terra` is the game binary and its version is the game version. `terra-server` is the dedicated server.
- `terra-core` is the bottom layer. No crate depends on a higher layer; no cycles.
- GPU code is confined to `terra-render`. Meshing (`terra-mesh`) and worldgen are pure CPU
  crates so they can be tested and benchmarked headless.
- Async runtime (tokio) is confined to `terra-net`.
- Crates are created by the task that first needs them, not as empty placeholders.
- Internal tooling lives in `terra-xtask` (invoked as `cargo xtask`).

## Consequences
- More `Cargo.toml` files and per-crate versions to maintain (automated by `cargo xtask bump`).
- Clear ownership boundaries for docs and reviews.

## Alternatives considered
- Single crate with modules: simpler, but slow incremental builds and no enforced layering.
- Finer split (e.g. separate `terra-ui`, `terra-physics`): deferred until a module earns it.
