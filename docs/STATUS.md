# Status — you are here

_Update this file in every commit that changes progress. Keep it short._

| | |
|---|---|
| **Game version** (`terra`) | 0.1.0 (tag `v0.1.0`) |
| **Milestone** | M0 — Foundations |
| **Last completed** | T-0007 Linux build prerequisites (no bump) |
| **In progress** | — |
| **Next up** | T-0008 M0 wrap-up |
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
| `/code-review` | 2026-10-22 (T-0009 + T-0005, `b3d3dff..2b2725b`): 9 findings → T-0010 | Not due (next: end of M0, or after a >500-line task) |
| `/security-review` | never | Not yet (first trigger: save files in M4, networking in M6) |

## Notes for the next session

- Open questions in `OPEN_QUESTIONS.md`: Q1 (Windows timing) and Q2 (art source) should be
  asked before M1 texture work; neither blocks M0.
