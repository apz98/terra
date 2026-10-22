# Terra — Vision

> A Minecraft clone I can play and give to friends, local or over the internet,
> that feels like the real thing but is optimized to the hilt.
> — Adrian, interview 2026-10-22

## Pillars

1. **Feels like Minecraft.** Modern (1.20+) gameplay as the long-term reference:
   blocks, mining, crafting, survival, mobs, 1.18-style tall worlds with caves and
   biomes, 16×16 pixel-art look. Players who know Minecraft should feel at home.
2. **Play together.** Multiplayer is a first-class citizen. Single-player runs an
   integrated server, and friends join over LAN or the internet (direct IP / dedicated server).
3. **Optimized to the hilt.** Runs at 60 fps with 16-chunk render distance on a modest
   Intel iGPU laptop, then scales up with LOD to huge view distances. Performance
   budgets are enforced, not hoped for (see `PERFORMANCE.md`).
4. **Moddable like Minecraft.** Vanilla content is itself a data pack. Data packs,
   resource packs, and sandboxed WASM plugins let anyone extend the game ([ADR 0006](adr/0006-modding.md)).
5. **Built to last.** Clean crate boundaries, docs synced with code, ADRs for
   decisions, and versioned saves and protocol with migrations.

## In scope (eventually)

- Game modes: **Creative**, **Survival**, **Multiplayer**, **Modding/scripting**
- Graphics: smooth lighting + AO, huge render distance (LOD), optional fancy
  shaders (shadows, reflections, volumetric fog), day/night, weather
- Worldgen: modern 1.18+ style, Y −64…320, 3D noise terrain, caves, aquifers,
  biomes, ores, vegetation, structures
- Platforms: **Linux first**, **Windows later**

## Non-goals

- Compatibility with Minecraft's network protocol or vanilla clients/servers.
- Using or shipping any Mojang assets or code. (Players *may* load their own
  resource packs locally, which is a later feature.)
- Web/WASM client and macOS (not planned; revisit only if Adrian asks).
- Central account service. Identity is keypair-based ([ADR 0008](adr/0008-identity.md)).
- Monetization, launchers, marketplaces.

## What "done" means for 1.0.0 (Friends Release)

Adrian can hand a Linux build to friends; they can play survival together on a
dedicated server or a LAN world: generate a modern world, mine, craft, fight mobs,
build, save and reload, with stable performance and no known data-loss bugs.
