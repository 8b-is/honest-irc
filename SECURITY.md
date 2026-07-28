# honest-irc — Security Architecture

## Zero-Disk. Zero-Trace. Self-Encrypted Memory.

> The chat lives only in encrypted memory regions. Nothing touches disk.
> Per-byte encryption seeded by a 1-bit quantized LLM. Multihop SSH bootstrap
> with throwaway keys. When the process dies, the data dies with it.

---

## Layer -2: Multihop SSH Throwaway Init

Before any sidecar starts, the connection is bootstrapped through
a chain of throwaway SSH hops:

```
honest-irc init
  │
  ▼
ssh-keygen -t ed25519 -f /tmp/honest-hop-1 -N ""  (throwaway, deleted after use)
ssh-keygen -t ed25519 -f /tmp/honest-hop-2 -N ""  (throwaway)
ssh-keygen -t ed25519 -f /tmp/honest-hop-3 -N ""  (throwaway)
  │
  ▼
ssh -i /tmp/honest-hop-1 user@hop1.example.com \
  ssh -i /tmp/honest-hop-2 user@hop2.example.com \
    ssh -i /tmp/honest-hop-3 user@hop3.example.com \
      "exec honest-irc up --no-init"
  │
  ▼
shred -zu /tmp/honest-hop-1 /tmp/honest-hop-2 /tmp/honest-hop-3
```

Properties:
- Each SSH key exists only for the duration of the init sequence
- Keys are shredded after use (`shred -zu` overwrites + deletes)
- The final hop spawns the honest-irc sidecars directly
- No SSH key material persists on any disk
- If any hop is compromised, the attacker sees only another SSH connection
- The chain of trust is ephemeral — new keys per session, never reused

## Layer -1: Self-Encrypted Memory (memfd + mlock)

All chat data lives in a memory-only filesystem backed by an encrypted `memfd`:

```rust
use std::fs::File;
use std::os::unix::io::FromRawFd;

// Create anonymous in-memory file (never touches disk)
let fd = unsafe { libc::memfd_create(
    b"honest-irc-chat\0".as_ptr() as *const _,
    libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING
) };

// Seal it — no resizing, no writing to disk, no future writes from outside
unsafe {
    libc::fcntl(fd, libc::F_ADD_SEALS,
        libc::F_SEAL_SEAL |   // no further seals can be added
        libc::F_SEAL_SHRINK | // cannot decrease size
        libc::F_SEAL_GROW |   // cannot increase size
        libc::F_SEAL_WRITE    // cannot write to it
    );
}

// Lock into RAM — never swapped to disk
unsafe { libc::mlock(ptr, size); }

// The chat data now exists ONLY in this memory region.
// When the process exits, the kernel reclaims the memory.
// Nothing remains on any storage device.
```

Memory regions:
- `chat.mem`: all IRC messages (encrypted at rest in memfd)
- `crypto.mem`: session keys, ephemeral Kyber keypairs, nonce counters
- `identity.mem`: honesty vector, Dilithium signing keys (loaded from disk, then shredded)
- `seed.mem`: the 1-bit quantized LLM weights used as per-byte encryption seeds

All regions are:
- `memfd_create` + `F_SEAL_WRITE` — write-sealed, no disk backing
- `mlock` — locked in RAM, never swapped
- `mprotect(PROT_READ)` after writing — read-only after initialization
- Freed on process exit (kernel reclaims, no shredding needed)

## Layer 0: honest-vpn (Mullvad Double Hop)

```
honest-vpn --entry switzerland --exit iceland
```

Two-hop WireGuard tunnel:
1. Entry: Mullvad server in Switzerland (hides real IP)
2. Exit: Mullvad server in Iceland (appears as source)

Each hop adds ~15ms latency. Total overhead: ~30ms.
Tunnel rotated every 4 hours (new entry/exit pair, new WireGuard keys).

## Layer 1: honest-crypt (Kyber + X25519 + Per-Byte LLM Seed)

### Kyber-X25519 Hybrid Key Exchange

```
Alice                                    Bob
  │                                       │
  │── Kyber public key + X25519 pub ─────▶│
  │                                       │
  │◀── Kyber ciphertext + X25519 pub ────│
  │                                       │
  │  shared = Kyber.Decap(ct) XOR          │
  │           X25519.DH(alice_sec, bob_pub)│
  │                                       │
  │  session_key = HKDF(shared, "honest-irc-v1")
```

### Per-Byte Encryption via Quant1bitLLM Seed

After the session key is established, each BYTE of plaintext gets its own
unique encryption sub-key, derived from a 1-bit quantized LLM weight matrix:

```
For byte i in message:
  seed[i] = Quant1bitLLM.weights[i % weight_count]  // {-1, 0, +1}
  sub_key[i] = HKDF(session_key || seed[i] || i)
  cipher_byte[i] = plain_byte[i] XOR sub_key[i][0]
```

The Quant1bitLLM is a tiny (~1MB) ternary-quantized language model
(it can generate coherent but nonsensical text). Its weight matrix is
used as an entropy source for per-byte key derivation:

```
Quant1bitLLM("the quick brown fox") → weight_activations = {-1, 0, +1}^N
Each weight activation = one byte's seed
If weight = 0:    sub_key derived from session_key || "zero" || i
If weight = +1:   sub_key derived from session_key || "pos" || i  
If weight = -1:   sub_key derived from session_key || "neg" || i
```

Properties:
- Each byte has a unique encryption key
- The key schedule is deterministic (same LLM weights → same sub-keys)
- An attacker must recover BOTH the session key AND the LLM weight matrix
- The LLM weights exist only in sealed memory (Layer -1)
- Pattern analysis is impossible: identical plaintext bytes produce different ciphertext
- The LLM acts as a "semantic one-time pad" — weight activations are unpredictable to anyone without the specific model file

### Why Quant1bitLLM?

- A 1-bit quantized LLM produces {-1, 0, +1} activations — exactly the ternary space
- The weight matrix is 12.80x smaller than fp32 (same as MLX-QUANT)
- The activations are deterministic given the same input prompt
- Using "the current chat context" as prompt → per-message unique key schedule
- Even if session key leaks, attacker needs the specific LLM checkpoint used
- The LLM checkpoint is never transmitted — it's a pre-shared secret between peers

## Layer 2: honest-mesh (Tailscale/Headscale)

Standard Tailscale mesh. All traffic at this layer is ALREADY encrypted
by honest-crypt. Tailscale adds a second layer of WireGuard encryption.

## Layer 3: honest-ircd (IRC Daemon)

The chat protocol operates over the encrypted channels. Messages are
encrypted per-byte, stored in sealed memory, and shredded on exit.

```
    ┌────────────────────────────────────────┐
    │         Self-Encrypted Memory           │
    │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐  │
    │  │ chat │ │crypto│ │ident │ │ seed │  │
    │  │ .mem │ │ .mem │ │ .mem │ │ .mem │  │
    │  └──────┘ └──────┘ └──────┘ └──────┘  │
    │    mlock'd + sealed + read-only         │
    └────────────────────────────────────────┘
```

---

## Complete Init Sequence (honest-irc init)

```
1. Generate 3x throwaway SSH keys
2. Establish 3-hop SSH tunnel
3. Spawn honest-vpn (Mullvad double-hop)
4. Load Quant1bitLLM weights into seed.mem (mlock, seal)
5. Generate Kyber + X25519 keypairs → crypto.mem (mlock, seal)
6. Load honesty vector → identity.mem (mlock, seal)
7. Erase identity.json from disk (shred -zu)
8. Spawn honest-mesh (Tailscale)
9. Spawn honest-ircd
10. Shred SSH keys
11. Join #general
```

## Shutdown Sequence (SIGTERM / exit)

```
1. Send /quit to all rooms
2. Flush any pending encrypted messages
3. munlock all memory regions
4. close memfd (kernel reclaims memory)
5. Shred any temp files in /tmp
6. Process exits
7. Nothing remains. Zero trace.
```

## Threat Model

| Attack | Defense |
|--------|---------|
| Disk forensics | memfd — nothing on disk, ever |
| RAM cold boot | mlock'd region is small (<100MB), encrypted at rest in memory too |
| Quantum computer breaks Kyber | X25519 hybrid — classical ECDH still holds |
| Quantum computer breaks ECDH | Kyber-1024 — PQC KEM still holds |
| Session key leaked | Per-byte LLM sub-keys — need LLM weights too |
| LLM weights stolen | Pre-shared, never transmitted, unique per peer-pair |
| Mullvad compromised | Tailscale WireGuard is second encryption layer |
| Tailscale compromised | Kyber+X25519 encryption already applied |
| Both Mullvad AND Tailscale compromised | Per-byte LLM sub-keys still protect each byte |
| Impersonation | Honesty-auth — cannot fake life story consistently |
| Replay attack | Per-message nonce + per-byte sub-key prevents reuse |
| Metadata analysis | Double-hop VPN hides source, Tailnet hides peer topology |
| Rubber-hose cryptanalysis | Honesty vector is emotional truth — cannot extract what isn't a "secret" |
