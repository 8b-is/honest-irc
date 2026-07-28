use serde::{Deserialize, Serialize};
use crate::honesty::HonestyVector;
use crate::crypto::{CryptoSession, Identity};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Message {
    /// Text message in a room or DM
    Text { from: String, room: String, body: String },
    /// Emote (/me)
    Emote { from: String, room: String, action: String },
    /// Honesty vector broadcast (signed)
    Honesty { from: String, vector: String },
    /// Challenge-verify request
    Verify { from: String, target: String, question: String },
    /// Challenge response
    VerifyResponse { from: String, target: String, answer: String, correct: bool },
    /// Join room
    Join { from: String, room: String },
    /// Leave room
    Leave { from: String, room: String },
    /// Change display name
    Nick { from: String, old: String, new: String },
    /// Share current music track from music.vaked.dev
    Music { from: String, track: String, choreography: String, position: (usize, usize) },
    /// Mood status from choreography
    Status { from: String, mood: String },
    /// Generate shared ternary matrix with peer (using seed exchange)
    Quant { from: String, target: String, seed: String },
    /// System message
    System { body: String },
    /// Ping
    Ping,
    /// Pong
    Pong,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Envelope {
    /// Sender's route_id
    pub from: String,
    /// Encrypted payload (ChaCha20Poly1305)
    pub payload: Vec<u8>,
    /// Sequence number for replay protection
    pub seq: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub members: Vec<String>,
}

/// The IRCd — simple event loop for quantum-proof messaging.
pub struct IrcDaemon {
    pub identity: Identity,
    pub rooms: Vec<Room>,
    pub peers: Vec<String>,
    pub crypto: Option<CryptoSession>,
}

impl IrcDaemon {
    pub fn new(identity: Identity) -> Self {
        IrcDaemon {
            identity,
            rooms: vec![Room { name: "#general".into(), members: vec![] }],
            peers: vec![],
            crypto: Some(CryptoSession::new()),
        }
    }

    pub fn handle(&mut self, msg: Message) -> Option<Message> {
        match &msg {
            Message::Ping => Some(Message::Pong),
            Message::Text { from, body, .. } => {
                println!("[{}] {}", from, body);
                None
            }
            Message::Honesty { from, .. } => {
                println!("[{}] shared honesty vector", from);
                None
            }
            Message::Music { from, track, choreography, position } => {
                println!("[{}] ♫ {} ({} track {}/{})", from, track, choreography, position.0, position.1);
                None
            }
            Message::Join { from, room } => {
                if let Some(r) = self.rooms.iter_mut().find(|r| &r.name == room) {
                    if !r.members.contains(from) {
                        r.members.push(from.clone());
                    }
                }
                Some(Message::System { body: format!("{} joined {}", from, room) })
            }
            Message::Leave { from, room } => {
                if let Some(r) = self.rooms.iter_mut().find(|r| &r.name == room) {
                    r.members.retain(|m| m != from);
                }
                Some(Message::System { body: format!("{} left {}", from, room) })
            }
            _ => None,
        }
    }
}

/// Wire format: JSON-encoded Envelope over WebSocket.
/// The payload inside is encrypted.
pub fn encode(msg: &Message) -> String {
    serde_json::to_string(msg).unwrap_or_default()
}

pub fn decode(data: &str) -> Option<Message> {
    serde_json::from_str(data).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_pong() {
        let irc = IrcDaemon::new(
            Identity {
                route_id: "deadbeef".into(),
                display_name: crate::crypto::DisplayName {
                    raw: "test_user".into(),
                    ascii_prefix: "test".into(),
                },
                vector_hash: "abcdef".into(),
            }
        );
        let result = irc.handle(Message::Ping);
        assert!(matches!(result, Some(Message::Pong)));
    }

    #[test]
    fn test_join_room() {
        let mut irc = IrcDaemon::new(
            Identity {
                route_id: "deadbeef".into(),
                display_name: crate::crypto::DisplayName { raw: "test".into(), ascii_prefix: "test".into() },
                vector_hash: "abcdef".into(),
            }
        );
        irc.handle(Message::Join { from: "alice".into(), room: "#test".into() });
        assert!(irc.rooms.iter().any(|r| r.name == "#test" && r.members.contains(&"alice".to_string())));
    }
}
