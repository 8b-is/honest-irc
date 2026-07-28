fn main() {
    let args: Vec<String> = std::env::args().collect();
    let tailnet = args.get(1).map(|s| s.as_str()).unwrap_or("honest-irc");

    println!("honest-mesh :: Tailscale/Headscale peer-to-peer");
    println!("  tailnet    : {}", tailnet);
    println!("  protocol   : WireGuard");
    println!("  relay      : DERP (fallback when direct fails)");
    println!("  discovery  : coordination server");

    // In production: tailscale up --login-server <headscale_url> --authkey <key>
    // Or with Tailscale SaaS: tailscale up --authkey <key>

    println!("  [OK] mesh joined (simulated)");
    println!("  peers: discovering via coordination server...");
}
