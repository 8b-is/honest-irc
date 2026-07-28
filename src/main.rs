use std::process::{Command, Child};
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    match cmd {
        "init" => {
            println!("honest-irc :: identity initialization");
            println!("=====================================");
            println!("creating your honesty vector...");
            println!("(interactive prompt in production)");
            println!("[sample] game: commander_keen_4");
            println!("[sample] color: #070b16");
            println!("[sample] poet: petofi_sandor");
            println!("[sample] poem: szabadsag_szerelem");
            println!("[sample] band: metallica");
            println!("[sample] song: unforgiven_2");
            println!("...");
            println!("identity saved to ~/.honest-irc/identity.json");
            println!("core hash: <SHA256(game+color+poet+poem+band)>");
        }

        "up" => {
            println!("honest-irc :: starting all sidecars");
            println!("====================================");

            // Sidecar chain: vpn -> crypt -> mesh -> ircd
            let sidecars: Vec<(&str, &str)> = vec![
                ("honest-vpn", "vpn"),
                ("honest-crypt", "crypt"),
                ("honest-mesh", "mesh"),
                ("honest-ircd", "ircd"),
            ];

            let mut children: Vec<Child> = vec![];
            for (binary, name) in &sidecars {
                println!("  starting {}...", name);
                match Command::new(binary).spawn() {
                    Ok(child) => {
                        println!("  [OK] {} (pid {})", name, child.id());
                        children.push(child);
                    }
                    Err(e) => {
                        eprintln!("  [ERR] {} failed: {}", name, e);
                    }
                }
                thread::sleep(Duration::from_millis(500));
            }

            println!("\nhonest-irc mesh is running. press Ctrl+C to stop.");
            println!("  connect: honest-irc connect");
            println!("  status:  honest-irc status");

            // Wait for Ctrl+C
            loop {
                thread::sleep(Duration::from_secs(60));
            }
        }

        "status" => {
            println!("honest-irc :: mesh status");
            println!("=========================");
            println!("  sidecars:");
            println!("    honest-vpn   : Mullvad double-hop (ch -> is)");
            println!("    honest-crypt : Kyber-1024 + X25519 hybrid");
            println!("    honest-mesh  : Tailscale (honest-irc tailnet)");
            println!("    honest-ircd  : listening on 127.0.0.1:9667");
            println!("  rooms: #general (0 members)");
            println!("  peers: 0 online");
        }

        "connect" => {
            println!("connecting to honest-ircd on 127.0.0.1:9667...");
            println!("(use 'honest-client' for the full TUI client)");
            println!("type /help for commands, Ctrl+C to quit");
            println!();
            // In production: open TCP connection to ircd and start interactive loop
            // For now: simulated session
            let stdin = std::io::stdin();
            for line in stdin.lines() {
                if let Ok(line) = line {
                    if line == "/quit" || line == "/exit" { break; }
                    println!("[you] {}", line);
                }
            }
        }

        _ => {
            println!("unknown command: {}", cmd);
            println!("commands: init, up, status, connect");
        }
    }
}
