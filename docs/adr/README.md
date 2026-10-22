# Architecture Decision Records

One file per significant decision: `NNNN-short-title.md`. Once **Accepted**, an ADR
is not rewritten. A later ADR supersedes it (and the old one gets
`Status: Superseded by NNNN`). Every ADR carries a **confidence** score.

Template:

```markdown
# NNNN — Title
- Status: Proposed | Accepted | Superseded by NNNN
- Date: YYYY-MM-DD
- Confidence: NN%
- Decided by: Adrian (interview) | Claude (delegated) | both

## Context
## Decision
## Consequences
## Alternatives considered
```

| # | Title | Status | Confidence |
|---|---|---|---|
| [0001](0001-custom-engine-wgpu.md) | Custom engine on wgpu + winit | Accepted | 85% |
| [0002](0002-client-server.md) | Client/server architecture from day one | Accepted | 90% |
| [0003](0003-crate-layout.md) | Workspace and `terra-*` crate layout | Accepted | 80% |
| [0004](0004-versioning.md) | Per-crate SemVer driven by Conventional Commits | Accepted | 85% |
| [0005](0005-ecs.md) | `bevy_ecs` (standalone) for entities; custom storage for blocks | Accepted | 70% |
| [0006](0006-modding.md) | Modding: data packs + resource packs + WASM plugins | Accepted | 80% |
| [0007](0007-own-protocol-and-formats.md) | Own network protocol and save format | Accepted | 90% |
| [0008](0008-identity.md) | Ed25519 keypair identity with trust-on-first-use | Accepted | 80% |
| [0009](0009-world-dimensions.md) | 16×16 columns, 16³ sections, Y −64…319 | Accepted | 90% |
| [0010](0010-license-and-assets.md) | MIT OR Apache-2.0; original/CC0 assets only | Accepted | 95% |
| [0011](0011-transport-quic.md) | QUIC (`quinn`) transport for remote play | Proposed | 75% |
