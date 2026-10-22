# 0002 — Client/server architecture from day one
- Status: Accepted
- Date: 2026-10-22
- Confidence: 90%
- Decided by: Adrian (interview)

## Context
Multiplayer (LAN and internet) is a core pillar. Retrofitting networking onto a
single-player engine is a well-known rewrite trap (Minecraft itself did this in 1.3).

## Decision
The server is authoritative and always separate from the client. Single-player runs an
**integrated server** on background threads in the `terra` process, connected through an
**in-memory transport** that implements the same transport trait as the network transport.
The dedicated `terra-server` binary runs the same server core headless.

The split lands in **M3**, right after basic terrain rendering (M1–M2), before gameplay
systems accumulate.

## Consequences
- One code path for single- and multiplayer; networking bugs surface early.
- Client needs prediction (local player) and interpolation (others) from M3/M6.
- Slight latency/complexity overhead in single-player (negligible with in-memory transport).
- M1–M2 render a locally generated world. M3 moves generation behind the server.

## Alternatives considered
- Single-player first, multiplayer later: simpler start, near-certain painful refactor.
