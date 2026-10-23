# Status — you are here

_Update this file in every commit that changes progress. Keep it short._

| | |
|---|---|
| **Game version** (`terra`) | 0.1.0 (tag `v0.1.0`) |
| **Milestone** | M1 — First Light (M0 done in 0.1.0) |
| **Last completed** | T-0008 M0 wrap-up (no bump) |
| **In progress** | — |
| **Next up** | T-0109 Fix `/code-review` findings on M0, then T-0108 `cargo-deny`, then T-0101 window + GPU surface |
| **Blocked** | — |
| **Remote / CI** | https://github.com/apz98/terra (public) · CI: GitHub Actions (`ubuntu-24.04`) runs `cargo xtask ci` on push/PR to `main`; wait on it with `cargo xtask wait-ci` |

## Crate versions

| Crate | Version |
|---|---|
| `terra` | 0.1.0 |
| `terra-core` | 0.1.0 |
| `terra-xtask` | 0.2.0 |

`ARCHITECTURE.md` §2 is the authoritative crate table; `cargo xtask check-docs` checks both.

## Reviews

| Review | Last run | Due? |
|---|---|---|
| `/code-review` | 2026-10-22 (end of M0, `2b2725b..bc4ab5c`): 8 findings → T-0109 | Not due (next: end of M1, or after a >500-line task) |
| `/security-review` | never | Not yet (first trigger: save files in M4, networking in M6) |

## Notes for the next session

- Q2 answered (procedural textures baked to PNG assets → T-0104). Q1 (Windows timing) is
  still open: Adrian's friends are mixed / unknown, so ask again before M6.
- T-0108 (`cargo-deny`) was put ahead of T-0101 so licenses are checked before wgpu/winit
  arrive. To undo, move it after T-0107 in TASKS.md.
