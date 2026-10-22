# 0008 — Ed25519 keypair identity with trust-on-first-use
- Status: Accepted
- Date: 2026-10-22
- Confidence: 80%
- Decided by: Adrian (interview)

## Context
Friends connect via direct IP / dedicated servers. There's no account service to run,
but names must not be spoofable (ops, bans, allowlists, inventories are tied to identity).

## Decision
- Each client generates an **Ed25519 keypair** on first launch, stored in the user's
  data dir. The public key is the player's identity; the display name is a chosen label.
- Handshake: the server sends a nonce, the client signs it, and the server verifies. Servers
  remember `public key ↔ name` (first-come name claim), plus allowlist/ops/bans by key.
- Server identity: the client pins the server's certificate fingerprint on first connect
  (SSH-style known_hosts) and warns on change.
- Export/import of the identity key so a player can move machines.

## Consequences
- No infrastructure, no passwords, no spoofing.
- Losing the key means losing the identity on servers (mitigated by export/backup and op tools).
- Key storage and handshake are security-sensitive → `/security-review` at M6.

## Alternatives considered
- Username only: trivially spoofable.
- Central accounts: real infrastructure to build, host, and secure.
