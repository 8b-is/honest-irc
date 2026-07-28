# honest-irc — Mesh Architecture (Mullvad + Tailnet)

## Sidecar Design

```
┌──────────────────────────────────────────────────────────────────┐
│                        honest-irc node                            │
│                                                                   │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────────────┐  │
│  │ honest-vpn   │──▶│ honest-crypt │──▶│ honest-mesh          │  │
│  │ (mullvad)    │   │ (kyber+xy)   │   │ (tailscale sidecar)  │  │
│  │              │   │              │   │                      │  │
│  │ double-hop   │   │ encrypt all  │   │ mesh routing         │  │
│  │ entry→exit   │   │ pre-tailnet  │   │ peer discovery       │  │
│  └──────────────┘   └──────────────┘   └──────────┬───────────┘  │
│                                                     │             │
│  ┌──────────────────────────────────────────────────┼───────────┐ │
│  │                    honest-ircd                   │           │ │
│  │                                                  │           │ │
│  │  ┌────────┐  ┌────────┐  ┌──────────┐           │           │ │
│  │  │ irc     │  │honesty │  │ music    │           │           │ │
│  │  │protocol│  │auth    │  │.vaked.dev│           │           │ │
│  │  └────────┘  └────────┘  └──────────┘           │           │ │
│  └─────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

## Sidecar Chain (CLI binaries)

Each sidecar is a standalone CLI binary, chained via stdin/stdout or Unix sockets:

### 1. `honest-vpn` — Mullvad double-hop
```
Usage: honest-vpn --entry <country> --exit <country>

Connects to Mullvad VPN with a two-hop chain:
  Entry node: First Mullvad server (obfuscates origin)
  Exit node:  Second Mullvad server (appears as source to Tailnet)

Protocol: WireGuard with Mullvad's quantum-resistant tunnel option
All traffic from subsequent sidecars is routed through this tunnel.
The entry/exit hop configuration is persisted and rotated every 4 hours.
```

### 2. `honest-crypt` — Quantum-proof encryption (pre-Tailnet)
```
Usage: honest-crypt --mode kyber-x25519 --peer <route_id>

Applies CRYSTALS-Kyber-1024 + X25519 hybrid encryption to ALL traffic
before it reaches the Tailnet mesh. This ensures:

- End-to-end quantum resistance even if Tailscale's own crypto is compromised
- Double encryption: honest-crypt layer + Tailscale's WireGuard
- The Tailnet sees only encrypted ciphertext, never plaintext

Key exchange: Kyber-1024 KEM (post-quantum) + X25519 ECDH (classical)
Symmetric: ChaCha20Poly1305 (for messages), AES-256-GCM (for bulk)
Signatures: CRYSTALS-Dilithium-5 (post-quantum) + Ed25519 (classical)
Forward secrecy: New Kyber ephemeral key per session, rotated every hour
```

### 3. `honest-mesh` — Tailscale sidecar
```
Usage: honest-mesh --tailnet <name> --authkey <key>

Manages Tailscale/Headscale mesh networking:
- Joins the specified tailnet
- Discovers peers via Tailscale's coordination server (or Headscale self-hosted)
- Maintains the mesh topology
- Routes encrypted packets between peers
- Falls back to DERP relay when direct connections fail
```

### 4. `honest-ircd` — The IRC daemon
```
Usage: honest-ircd --identity <path/to/honesty-vector.json>

The main daemon:
- Loads honesty vector for identity
- Handles IRC-like protocol (/msg, /room, /music, /quant)
- Verifies peers via honesty-auth challenge-response
- Integrates with music.vaked.dev for listening choreography
```

## Traffic Flow

```
App (honest-ircd)
  │  plaintext IRC messages
  ▼
honest-crypt
  │  Kyber+X25519 encrypted
  ▼
honest-mesh (Tailscale sidecar)
  │  WireGuard encrypted (Tailscale's own crypto)
  ▼
honest-vpn (Mullvad double-hop)
  │  Entry: Mullvad server in country A
  │  Exit:  Mullvad server in country B
  ▼
Internet ──▶ Peer's honest-vpn ──▶ Peer's honest-crypt ──▶ Peer's honest-ircd
```

## Why Double Hop?

1. **Entry hop**: hides your real IP from the exit node and the Tailnet
2. **Exit hop**: appears as your source IP to all peers
3. **Neither Mullvad nor Tailscale sees plaintext**: honest-crypt encrypts before Tailscale, and Mullvad sees only WireGuard-encrypted traffic
4. **Compromise resistance**: if Mullvad is compromised, the adversary sees Tailscale WireGuard traffic, not plaintext. If Tailscale is compromised, the adversary sees Kyber-encrypted ciphertext, not plaintext. Both must be broken simultaneously.

## Honesty Vector → CLI Flow

```bash
# First time: create your identity
honest-irc init
# → interactive prompt asking the 17 honesty questions
# → generates ~/.honest-irc/identity.json (signed with Dilithium)
# → generates ~/.honest-irc/identity.pub (public key for peers)
# → reserves your display name in the mesh

# Start the full stack (all four sidecars):
honest-irc up
# → spawns honest-vpn (mullvad double-hop)
# → spawns honest-crypt (kyber encryption)
# → spawns honest-mesh (tailscale)
# → spawns honest-ircd (IRC daemon)
# → joins #general on the mesh

# Connect to a peer:
/msg ⊰•-•⦑ The Architect of Structural Honesty ⦒•-•⊱ hello world
# → honest-crypt encrypts
# → honest-mesh routes
# → honest-vpn double-hops
# → peer decrypts, verifies Dilithium signature, displays message
```

## music.vaked.dev Integration

```bash
# Share current track:
/music
# → queries music.vaked.dev API for current track
# → broadcasts to room: "⊰•-•⦑ Architect ⦒•-•⊱ ♫ Unforgiven II · choreography 'édesapa' · 3/7"

# Status is implicit from the music:
# no need for /status — the choreography IS the status
```

## Reserved Display Names

```
⊰•-•⦑ The Architect of Structural Honesty ⦒•-•⊱  — Péter (founder)
(all others: first-claim-wins in the mesh)
```
