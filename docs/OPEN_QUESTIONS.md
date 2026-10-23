# Open questions

Unresolved items that need Adrian's input (or a later technical decision).
Interview Adrian **before** a question blocks a task. When answered, log it in
`INTERVIEW.md`, update affected docs, and delete it here.

| # | Question | Needed by | Claude's current recommendation |
|---|---|---|---|
| Q1 | Do your friends mostly play on **Windows**? If so, Windows support (M14) should move **before** the Friends Release (M11). Asked 2026-10-22: "mixed / not sure yet". | Before M6 | Move Windows to just before M11 if most friends use Windows (85%); otherwise keep it last |
| Q3 | Game feel: exact Minecraft numbers (walk 4.317 m/s, jump 1.25 blocks, 20 TPS) or tuned to taste? | M4 | Match Minecraft numbers as defaults in a data file, tweakable later (80%) |
| Q4 | Should players be able to load their **own Minecraft resource packs** locally (format converter)? | M10 | Yes, local-only importer, never redistributed (70%) |

## Technical questions (Claude decides via ADR when reached)

- Transport: QUIC via `quinn` (proposed in [ADR 0011](adr/0011-transport-quic.md)), to confirm at M6.
- Serialization for protocol/saves (`bitcode` vs `postcard` vs hand-rolled): decide in M3/M4 with benchmarks.
- Chunk compression (zstd vs lz4) for network vs disk: decide with benchmarks.
- UI tech for menus/HUD (custom vs egui) → ADR during T-0106/M4.
- Job system: rayon vs custom priority pool → ADR in M2.
