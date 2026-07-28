use honest_irc::crypto::CryptoSession;

fn main() {
    println!("honest-crypt :: Kyber-1024 + X25519 hybrid encryption");
    println!("  KEM     : ML-KEM-1024 (post-quantum)");
    println!("  ECDH    : X25519 (classical)");
    println!("  sym     : ChaCha20Poly1305");
    println!("  sig     : ML-DSA-87 (post-quantum)");

    let session = CryptoSession::new();
    let pk = session.public_key_bytes();
    println!("  pubkey  : {}", hex::encode(&pk[..8]));

    // In production: exchange keys with peer, encrypt/decrypt all traffic
    // before passing to Tailscale mesh

    println!("  [OK] crypto engine ready");
}
