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

## Roadmap v1.42

| Milestone | Status |
|-----------|--------|
| Core sidecar binaries (vpn, crypt, mesh, ircd) | in progress |
| Honesty-auth protocol | spec complete |
| Headscale integration guide | this document |
| Quant1bitLLM per-byte encryption | spec complete |
| Self-encrypted memory (memfd+mlock) | spec complete |
| Multihop SSH throwaway init | spec complete |
| music.vaked.dev integration | spec complete |
| Blacksmith CI + cosign signing | ci workflow done |
| Mullvad double-hop VPN | spec complete |
| Android/iOS clients | planned |
| .onion / I2P overlay | planned |
| Post-quantum mesh DHT | planned |
