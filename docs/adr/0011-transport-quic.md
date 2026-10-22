# 0011 — QUIC (`quinn`) transport for remote play
- Status: **Proposed**, to be confirmed or revised when M6 starts
- Date: 2026-10-22
- Confidence: 75%
- Decided by: Claude (proposal)

## Context
Remote multiplayer needs reliable ordered streams (chunks, inventory, chat), unreliable
datagrams (frequent movement updates), encryption, and resilience to packet loss,
over direct IP connections to friends' servers.

## Proposal
Use **QUIC** via the `quinn` crate:
- Multiple independent streams avoid head-of-line blocking (a big chunk won't stall movement).
- Unreliable datagrams for high-frequency state.
- TLS 1.3 built in; self-signed server certs pinned by fingerprint ([ADR 0008](0008-identity.md)).
- UDP-based, which keeps NAT traversal ("Open to friends") possible later.
- tokio stays confined to `terra-net`.

## Open points
- Benchmark against a simpler TCP+UDP design on lossy links before accepting.
- LAN discovery mechanism (UDP broadcast/multicast) is separate from the transport.

## Alternatives considered
- TCP + custom UDP: more code, home-made encryption/reliability.
- `renet`/`laminar`-style game UDP libs: smaller, less mature encryption/stream story.
