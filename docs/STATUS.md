# Status — you are here

_Update this file in every commit that changes progress. Keep it short._

| | |
|---|---|
| **Game version** (`terra`) | 0.0.0 (docs baseline, tag `v0.0.0`) |
| **Milestone** | M0 — Foundations |
| **Last completed** | T-0005 CI on GitHub Actions (no bump) |
| **In progress** | — |
| **Next up** | T-0006 Diagnostics foundation in `terra-core` |
| **Blocked** | — |
| **Remote / CI** | https://github.com/apz98/terra (public) · CI: GitHub Actions runs `cargo xtask ci` on push/PR to `main` |

## Crate versions

| Crate | Version |
|---|---|
| `terra` | 0.0.0 |
| `terra-core` | 0.0.0 |
| `terra-xtask` | 0.1.1 |

`ARCHITECTURE.md` §2 is the authoritative crate table; `cargo xtask check-docs` checks both.

## Reviews

| Review | Last run | Due? |
|---|---|---|
| `/code-review` | 2026-10-22 (T-0004, `2eea2bc`): 10 findings, all fixed in T-0009 | **Due**: T-0009 rewrote ~1,200 lines of xtask (>500-line trigger); also at the end of M0 |
| `/security-review` | never | Not yet (first trigger: save files in M4, networking in M6) |

## Notes for the next session

- Open questions in `OPEN_QUESTIONS.md`: Q1 (Windows timing) and Q2 (art source) should be
  asked before M1 texture work; neither blocks M0.
