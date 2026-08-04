use anyhow::{Context, Result};
use crypto::{digest::Digest, sha2::Sha256};
use jsonwebtoken::{EncodingKey, Header};
use rand::Rng;
use serde::Serialize;

const CHARSET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789"; // 28 chars, no 0/O/1/I/L

/// Generate a random one-time code of given length
pub fn generate_code(length: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

/// SHA256 hash of a code — used as the sled key for invites
pub fn hash_code(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.input_str(code);
    hasher.result_str()
}

#[derive(Serialize)]
struct TokenClaims {
    n: String,   // name
    f: String,   // family
    exp: usize,  // expiration
    iat: usize,  // issued at
}

/// Sign a JWT with ES256 key (same key pair as meme uses for verification).
/// `expire` 是有效秒数（前端 raven 传 `?e=`），默认 1h。
pub fn sign_jwt(name: &str, family: &str, key: &EncodingKey, expire: Option<u64>) -> Result<String> {
    let now = chrono::Utc::now().timestamp() as usize;
    let ttl = expire.unwrap_or(3600) as usize; // default 1 hour
    let claims = TokenClaims {
        n: name.to_string(),
        f: family.to_string(),
        iat: now,
        exp: now + ttl,
    };
    jsonwebtoken::encode(&Header::new(jsonwebtoken::Algorithm::ES256), &claims, key)
        .with_context(|| "JWT signing failed")
}

/// Load ES256 private key from base64 PEM (same format as meme.jwt_secret_key)
pub fn load_signing_key(pem_b64: &str) -> Result<EncodingKey> {
    let pem_bytes =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, pem_b64)
            .with_context(|| "Failed to decode base64 JWT secret key")?;
    EncodingKey::from_ec_pem(&pem_bytes).with_context(|| "Failed to parse EC private key")
}
