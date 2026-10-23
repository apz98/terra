# Status — you are here

_Update this file in every commit that changes progress. Keep it short._

| | |
|---|---|
| **Game version** (`terra`) | 0.0.0 (docs baseline, tag `v0.0.0`) |
| **Milestone** | M0 — Foundations |
| **Last completed** | T-0004 xtask: `bump`, `check-docs`, `ci` (terra-xtask 0.1.0) |
| **In progress** | — |
| **Next up** | T-0009 Fix `/code-review` findings on T-0004, then T-0005 CI |
| **Blocked** | — |
| **Remote / CI** | https://github.com/apz98/terra (public) · no CI yet |

## Crate versions

| Crate | Version |
|---|---|
| `terra` | 0.0.0 |
| `terra-core` | 0.0.0 |
| `terra-xtask` | 0.1.0 |

`ARCHITECTURE.md` §2 is the authoritative crate table; `cargo xtask check-docs` checks both.

## Reviews

| Review | Last run | Due? |
|---|---|---|
| `/code-review` | 2026-10-22 (T-0004, `2eea2bc`): 10 findings → T-0009 | Next at the end of M0 |
| `/security-review` | never | Not yet (first trigger: save files in M4, networking in M6) |

## Notes for the next session

- Open questions in `OPEN_QUESTIONS.md`: Q1 (Windows timing) and Q2 (art source) should be
  asked before M1 texture work; neither blocks M0.
