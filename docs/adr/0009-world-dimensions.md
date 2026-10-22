# 0009 — 16×16 columns, 16³ sections, Y −64…319
- Status: Accepted
- Date: 2026-10-22
- Confidence: 90%
- Decided by: Adrian (modern 1.18+ worldgen), Claude (storage layout)

## Context
Adrian wants modern Minecraft-style worlds (1.18+): tall terrain, deep caves.

## Decision
- Horizontal: infinite, chunk columns of 16×16 blocks (`ChunkPos`).
- Vertical: fixed **Y −64…319** (384 blocks) = 24 sections of 16³ per column.
- Storage per section: block-state palette + bit-packed indices, with a single-value
  fast path; light as two 4-bit nibble arrays (sky, block).
- Height bounds are constants in `terra-core`, used everywhere (no magic numbers), so
  they could become per-dimension later.

## Consequences
- Matches player expectations and the 1.18 worldgen design.
- Column-based loading keeps sky light and heightmaps simple compared to cubic chunks.
- Fixed height limits extreme builds. Acceptable (matches Minecraft).

## Alternatives considered
- Classic 0…255: smaller, but not what Adrian wants.
- Cubic chunks (infinite height): flexible, much harder lighting/worldgen/streaming.
