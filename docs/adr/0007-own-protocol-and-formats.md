# 0007 — Own network protocol and save format
- Status: Accepted
- Date: 2026-10-22
- Confidence: 90%
- Decided by: Adrian (interview)

## Context
Compatibility with Minecraft's Java protocol or Anvil world format would lock Terra
into Mojang's design choices and impose huge scope.

## Decision
- Terra uses its **own binary network protocol**, versioned by a protocol number
  checked at handshake. Terra clients only connect to Terra servers.
- Terra uses its **own world save format** (region-style files, per-chunk compression,
  a format version per chunk enabling migrations), implemented in `terra-storage` (M4).
- Serialization and compression choices are decided with benchmarks in M3/M4 (see OPEN_QUESTIONS).

## Consequences
- Freedom to design for performance (bit-packed sections, delta updates, priorities).
- No vanilla client/server interop, and no Minecraft world import (could be a future converter).
- Save files and packets parsed from untrusted sources need strict bounds checks and fuzzing.

## Alternatives considered
- Java protocol compatibility: enormous scope, constrains every design decision.
- Anvil import: possible later as an offline converter; not planned.
