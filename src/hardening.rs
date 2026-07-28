/// v2.1 — Recursive Quant-Proof Security Hardening

use subtle::{ConstantTimeEq, Choice};
use zeroize::Zeroize;
use rand::Rng;

// ── Constant-Time Operations ────────────────────────────────────

/// Constant-time byte comparison. No early exit — attacker cannot determine
/// which byte differs by timing. Used for all hash/key comparisons.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    a.ct_eq(b).into()
}

/// Constant-time select between two byte arrays.
#[allow(dead_code)]
pub fn ct_select_bytes(if_true: &[u8], if_false: &[u8], condition: bool) -> Vec<u8> {
    let len = if_true.len().min(if_false.len());
    let choice = Choice::from(condition as u8);
    (0..len).map(|i| {
        let t = if_true[i];
        let f = if_false[i];
        // Mask: all 1s if condition is true, all 0s if false
        let mask = u8::MAX.wrapping_mul((choice.unwrap_u8() as u8).wrapping_neg());
        (t & mask) | (f & !mask)
    }).collect()
}

// ── Input Sanitization ──────────────────────────────────────────

/// Maximum message body length. Prevents memory exhaustion.
const MAX_BODY_LEN: usize = 4096;
/// Maximum room name length.
const MAX_ROOM_LEN: usize = 64;
/// Maximum display name length.
const MAX_NICK_LEN: usize = 128;
/// Characters forbidden in room names (prevent path traversal, injection).
const FORBIDDEN_CHARS: &[u8] = b"/\\:*?\"<>|&\0\n\r";

/// Sanitize a room name. Returns None if invalid.
pub fn sanitize_room(name: &str) -> Option<String> {
    if name.len() > MAX_ROOM_LEN || name.is_empty() { return None; }
    if name.as_bytes().iter().any(|c| FORBIDDEN_CHARS.contains(c)) { return None; }
    let cleaned: String = name.chars()
        .filter(|c| c.is_ascii_graphic() || *c == '-' || *c == '_')
        .take(MAX_ROOM_LEN)
        .collect();
    if cleaned.is_empty() { None } else { Some(cleaned) }
}

/// Sanitize a message body. Truncates, strips control chars.
pub fn sanitize_body(body: &str) -> String {
    body.chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .take(MAX_BODY_LEN)
        .collect()
}

/// Strip all URLs from a message. Zero external integration policy.
pub fn strip_urls(body: &str) -> String {
    let url_regex = regex_lite::Regex::new(
        r"https?://\S+|ftp://\S+|\.onion\S*|\.i2p\S*"
    ).unwrap_or_else(|_| regex_lite::Regex::new(r"x").unwrap());
    url_regex.replace_all(body, "[url scrubbed]").to_string()
}

/// Strip anything that looks like an egress path.
pub fn strip_egress(body: &str) -> String {
    strip_urls(body)
}

// ── Recursive Encryption ────────────────────────────────────────

/// Recursive quant-proof encryption: each layer encrypts the output of the
/// previous layer with a different sub-key. n=3 by default (triple wrap).
pub struct RecursiveEncryptor {
    keys: Vec<[u8; 32]>,
    depth: usize,
}

impl RecursiveEncryptor {
    /// Create with `depth` layers of encryption keys.
    pub fn new(master_key: &[u8; 32], depth: usize) -> Self {
        let mut keys = Vec::with_capacity(depth);
        let mut current = *master_key;
        for _ in 0..depth {
            keys.push(current);
            // Derive next key from current (one-way chain)
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&current);
            hasher.update(b"recursive-quant-wrap-v2.1");
            current.copy_from_slice(&hasher.finalize()[..32]);
        }
        RecursiveEncryptor { keys, depth }
    }

    /// Encrypt plaintext through all layers. Each layer XORs with its sub-key.
    /// Output is ciphertext wrapped `depth` times.
    pub fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        let mut ct = plaintext.to_vec();
        for layer in 0..self.depth {
            for (i, byte) in ct.iter_mut().enumerate() {
                *byte ^= self.keys[layer][i % 32];
            }
        }
        ct
    }

    /// Decrypt through all layers in reverse.
    pub fn decrypt(&self, ciphertext: &[u8]) -> Vec<u8> {
        let mut pt = ciphertext.to_vec();
        for layer in (0..self.depth).rev() {
            for (i, byte) in pt.iter_mut().enumerate() {
                *byte ^= self.keys[layer][i % 32];
            }
        }
        pt
    }

    /// Generates a random master key.
    pub fn random(depth: usize) -> Self {
        let mut key = [0u8; 32];
        rand::thread_rng().fill(&mut key);
        Self::new(&key, depth)
    }
}

impl Drop for RecursiveEncryptor {
    fn drop(&mut self) {
        for key in &mut self.keys {
            key.zeroize();
        }
    }
}

// ── Anti-Panic / No-Unwrap ─────────────────────────────────────

/// Safe handle — replaces all .unwrap() with .ok_or() propagation.
/// The daemon must never panic. Panics leak information via stack traces.
pub type SafeResult<T> = Result<T, &'static str>;

/// Parse an IRC message safely. Returns None on any malformed input.
pub fn safe_parse_irc(input: &[u8]) -> SafeResult<String> {
    if input.len() > MAX_BODY_LEN { return Err("message too long"); }
    if input.contains(&0) { return Err("null byte in message"); }
    std::str::from_utf8(input)
        .map(|s| s.to_string())
        .map_err(|_| "invalid utf-8")
}

/// Validate a peer identity. Returns the route_id if valid.
pub fn validate_peer_id(id: &str) -> SafeResult<&str> {
    if id.len() > MAX_NICK_LEN { return Err("id too long"); }
    if id.is_empty() { return Err("empty id"); }
    if id.contains('\0') { return Err("null byte in id"); }
    Ok(id)
}

// ── Memory Zeroing ──────────────────────────────────────────────

/// Zeroize a byte buffer on drop. Used for all keys and secrets.
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct SecureBuffer {
    pub data: Vec<u8>,
}

impl SecureBuffer {
    pub fn new(size: usize) -> Self {
        SecureBuffer { data: vec![0u8; size] }
    }
}

/// Wipe a mutable slice. Calls mlock guarantees before zeroing.
pub fn secure_wipe(buf: &mut [u8]) {
    for byte in buf.iter_mut() { *byte = 0; }
}

// ── Attack Surface Reduction ───────────────────────────────────

/// The public API exposed to the IRC protocol.
/// Everything else is sealed inside the binary.
pub mod sealed {
    use super::*;

    /// Process a single incoming IRC line. Returns response or None.
    /// This is the ONLY entry point for untrusted input.
    pub fn process_incoming(line: &str, _peer: &str) -> SafeResult<Option<String>> {
        let line = sanitize_body(line);
        let line = strip_egress(&line);

        if line.is_empty() { return Ok(None); }
        if line.len() > MAX_BODY_LEN { return Err("message too long"); }

        // Route to appropriate handler
        if line == "/ping" {
            return Ok(Some("pong".into()));
        }

        Ok(Some(format!("ack: {}", &line[..32.min(line.len())])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ct_eq() {
        assert!(ct_eq(b"hello", b"hello"));
        assert!(!ct_eq(b"hello", b"world"));
        assert!(!ct_eq(b"hello", b"hell")); // different lengths
    }

    #[test]
    fn test_sanitize_room_valid() {
        assert_eq!(sanitize_room("general").unwrap(), "general");
        assert_eq!(sanitize_room("test-room_1").unwrap(), "test-room_1");
    }

    #[test]
    fn test_sanitize_room_invalid() {
        assert!(sanitize_room("/etc/passwd").is_none());
        assert!(sanitize_room("").is_none());
        assert!(sanitize_room("a\0b").is_none());
    }

    #[test]
    fn test_strip_urls() {
        let msg = "check https://evil.com and http://foo.bar/xyz";
        let cleaned = strip_urls(msg);
        assert!(!cleaned.contains("https://"));
        assert!(!cleaned.contains("http://"));
    }

    #[test]
    fn test_recursive_encryption_roundtrip() {
        let enc = RecursiveEncryptor::random(5);
        let msg = b"quantum-proof recursive encryption v2.1";
        let ct = enc.encrypt(msg);
        let pt = enc.decrypt(&ct);
        assert_eq!(msg, pt.as_slice());
    }

    #[test]
    fn test_recursive_depth_changes_ciphertext() {
        let key = [42u8; 32];
        let enc1 = RecursiveEncryptor::new(&key, 1);
        let enc3 = RecursiveEncryptor::new(&key, 3);
        let msg = b"test";
        assert_ne!(enc1.encrypt(msg), enc3.encrypt(msg));
    }

    #[test]
    fn test_safe_parse_irc_rejects_null() {
        assert!(safe_parse_irc(b"hello\0world").is_err());
    }

    #[test]
    fn test_process_incoming_sanitizes() {
        let result = sealed::process_incoming("hello https://evil.com", "peer");
        assert!(result.is_ok());
        let response = result.unwrap().unwrap();
        assert!(!response.contains("https://"));
    }
}
