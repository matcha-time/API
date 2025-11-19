use rand::Rng;
use sha2::{Digest, Sha256};

/// Generate a secure random token
pub fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let token_bytes: [u8; 32] = rng.r#gen();
    hex::encode(token_bytes)
}

/// Hash a token for secure storage in the database
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
