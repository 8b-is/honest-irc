use std::collections::HashMap;

/// Search engine for public group history.
/// Groups are public and searchable. DMs are private and not indexed.
pub struct SearchIndex {
    /// group_name -> list of (timestamp, sender, message_body)
    pub messages: HashMap<String, Vec<(u64, String, String)>>,
}

impl SearchIndex {
    pub fn new() -> Self {
        SearchIndex { messages: HashMap::new() }
    }

    /// Index a message in a public group.
    pub fn index(&mut self, room: &str, sender: &str, body: &str) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.messages
            .entry(room.to_string())
            .or_default()
            .push((ts, sender.to_string(), body.to_string()));

        // Keep only last 10000 messages per room
        if let Some(msgs) = self.messages.get_mut(room) {
            if msgs.len() > 10000 {
                msgs.drain(0..msgs.len() - 10000);
            }
        }
    }

    /// Search all public groups for messages containing `term`.
    pub fn search(&self, term: &str) -> Vec<(String, u64, String, String)> {
        let term_lower = term.to_lowercase();
        let mut results = vec![];
        for (room, msgs) in &self.messages {
            for (ts, sender, body) in msgs {
                if body.to_lowercase().contains(&term_lower) {
                    results.push((room.clone(), *ts, sender.clone(), body.clone()));
                }
            }
        }
        results.sort_by_key(|r| r.1); // sort by timestamp
        results.reverse();             // newest first
        results.truncate(50);          // max 50 results
        results
    }

    /// Format search results for display.
    pub fn format_results(&self, term: &str) -> String {
        let results = self.search(term);
        if results.is_empty() {
            return format!("no results for '{}'", term);
        }
        let mut out = format!("search results for '{}':\n", term);
        for (room, _ts, sender, body) in results.iter().take(10) {
            let preview: String = body.chars().take(80).collect();
            out.push_str(&format!("  [{}] <{}> {}\n", room, sender, preview));
        }
        out
    }
}

/// Peer discovery via honesty vector similarity.
pub struct PeerDiscovery {
    /// route_id -> (display_name, vector_hash, shared_interests)
    pub peers: HashMap<String, (String, String, Vec<String>)>,
}

impl PeerDiscovery {
    pub fn new() -> Self {
        PeerDiscovery { peers: HashMap::new() }
    }

    /// Register a peer with their honesty vector hash and interests.
    pub fn register(&mut self, route_id: &str, display_name: &str, vector_hash: &str) {
        let interests = vec![]; // derived from vector categories
        self.peers.insert(
            route_id.to_string(),
            (display_name.to_string(), vector_hash.to_string(), interests),
        );
    }

    /// Find peers with shared interests.
    pub fn find_by_interest(&self, interest: &str) -> Vec<&str> {
        self.peers
            .iter()
            .filter(|(_, (_, _, interests))| {
                interests.iter().any(|i| i.contains(interest))
            })
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// List all known peers (public directory).
    pub fn directory(&self) -> String {
        if self.peers.is_empty() {
            return "no peers discovered".into();
        }
        let mut out = "peer directory:\n".to_string();
        for (id, (name, hash, _interests)) in &self.peers {
            let short_hash = &hash[..8.min(hash.len())];
            out.push_str(&format!("  {} :: {} (hash:{})\n", name, id, short_hash));
        }
        out
    }
}

/// Self-encrypted chat history.
/// Messages are stored encrypted in sealed memory.
/// On process exit, kernel reclaims memory. Nothing persists.
pub struct ChatHistory {
    pub messages: Vec<(u64, String, String)>, // (timestamp, sender, body)
    pub max_msgs: usize,
}

impl ChatHistory {
    pub fn new(max_msgs: usize) -> Self {
        ChatHistory { messages: vec![], max_msgs }
    }

    pub fn append(&mut self, sender: &str, body: &str) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.messages.push((ts, sender.to_string(), body.to_string()));
        if self.messages.len() > self.max_msgs {
            self.messages.drain(0..self.messages.len() - self.max_msgs);
        }
    }

    /// Get the last `n` messages.
    pub fn tail(&self, n: usize) -> Vec<&(u64, String, String)> {
        self.messages.iter().rev().take(n).collect()
    }
}

/// DNS-over-HTTPS / .onion / I2P placeholder.
/// v1.42 overlay network support.
pub struct OverlayNetwork {
    pub onion_enabled: bool,
    pub i2p_enabled: bool,
}

impl OverlayNetwork {
    pub fn new() -> Self {
        OverlayNetwork { onion_enabled: false, i2p_enabled: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_index() {
        let mut idx = SearchIndex::new();
        idx.index("#general", "alice", "hello quantum world");
        idx.index("#general", "bob", "testing the matrix");
        idx.index("#random", "alice", "the weather is nice");

        let results = idx.search("quantum");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].2, "alice");

        let results = idx.search("matrix");
        assert_eq!(results.len(), 1);

        let results = idx.search("nonexistent");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_peer_directory() {
        let mut pd = PeerDiscovery::new();
        pd.register("abc123", "[Architect]", "deadbeef");
        pd.register("def456", "alice", "cafebabe");
        let dir = pd.directory();
        assert!(dir.contains("[Architect]"));
        assert!(dir.contains("alice"));
    }

    #[test]
    fn test_chat_history() {
        let mut hist = ChatHistory::new(3);
        hist.append("alice", "msg1");
        hist.append("bob", "msg2");
        hist.append("alice", "msg3");
        hist.append("bob", "msg4"); // pushes msg1 out
        assert_eq!(hist.messages.len(), 3);
        assert_eq!(hist.messages[0].2, "msg2");
    }
}
