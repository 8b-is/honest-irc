# honest-irc — Headscale Setup Guide (Roadmap v1.42)

## Overview

honest-irc uses Tailscale-compatible mesh networking via the `honest-mesh` sidecar.
For fully air-gapped, self-hosted deployments, replace Tailscale's coordination
server with **Headscale** — the open-source implementation.

This guide covers deploying Headscale + honest-irc on a Hetzner CX or similar VPS,
with hardware from the [nix-base](https://github.com/peterlodri-sec/nix-base) fleet.

---

## Prerequisites

- A Linux server (Hetzner CX22 or larger, any KVM VPS)
- A domain name (e.g. `mesh.yourdomain.dev`)
- Nix package manager (optional but recommended — nix-base fleet uses NixOS)
- Tailscale client installed on all peers (or Headscale's `tailscale` fork)

---

## Step 1: Deploy Headscale

### Option A: NixOS (nix-base fleet)

```nix
# hosts/your-host/headscale.nix
{ config, pkgs, ... }:
{
  services.headscale = {
    enable = true;
    address = "0.0.0.0";
    port = 8080;
    settings = {
      server_url = "https://mesh.yourdomain.dev";
      listen_addr = "0.0.0.0:8080";
      grpc_listen_addr = "0.0.0.0:50443";
      grpc_allow_insecure = false;
      ip_prefixes = [
        "100.64.0.0/10"   # Tailscale IPv4 range
        "fd7a:115c:a1e0::/48"  # Tailscale IPv6 range
      ];
      derp = {
        server = {
          enabled = false;  # use Tailscale's DERP or self-host
        };
      };
      dns_config = {
        override_local_dns = true;
        nameservers = [ "1.1.1.1" ];
      };
    };
  };

  # Reverse proxy for Headscale API
  services.nginx.virtualHosts."mesh.yourdomain.dev" = {
    forceSSL = true;
    enableACME = true;
    locations."/" = {
      proxyPass = "http://127.0.0.1:8080";
    };
  };
}
```

```bash
nh os switch .#your-host
```

### Option B: Docker (any VPS)

```bash
mkdir -p ./headscale/config
cat > ./headscale/config/config.yaml << 'EOF'
server_url: https://mesh.yourdomain.dev
listen_addr: 0.0.0.0:8080
grpc_listen_addr: 0.0.0.0:50443
ip_prefixes:
  - 100.64.0.0/10
  - fd7a:115c:a1e0::/48
dns_config:
  override_local_dns: true
  nameservers:
    - 1.1.1.1
EOF

docker run -d \
  --name headscale \
  -v $(pwd)/headscale/config:/etc/headscale \
  -v $(pwd)/headscale/data:/var/lib/headscale \
  -p 8080:8080 \
  -p 50443:50443 \
  headscale/headscale:latest \
  headscale serve
```

---

## Step 2: Create the honest-irc mesh namespace

```bash
# Create a user for the mesh
headscale users create honest-irc

# Generate a pre-auth key (valid for 24h, reusable)
headscale preauthkeys create \
  --user honest-irc \
  --reusable \
  --expiration 24h

# Output:
# <PREAUTH_KEY>
```

---

## Step 3: Configure honest-irc peers

On each peer machine:

```bash
# Install Tailscale client
curl -fsSL https://tailscale.com/install.sh | sh

# Point Tailscale to your Headscale server
tailscale up \
  --login-server https://mesh.yourdomain.dev \
  --authkey <PREAUTH_KEY> \
  --hostname honest-alice \
  --accept-routes

# Verify connectivity
tailscale status
# honest-alice    alice@    linux   -
# honest-bob      bob@      linux   active; direct 100.64.0.3:41641
```

---

## Step 4: Start honest-irc with Headscale

```bash
# Skip the Mullvad VPN if on trusted LAN/VPS
honest-irc up --no-vpn --headscale-url https://mesh.yourdomain.dev

# Or with full Mullvad double-hop:
honest-irc up --headscale-url https://mesh.yourdomain.dev
```

---

## Step 5: Verify mesh connectivity

```bash
honest-irc status
# Mesh: 3 peers online
#   [The Architect of Structural Honesty] — 100.64.0.2:41641
#   alice — 100.64.0.3:41641
#   bob — 100.64.0.4:41641
#   DERP relay: ams (Amsterdam) — latency 12ms
#
# Rooms: #general (3 members)
# DM: active with alice, bob

# Test connection:
/msg alice ping
# alice: pong (latency 8ms direct, kyber-verified)
```

---

## Troubleshooting

### Peer not connecting directly (shows DERP relay)

```bash
# Check firewall — UDP 41641 must be open
sudo ufw allow 41641/udp

# Check NAT — if behind CGNAT, DERP relay is the fallback
tailscale status
# shows "relay" instead of "direct"
# → enable random high ports or use UPnP
```

### Headscale API unreachable

```bash
# Check if Headscale is listening
curl https://mesh.yourdomain.dev/health
# {"status":"pass"}
```

### Pre-auth key expired

```bash
headscale preauthkeys create \
  --user honest-irc \
  --reusable \
  --expiration 87600h  # 10 years
```

---

## Roadmap

### v0.1.0 — Genesis (current)
- [x] Core sidecar architecture design
- [x] Honesty-auth protocol spec + Rust implementation
- [x] X25519 + ChaCha20Poly1305 encryption
- [x] IRC-like protocol (/msg, /room, /music, /quant, /honesty, /verify)
- [x] Reserved names registry
- [x] DM = private 1:1, Group = public + searchable
- [x] ASCII README signed by [The Architect of Structural Honesty]
- [x] Blacksmith CI + cosign signing
- [x] All 7 Rust tests passing

### v0.2.0 — Sidecar Sidecars [DONE]
- [x] `honest-vpn` binary: Mullvad double-hop WireGuard via CLI
- [x] `honest-crypt` binary: Kyber-1024 + X25519 hybrid KEM
- [x] `honest-mesh` binary: Headscale/Tailscale auto-join
- [x] `honest-ircd` binary: standalone daemon with TCP listener (:9667)
- [x] Sidecar orchestration: `honest-irc up` spawns all 4
- [x] ASCII architecture blueprint (ARCHITECTURE.txt)
- [x] Pimped README with embedded architecture + security table

### v0.3.0 — Memory Fortress [DONE]
- [x] `memfd_create` + `F_SEAL_WRITE` for chat/crypto/identity/seed regions
- [x] `mlock` all regions — never swapped to disk
- [x] `mprotect(PROT_READ)` after init — immutable after setup
- [x] `prctl(PR_SET_DUMPABLE, 0)` — no ptrace, no core dumps
- [x] `MADV_DONTDUMP` — exclude from crash dumps
- [x] Honesty vector scaffold (JSON serializable)
- [x] SSH keys via /dev/fd pipes — never touch filesystem

### v0.4.0 — Quant1bitLLM Per-Byte Crypto [DONE]
- [x] Per-byte sub-key derivation: `HKDF(session_key || LLM_weight[i] || nonce || i)`
- [x] Timing-safe: all 3 sub-keys loaded in registers, constant-time select
- [x] CSPRNG nonce combined with LLM weights (not raw XOR)
- [x] Forward secrecy: session key rotation every N messages
- [x] LLM distribution: BIP39 seed phrase option

### v0.5.0 — music.vaked.dev Integration [DONE]
- [x] `/music` command queries music.vaked.dev API
- [x] Choreography tracking: position + total tracks
- [x] Status auto-derived from current track
- [x] Sample choreographies: 'edesapa' (Metallica), 'vivaldi-spring'

### v0.6.0 — Post-Quantum Crypto [DONE]
- [x] Kyber-1024 KEM (ML-KEM-1024) key generation, encapsulation, decapsulation
- [x] Dilithium-5 signatures (ML-DSA-87) key generation, signing, verification
- [x] SPHINCS+ backup signatures (SLH-DSA-SHAKE-256s)
- [x] Hybrid PQC bundle generation
- [x] Trust Model separated from Threat Model
- [x] All 7 security audit findings resolved

### v0.7.0 — Search & Discovery
- [ ] `/search <term>` — search all public group history
- [ ] Peer discovery via Headscale API
- [ ] Room directory: `/rooms` lists all public rooms
- [ ] Honesty vector search: find peers by shared interests

### v1.0.0 — Production Mesh
- [ ] DERP relay self-hosting (no Tailscale DERP dependency)
- [ ] Persistent room history (in sealed memory, purged on restart)
- [ ] Rate limiting: max messages/sec per peer
- [ ] Honesty vector rotation: re-verify every 30 days
- [ ] Automatic peer reconnection after VPN rotation

### v1.42 — The Answer
- [ ] .onion / I2P overlay (Tor hidden service for honest-irc)
- [ ] Post-quantum mesh DHT (Kyber-based Kademlia)
- [ ] Cross-platform clients: Terminal (TUI), Desktop (webview), Mobile (Flutter)
- [ ] Quant1bitLLM live retraining from chat context
- [ ] Honesty vector "proof of personhood" — zero-knowledge proof that you are you, without revealing the vector
- [ ] Full nix-base fleet integration: deploy honest-irc on dev-cx53, hetzner, public-services-host
- [ ] music.vaked.dev "listening together" — synchronized playback across peers
