# honest-irc: quantum-proof decentralized messaging

auth via honesty. transport via Mullvad double-hop. crypto via Kyber+X25519.
memory via memfd+mlock. per-byte encryption via Quant1bitLLM seed.
multihop SSH throwaway init. cosign-signed releases. zero disk trace.

## architecture

```
honest-irc node
  |
  |-- honest-vpn    (Mullvad double-hop: entry -> exit)
  |-- honest-crypt  (Kyber-1024 + X25519 hybrid, per-byte LLM sub-keys)
  |-- honest-mesh   (Tailscale/Headscale sidecar)
  |-- honest-ircd   (IRC-like daemon, honesty-auth, music.vaked.dev)
```

## commands

```
honest-irc init     create your honesty vector identity
honest-irc up       start all sidecars (vpn -> crypt -> mesh -> ircd)
honest-irc status   show mesh topology + connected peers
honest-irc quant    generate shared ternary matrix with a peer
```

## security layers

| layer | what | why |
|-------|------|-----|
| -2 | multihop SSH throwaway init | ephemeral bootstrap, keys shredded after use |
| -1 | memfd + mlock + F_SEAL_WRITE | self-encrypted memory, never on disk, sealed |
| 0  | Mullvad double-hop VPN | hides real IP, entry->exit chain |
| 1  | Kyber-1024 + X25519 hybrid | post-quantum + classical defense-in-depth |
| 1b | per-byte Quant1bitLLM sub-keys | unique key per byte, LLM weights as entropy |
| 2  | Tailscale WireGuard mesh | peer-to-peer encrypted tunnels, DERP relay |
| 3  | honesty-auth | identity through personality, not passwords |

## honesty-auth

identity is a vector of personal answers. no passwords. no OAuth. no email.

fields: game, color, poet, poem, band, song, birth, mother, constellation,
belief, names, country, pets, last_sex. the first 5 fields = core identity hash.
stable fields (birth, names, constellation) must match exactly.
volatile fields (song, mood, belief) can vary. 80% threshold for verification.

if you are not you, you cannot answer consistently. a government can steal
your password. it cannot steal your mother relationship.

## protocol

```
/msg <user> <text>       DM a peer
/room <name>              join or create a room
/leave                    leave current room
/me <action>              3rd-person emote
/nick <name>              change display name
/honesty                  share signed honesty vector
/verify <user>            challenge-verify a peer
/music                    share current music.vaked.dev track
/quant <seed>             generate shared ternary matrix
```

## reserved names

```
[The Architect of Structural Honesty]  -- Peter (founder)
(all others: first-claim-wins in the mesh)
```

## genesis

```
vaked-base genesis seal hash:
7c242080f5f821e5eaf563fe2208d60632c451687baf65f4fe8e4a0d226e3ecf
```

signed on 2026-07-28, full moon, 10,000X
by [The Architect of Structural Honesty]

WE. {-1, 0, +1}.
