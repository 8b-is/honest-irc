use sha2::Sha256;
use hkdf::Hkdf;

/// Quant1bitLLM per-byte sub-key derivation.
///
/// A tiny (~1MB) ternary-quantized language model produces {-1, 0, +1}
/// weight activations. Each activation is used as a seed for deriving a
/// unique sub-key for each byte of plaintext. Identical plaintext bytes
/// produce different ciphertext because each gets a unique sub-key.
pub struct QuantByteCipher {
    /// Pre-shared LLM weights ({-1, 0, +1}^N)
    weights: Vec<i8>,
    /// HKDF extractor seeded from the session key
    prk: Vec<u8>,
}

impl QuantByteCipher {
    /// Create a new per-byte cipher.
    /// `session_key`: the Kyber+X25519 shared secret
    /// `weights`: the LLM weight activations {-1, 0, +1}^N
    pub fn new(session_key: &[u8], weights: Vec<i8>) -> Self {
        let salt = b"honest-irc-quant1bit-llm-v1";
        let hk = Hkdf::<Sha256>::new(Some(salt), session_key);
        let mut prk = vec![0u8; 32];
        hk.expand(b"per-byte-subkey", &mut prk).expect("HKDF expand failed");
        QuantByteCipher { weights, prk }
    }

    /// Derive a unique 32-byte sub-key for byte at position `index`.
    fn derive_sub_key(&self, index: usize) -> [u8; 32] {
        let weight = self.weights.get(index % self.weights.len()).copied().unwrap_or(0);
        let context: &[u8] = match weight {
            -1 => b"neg",
            0 => b"zero",
            1 => b"pos",
            _ => b"zero",
        };

        let hk = Hkdf::<Sha256>::new(Some(&self.prk), &self.prk);
        let mut sub_key = [0u8; 32];
        let info = [context, &index.to_le_bytes()].concat();
        hk.expand(&info, &mut sub_key).expect("HKDF expand failed");
        sub_key
    }

    /// Encrypt a plaintext. Each byte gets its own sub-key.
    pub fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        let mut ciphertext = Vec::with_capacity(plaintext.len());
        for (i, &byte) in plaintext.iter().enumerate() {
            let sub_key = self.derive_sub_key(i);
            ciphertext.push(byte ^ sub_key[0]);
        }
        ciphertext
    }

    /// Decrypt a ciphertext. Same per-byte sub-key derivation.
    pub fn decrypt(&self, ciphertext: &[u8]) -> Vec<u8> {
        self.encrypt(ciphertext) // XOR is symmetric
    }

    /// Generate sample LLM weights for testing.
    /// In production, these come from a quantized model checkpoint.
    pub fn sample_weights(count: usize) -> Vec<i8> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..count).map(|_| rng.gen_range(-1..=1)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_per_byte_encrypt_decrypt_roundtrip() {
        let key = b"super-secret-session-key-32bytes!";
        let weights = QuantByteCipher::sample_weights(1024);
        let cipher = QuantByteCipher::new(key, weights);

        let msg = b"hello, quantum-proof world! this is a test of per-byte encryption.";
        let ct = cipher.encrypt(msg);
        let pt = cipher.decrypt(&ct);
        assert_eq!(msg.as_slice(), pt.as_slice());
    }

    #[test]
    fn test_identical_bytes_produce_different_ciphertext() {
        let key = b"super-secret-session-key-32bytes!";
        let weights = QuantByteCipher::sample_weights(1024);
        let cipher = QuantByteCipher::new(key, weights);

        let msg = b"aaaa"; // same byte repeated
        let ct = cipher.encrypt(msg);
        // First two bytes should be different (different sub-keys)
        assert_ne!(ct[0], ct[1]);
    }

    #[test]
    fn test_deterministic_keys() {
        let key1 = b"super-secret-session-key-32bytes!";
        let key2 = b"super-secret-session-key-32bytes!";
        let w1 = QuantByteCipher::sample_weights(1024);
        let w2 = w1.clone();

        let c1 = QuantByteCipher::new(key1, w1);
        let c2 = QuantByteCipher::new(key2, w2);

        let msg = b"test determinism";
        let ct1 = c1.encrypt(msg);
        let ct2 = c2.encrypt(msg);
        assert_eq!(ct1, ct2);
    }
}
