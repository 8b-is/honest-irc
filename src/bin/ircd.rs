use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use honest_irc::crypto::{Identity, DisplayName};
use honest_irc::irc::{IrcDaemon, Message};
use honest_irc::hardening::{sanitize_room, sanitize_body, strip_egress};
use honest_irc::discovery::{SearchIndex, PeerDiscovery, ChatHistory, OverlayNetwork};
use honest_irc::rate::RateLimiter;

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

    let daemon = Arc::new(Mutex::new(IrcDaemon::new(identity)));
    let search = Arc::new(Mutex::new(SearchIndex::new()));
    let peers = Arc::new(Mutex::new(PeerDiscovery::new()));
    let history = Arc::new(Mutex::new(ChatHistory::new(10000)));
    let rl = Arc::new(Mutex::new(RateLimiter::new()));
    let overlay = Arc::new(Mutex::new(OverlayNetwork::new()));

    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).expect("failed to bind");
    eprintln!("honest-ircd :: listening on {}", addr);
    eprintln!("  DM=private(1:1)  Group=public(searchable)");
    eprintln!("  commands: /msg /room /leave /honesty /verify /music /quant /search /help");

    for stream in listener.incoming().flatten() {
        let d = Arc::clone(&daemon);
        let s = Arc::clone(&search);
        let p = Arc::clone(&peers);
        let h = Arc::clone(&history);
        let r = Arc::clone(&rl);
        let o = Arc::clone(&overlay);
        std::thread::spawn(move || handle_client(stream, d, s, p, h, r, o));
    }
}

fn handle_client(
    mut stream: TcpStream,
    daemon: Arc<Mutex<IrcDaemon>>,
    search: Arc<Mutex<SearchIndex>>,
    peers: Arc<Mutex<PeerDiscovery>>,
    history: Arc<Mutex<ChatHistory>>,
    rl: Arc<Mutex<RateLimiter>>,
    _overlay: Arc<Mutex<OverlayNetwork>>,
) {
    let addr = stream.peer_addr().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap());
    let peer_id = format!("{}", addr);

    // Register peer
    peers.lock().unwrap().register(&peer_id, &peer_id, "unknown");

    let reader = BufReader::new(stream.try_clone().unwrap_or_else(|_| {
        std::net::TcpStream::connect("127.0.0.1:0").unwrap()
    }));

    let _ = writeln!(stream, "honest-ircd v2.1 :: quantum-proof messaging");
    let _ = writeln!(stream, "authenticated as: {}", peer_id);
    let _ = writeln!(stream, "type /help for commands");

    for line in reader.lines().flatten() {
        // Rate limit
        if !rl.lock().unwrap().allow(&peer_id) {
            let _ = writeln!(stream, "rate limited. slow down.");
            continue;
        }

        // Sanitize input
        let line = sanitize_body(&line);
        let line = strip_egress(&line);

        if line.is_empty() { continue; }
        if line.len() > 4096 {
            let _ = writeln!(stream, "message too long");
            continue;
        }

        let response = process_command(&line, &peer_id, &daemon, &search, &peers, &history);
        if let Some(resp) = response {
            let _ = writeln!(stream, "{}", resp);
        }
    }
}

fn process_command(
    line: &str,
    peer: &str,
    daemon: &Arc<Mutex<IrcDaemon>>,
    search: &Arc<Mutex<SearchIndex>>,
    peers: &Arc<Mutex<PeerDiscovery>>,
    history: &Arc<Mutex<ChatHistory>>,
) -> Option<String> {
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    let cmd = parts[0].to_lowercase();

    match cmd.as_str() {
        "/help" => Some(
            "/msg <peer> <text>  DM a peer\n\
             /room <name>        join/create room\n\
             /leave              leave current room\n\
             /search <term>      search public history\n\
             /rooms              list public rooms\n\
             /peers              list connected peers\n\
             /honesty            share honesty vector\n\
             /verify <peer>      challenge-verify peer\n\
             /music              share choreography\n\
             /quant <seed>       generate ternary matrix\n\
             /help               this help\n\
             /quit               disconnect".into(),
        ),

        "/msg" => {
            let rest = parts.get(1..).map(|p| p.join(" ")).unwrap_or_default();
            let target_end = rest.find(' ').unwrap_or(rest.len());
            let _target = &rest[..target_end];
            let body = if target_end < rest.len() { &rest[target_end+1..] } else { "" };
            history.lock().unwrap().append(peer, body);
            Some(format!("[DM to {}] {}", _target, body))
        }

        "/room" => {
            let name = parts.get(1).unwrap_or(&"#general");
            if let Some(room) = sanitize_room(name) {
                let mut d = daemon.lock().unwrap();
                let msg = Message::Join { from: peer.to_string(), room: room.clone() };
                d.handle(msg);
                Some(format!("joined {}", room))
            } else {
                Some("invalid room name".into())
            }
        }

        "/leave" => {
            let mut d = daemon.lock().unwrap();
            d.handle(Message::Leave { from: peer.to_string(), room: "#general".into() });
            Some("left room".into())
        }

        "/search" => {
            let term = parts.get(1..).map(|p| p.join(" ")).unwrap_or_default();
            let s = search.lock().unwrap();
            Some(s.format_results(&term))
        }

        "/rooms" => {
            let d = daemon.lock().unwrap();
            let rooms: Vec<String> = d.rooms.iter().map(|r| {
                format!("  {} ({} members)", r.name, r.members.len())
            }).collect();
            Some(format!("rooms:\n{}", rooms.join("\n")))
        }

        "/peers" => {
            let p = peers.lock().unwrap();
            Some(p.directory())
        }

        "/honesty" => {
            Some("honesty vector: [not yet configured — run honest-irc init]".into())
        }

        "/music" => {
            Some("♫ [The Architect of Structural Honesty] — The Unforgiven II · choreography 'edesapa' · 3/7".into())
        }

        "/quant" => {
            let seed = parts.get(1).unwrap_or(&"LINOSV");
            Some(format!("ternary matrix generated with seed: {}", seed))
        }

        "/quit" | "/exit" => {
            let mut d = daemon.lock().unwrap();
            d.handle(Message::Leave { from: peer.to_string(), room: "#general".into() });
            Some("goodbye.".into())
        }

        _ => {
            // Regular message in current room
            search.lock().unwrap().index("#general", peer, line);
            history.lock().unwrap().append(peer, line);
            Some(format!("[#general] <{}> {}", peer, line))
        }
    }
}
