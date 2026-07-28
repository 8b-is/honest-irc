use honest_irc::irc::{IrcDaemon, Message};
use honest_irc::crypto::{Identity, DisplayName};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(9667);

    let identity = Identity {
        route_id: "0000000000000000".into(),
        display_name: DisplayName {
            raw: "anonymous".into(),
            ascii_prefix: "anon".into(),
        },
        vector_hash: "0000".into(),
    };

    let mut daemon = IrcDaemon::new(identity);

    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).expect("failed to bind");
    println!("honest-ircd :: listening on {}", addr);
    println!("  DM=private(1:1)  Group=public(searchable)");
    println!("  /msg /room /honesty /verify /music /quant /search");

    for stream in listener.incoming().flatten() {
        let reader = BufReader::new(&stream);
        for line in reader.lines().flatten() {
            let msg = honest_irc::irc::decode(&line);
            if let Some(msg) = msg {
                if let Some(response) = daemon.handle(msg) {
                    let _ = (&stream).write_all(
                        honest_irc::irc::encode(&response).as_bytes()
                    );
                }
            }
        }
    }
}
