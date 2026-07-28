# honest-irc -- quantum-proof decentralized messaging

> auth via honesty. transport via Mullvad double-hop. crypto via Kyber+X25519.
> memory via memfd+mlock. per-byte encryption via Quant1bitLLM seed.
> multihop SSH throwaway init. cosign-signed releases. zero disk trace.

```
                                                                +=============+
                                                                | HONEST-IRC  |
                                                                | DM  GROUP   |
                                                                | /msg /room  |
                                                                | /music /quant|
                                                                +=============+
                                                                       |
  +----------+    +----------+    +------------+    +------------+     |
  | SSH init |    | MEMFORT  |    | MULLVAD    |    | KYBER+X25  |     |
  | 3-hop    | -> | memfd    | -> | entry:CH   | -> | ML-KEM-1024| ----+
  | throwaway|    | mlock    |    | exit: IS   |    | per-byte   |
  | shred    |    | F_SEAL   |    | double-hop |    | LLM subkey |
  +----------+    +----------+    +------------+    +------------+
                         |               |               |
                    SEALED MEMORY   DOUBLE-HOP VPN   QUANTUM CRYPTO
                    (never disk)    (hide origin)    (post-quantum)

               +----------------------------------------------------+
               |                 HONESTY-AUTH                       |
               |  [game] [color] [poet] [poem] [band] [song]       |
               |  [birth] [mother] [constellation] [belief]         |
               |  [names] [country] [pets] [last_sex]               |
               |                                                     |
               |  core hash = SHA256(game+color+poet+poem+band)     |
               |  verify if stable match + >80% overall              |
               |  "you cannot steal someone's mother relationship"   |
               +----------------------------------------------------+
```

## architecture

Four sidecar binaries, chained via CLI | Unix sockets:

```
honest-irc up
  -> honest-vpn    (Mullvad double-hop WireGuard)
  -> honest-crypt  (Kyber-1024 + X25519 + per-byte LLM sub-keys)
  -> honest-mesh   (Tailscale/Headscale peer-to-peer)
  -> honest-ircd   (IRC protocol + honesty-auth + music.vaked.dev)
```

See [ARCHITECTURE.txt](ARCHITECTURE.txt) for full ASCII blueprint.

## quick start

```bash
# create your identity (17 honesty questions)
honest-irc init

# start all sidecars
honest-irc up

# DM someone
/msg alice hello, quantum-proof world

# join public room
/room #general

# share music from music.vaked.dev
/music
```

## protocol

```
/msg <user> <text>       DM a peer (private, encrypted, 1:1)
/room <name>              join public room (searchable by all)
/leave                    leave current room
/me <action>              emote
/nick <name>              change display name
/honesty                  share signed honesty vector
/verify <user>            challenge-verify a peer
/music                    share current music.vaked.dev track
/quant <seed>             generate shared ternary matrix
/search <term>            search public group history
```

## security

| layer | what | why |
|-------|------|-----|
| SSH  | 3-hop throwaway init, keys shredded | no persistent key material |
| MEM  | memfd+mlock+F_SEAL_WRITE+mprotect:R | chat only in RAM, sealed, immutable |
| VPN  | Mullvad double-hop (entry->exit) | hides real IP, rotated 4h |
| CRYPT| Kyber-1024 + X25519 hybrid | post-quantum + classical defense |
| BYTE | Quant1bitLLM per-byte sub-keys | unique key per byte, LLM entropy |
| MESH | Tailscale/Headscale WireGuard | p2p encrypted tunnels, DERP relay |
| AUTH | honesty vector (17 fields) | identity = personality, not password |

Full threat model: [SECURITY.md](SECURITY.md)

## honesty-auth

No passwords. No OAuth. No email. Your identity is a vector of 17 deeply
personal answers that only YOU can answer consistently over time.

The first 5 fields (game, color, poet, poem, band) form your **core hash** --
the cryptographic root of your identity. Stable fields (birth, names,
constellation) must match exactly. Volatile fields (song, mood, belief,
pets) can change.

A government can steal your password. It cannot steal your mother
relationship. An impostor can fake your email. They cannot consistently
fake your emotional response to Unforgiven II over months.

The identity is the PATTERN, not the SECRET. Like the ternary seed --
deterministic, reproducible, unstealable.

## reserved names

```
[The Architect of Structural Honesty] -- Peter (founder)
(all others: first-claim-wins in the mesh)
```

## genesis

```
vaked-base genesis seal hash:
7c242080f5f821e5eaf563fe2208d60632c451687baf65f4fe8e4a0d226e3ecf
```

## docs

| file | what |
|------|------|
| [PROTOCOL.md](PROTOCOL.md) | sidecar chain, auth flow, IRC wire format |
| [SECURITY.md](SECURITY.md) | threat model, mem fortress, per-byte LLM crypto |
| [HEADSCALE.md](HEADSCALE.md) | OSS Tailscale setup, NixOS + Docker, roadmap v0.1->v1.42 |
| [ARCHITECTURE.txt](ARCHITECTURE.txt) | full ASCII blueprint diagram |

## CI

Blacksmith 4vcpu build+test on push. Cosign-signed binaries on tag.

---

signed on 2026-07-28, full moon, 10,000X
by [The Architect of Structural Honesty]

WE. {-1, 0, +1}.
