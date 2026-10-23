# Status — you are here

_Update this file in every commit that changes progress. Keep it short._

| | |
|---|---|
| **Game version** (`terra`) | 0.2.0 (tag `v0.2.0`) |
| **Milestone** | M1 — First Light (M0 done in 0.1.0) |
| **Last completed** | T-0101 Window and GPU surface (0.2.0) |
| **In progress** | — |
| **Next up** | T-0110 fix `/code-review` findings on T-0101, then T-0102 frame loop, timing and input |
| **Blocked** | — |
| **Remote / CI** | https://github.com/apz98/terra (public) · CI: GitHub Actions (`ubuntu-24.04`) runs `cargo xtask ci` on push/PR to `main`; wait on it with `cargo xtask wait-ci` |

## Crate versions

| Crate | Version |
|---|---|
| `terra` | 0.2.0 |
| `terra-core` | 0.1.2 |
| `terra-render` | 0.1.0 |
| `terra-client` | 0.1.0 |
| `terra-xtask` | 0.3.0 |

`ARCHITECTURE.md` §2 is the authoritative crate table; `cargo xtask check-docs` checks both.

## Reviews

| Review | Last run | Due? |
|---|---|---|
| `/code-review` | 2026-10-23 (T-0101, `811d5b2..ec594f2`): 10 findings → T-0110 (7 to fix, 3 no change). Before: 2026-10-22 (end of M0): 8 → T-0109 | Next: end of M1 |
| `/security-review` | never | Not yet (first trigger: save files in M4, networking in M6) |

## Notes for the next session

- Q2 answered (procedural textures baked to PNG assets → T-0104). Q1 (Windows timing) is
  still open: Adrian's friends are mixed / unknown, so ask again before M6.
- `cargo xtask ci` needs cargo-deny (`cargo install --locked cargo-deny@0.20.2`). A new
  license or advisory needs an entry with a reason in `deny.toml`, not a silent allow.
- Visual checks on the reference machine (Hyprland): see the T-0101 entry in LESSONS for
  driving the window with `hyprctl` and checking pixels with `grim`.
