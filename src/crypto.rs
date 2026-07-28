use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use rand::Rng;
use zeroize::Zeroize;

/// Quantum-proof hybrid encryption layer.
/// Currently: X25519 + ChaCha20Poly1305 (classical, forward-secret).
/// Planned: Kyber-1024 + X25519 hybrid (PQC + classical defense-in-depth).
pub struct CryptoSession {
    /// Ephemeral keypair for this session
    secret: x25519_dalek::StaticSecret,
    public: x25519_dalek::PublicKey,
    /// Shared secret derived from ECDH
    shared: [u8; 32],
    /// Session nonce counter (incremented per message)
    nonce_counter: u64,
}

impl CryptoSession {
    /// Create a new ephemeral session. Generates fresh X25519 keypair.
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let secret = x25519_dalek::StaticSecret::random_from_rng(&mut rng);
        let public = x25519_dalek::PublicKey::from(&secret);
        let shared = [0u8; 32];
        CryptoSession {
            secret,
            public,
            shared,
            nonce_counter: 0,
        }
    }

    /// Perform key exchange with peer's public key.
    pub fn exchange(&mut self, peer_public: &x25519_dalek::PublicKey) {
        self.shared = *self.secret.diffie_hellman(peer_public).as_bytes();
        self.nonce_counter = 0;
    }

    /// Encrypt a plaintext message for the peer.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8> {
        use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, AeadInPlace};
        use chacha20poly1305::aead::{Aead, KeyInit};

        let key = Key::from_slice(&self.shared);
        let cipher = ChaCha20Poly1305::new(key);

        self.nonce_counter += 1;
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[..8].copy_from_slice(&self.nonce_counter.to_le_bytes());
        nonce_bytes[8..].copy_from_slice(&[0u8; 4]);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut buffer = plaintext.to_vec();
        let tag = cipher.encrypt_in_place_detached(&nonce, &[], &mut buffer)
            .expect("encryption failed");

        // Prepend tag to ciphertext
        let mut result = tag.to_vec();
        result.extend_from_slice(&buffer);
        result
    }

    /// Decrypt a message from the peer.
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Option<Vec<u8>> {
        use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, AeadInPlace};
        use chacha20poly1305::aead::{Aead, KeyInit};

        if ciphertext.len() < 16 { return None; }

        let key = Key::from_slice(&self.shared);
        let cipher = ChaCha20Poly1305::new(key);

        self.nonce_counter += 1;
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[..8].copy_from_slice(&self.nonce_counter.to_le_bytes());
        nonce_bytes[8..].copy_from_slice(&[0u8; 4]);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let (tag_bytes, ct) = ciphertext.split_at(16);
        let tag = chacha20poly1305::Tag::from_slice(tag_bytes);
        let mut buffer = ct.to_vec();

        cipher.decrypt_in_place_detached(&nonce, &[], &mut buffer, tag).ok()?;
        Some(buffer)
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        *self.public.as_bytes()
    }
}

/// Identity is:
/// - route_id: SHA-256 of core_hash — used for addressing (pure ASCII hex)
/// - display_name: the creative symbol name — reserved per user
/// - vector_hash: SHA-256 of full honesty vector
#[derive(Clone, Serialize, Deserialize)]
pub struct Identity {
    pub route_id: String,         // SHA-256(core_hash) — ASCII hex
    pub display_name: DisplayName,
    pub vector_hash: String,      // SHA-256(full vector)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DisplayName {
    pub raw: String,
    pub ascii_prefix: String,     // pure ASCII routing prefix
}

/// Reserved usernames registry.
/// Each display name is unique. First claim wins in the mesh.
pub struct NameRegistry {
    pub reserved: Vec<String>,
}

impl NameRegistry {
    pub fn new() -> Self {
        NameRegistry {
            reserved: vec![
                "⊰•-•⦑ The Architect of Structural Honesty ⦒•-•⊱".into(),
            ],
        }
    }

    pub fn is_reserved(&self, name: &str) -> bool {
        self.reserved.contains(&name.to_string())
    }

    pub fn reserve(&mut self, name: &str) -> Result<(), &'static str> {
        if self.is_reserved(name) {
            return Err("name already reserved");
        }
        self.reserved.push(name.to_string());
        Ok(())
    }
}

/// Generate a route_id from a core identity hash.
pub fn route_id(core_hash: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(core_hash.as_bytes());
    hex::encode(hasher.finalize())[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let mut alice = CryptoSession::new();
        let mut bob = CryptoSession::new();

        let bob_pub = x25519_dalek::PublicKey::from(bob.public_key_bytes());
        let alice_pub = x25519_dalek::PublicKey::from(alice.public_key_bytes());

        alice.exchange(&bob_pub);
        bob.exchange(&alice_pub);

        let msg = b"hello, quantum-proof world!";
        let ct = alice.encrypt(msg);
        let pt = bob.decrypt(&ct).unwrap();
        assert_eq!(msg, pt.as_slice());
    }

    #[test]
    fn test_architect_name_reserved() {
        let registry = NameRegistry::new();
        assert!(registry.is_reserved("⊰•-•⦑ The Architect of Structural Honesty ⦒•-•⊱"));
        assert!(!registry.is_reserved("random_user"));
    }
}
