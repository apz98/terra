# 0006 — Modding: data packs + resource packs + WASM plugins
- Status: Accepted
- Date: 2026-10-22
- Confidence: 80%
- Decided by: Claude (delegated: "the type native Minecraft has… I'll let you decide")

## Context
Minecraft's modding has two layers: **data-driven** (data packs: recipes, loot,
worldgen, tags; resource packs: textures, models, sounds, lang) and **code mods**
(Forge/Fabric on the JVM, Bukkit/Paper plugins server-side). Terra needs the same
power without the JVM, and safe to share among friends.

## Decision
1. **Everything is content from day one.** Blocks, items, recipes, etc. are namespaced
   (`terra:stone`) and loaded from data. The vanilla game ships as the built-in
   `terra` data pack + resource pack. No hardcoded block lists in code.
2. **Data packs** (JSON) and **resource packs** (PNG/JSON/OGG) with a Minecraft-like
   folder layout, loaded by `terra-data`. Servers can push required packs to clients.
3. **Code mods** are **WebAssembly components** run by `wasmtime` in `terra-mod`, with a
   versioned WIT interface (events, commands, entity/block APIs). They're sandboxed with
   fuel/epoch limits and no filesystem/network unless granted. Server-side first
   (Paper-like), client-side mods later.

## Consequences
- Early constraint: registries and data loading exist from M2, which costs some upfront work.
- Mods can be written in any language that targets WASM (Rust, C, Zig, AssemblyScript, …).
- WASM plugins are safe to download from friends; the host API must be designed carefully.
- `wasmtime` is heavy, so it sits behind a cargo feature and is only linked where needed.
- Mod loading is a security surface → `/security-review` required in M13.

## Alternatives considered
- **Lua (mlua)**: easy scripting, weaker sandbox, single language (~55% alt).
- **Native dylib mods**: maximum power, unsafe, ABI-fragile.
- **Data-only**: too limited for "real" mods.
