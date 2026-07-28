use honest_irc::hardening::{sanitize_body, strip_egress};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let addr = args.get(1).map(|s| s.as_str()).unwrap_or("127.0.0.1:9667");

    println!("connecting to {}...", addr);
    let mut stream = TcpStream::connect(addr).expect("failed to connect to ircd");

    // Read banner
    let mut reader = BufReader::new(stream.try_clone().expect("clone failed"));
    for _ in 0..3 {
        let mut line = String::new();
        if reader.read_line(&mut line).is_ok() && !line.is_empty() {
            print!("{}", line);
        }
    }

    println!();
    println!("connected. type /help for commands, /quit to exit.");
    println!();

    let stdin = std::io::stdin();
    let input_reader = BufReader::new(stdin);

    // Spawn reader thread for server responses
    let mut stream_clone = stream.try_clone().expect("clone failed");
    std::thread::spawn(move || {
        let mut r = BufReader::new(&mut stream_clone);
        loop {
            let mut line = String::new();
            match r.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => print!("{}", line),
                Err(_) => break,
            }
        }
    });

    for line in input_reader.lines().flatten() {
        let line = sanitize_body(&line);
        let line = strip_egress(&line);

        if line == "/quit" || line == "/exit" {
            let _ = writeln!(stream, "/quit");
            println!("goodbye.");
            break;
        }

        if line.is_empty() { continue; }
        let _ = writeln!(stream, "{}", line);
    }
}
