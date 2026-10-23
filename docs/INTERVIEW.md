# Interview log

Every interview with Adrian is recorded here, newest last. These answers are the
source of truth for preferences. When an answer changes a plan, update the
affected docs in the same commit.

---

## 2026-10-22 — Kickoff interview

**Vision.** *"I want to make a minecraft clone that I can play and give to friends
so we can play either local host multiplayer or if they were at home we can play
like if it was real minecraft but optimized the hell out of it."*

| Topic | Answer |
|---|---|
| Game modes | Creative, Survival, Multiplayer, Modding/scripting (all) |
| Architecture | Client/server from day one (integrated server for single-player) |
| Platforms | Linux first; Windows is later work |
| Engine | Custom engine on **wgpu** + winit |
| ECS / state | "You decide" → Claude chose bevy_ecs standalone ([ADR 0005](adr/0005-ecs.md)) |
| Modding | "The type native Minecraft has… I'll let you decide" → data packs + resource packs + WASM plugins ([ADR 0006](adr/0006-modding.md)) |
| MC compatibility | Own protocol & formats |
| Art | Minecraft-style 16×16 pixel art (original, no Mojang assets) |
| Graphics priorities | Smooth lighting/AO, huge render distance, shaders/fancy effects, day/night + weather (all) |
| Worldgen | Modern 1.18+ (Y −64…320, caves, biomes) |
| Gameplay reference | Modern Minecraft 1.20+ |
| Versioning | Per-crate SemVer **and** Conventional Commits driven; `terra` crate = game version; start at 0.0.0 |
| Git hosting | GitHub now, with CI. Public repo `apz98/terra` |
| License | MIT OR Apache-2.0 |
| Performance target | 60 fps @ 16 chunks on this laptop (Intel UHD iGPU, 8 GB RAM); <50 ms server tick; <2 GB RAM |
| Internet multiplayer | Direct IP + dedicated server (LAN discovery for local) |
| Identity/auth | Ed25519 keypair identity, trust-on-first-use |
| Resume command | `/next` |
| First milestone | "You decide" → M1 First Light (window, camera, one textured chunk), then M2 Terrain |
| Reviews | Milestone + risk based reminders for `/code-review` and `/security-review` |

**Working style (verbatim intent).** Claude does everything. Adrian asks "what's
next?" and Claude starts on it. Claude:
- provides a `/next` command to resume after `/clear`;
- reminds Adrian when to run a code or security review;
- treats this as a serious solo-dev project and works **in order**;
- always gives its best recommendation **with a confidence score**;
- keeps documentation always up to date;
- **interviews Adrian** whenever curious, uncertain, or in need of context.

## 2026-10-22 — Learning across `/clear`

**Adrian:** "I will use the clear slash command, so when things go wrong, or so that we
don't get caught with surprises, I would like you to write and document it in a way that
every time we run the clear command you get better and don't make the same mistakes again."

**Outcome:** Claude recommended a tracked lessons log (90%) and set it up:
`docs/LESSONS.md` (log + short Rules list, promoted to automated checks where possible),
read at the start of every session (CLAUDE.md, `/next`) and updated by a new **Retro**
step in the WORKFLOW task loop.

## 2026-10-22 — M0 wrap-up (T-0008)

| Question | Claude's recommendation | Adrian's answer |
|---|---|---|
| Q2: Where does the 16×16 art come from? | Procedural placeholders now, a CC0 pack or hand-drawn art before M11 (75%) | "Procedural now and convert that to real art assets" |
| Q1: Do your friends mostly play on Windows? | Move Windows before M11 if so (85%) | Mixed / not sure yet |

**Outcome.** Q2: the texture generator is a tool (`cargo xtask gen-textures`) whose output
is committed as PNG files under `assets/`. The game loads those files, and they're the art
from then on, so they can be edited or replaced one by one without touching code. The
generator won't overwrite them without `--force` (T-0104). Q1 stays open (OPEN_QUESTIONS),
to ask again before M6.
