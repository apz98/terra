# 0010 — MIT OR Apache-2.0; original/CC0 assets only
- Status: Accepted
- Date: 2026-10-22
- Confidence: 95%
- Decided by: Adrian (license), Claude (asset policy)

## Context
Terra is public on GitHub and will be shared with friends. It imitates Minecraft's
look and gameplay, which is fine for mechanics and style, but Mojang's assets and code are copyrighted.

## Decision
- Code: dual-licensed **MIT OR Apache-2.0** (`LICENSE-MIT`, `LICENSE-APACHE`).
- Assets in the repo: **original works or CC0/permissive** with attribution in
  `assets/CREDITS.md` (created with the first asset).
- Never commit Mojang textures, sounds, models, code, or decompiled material.
  Don't name the project or binaries "Minecraft".
- Players may load their own resource packs locally (M10); Terra never redistributes them.
- Dependencies must have compatible licenses (checked with `cargo-deny`, added later).

## Consequences
- Art must be produced or sourced (see OPEN_QUESTIONS Q2).
- Clean legal footing for sharing builds publicly.
