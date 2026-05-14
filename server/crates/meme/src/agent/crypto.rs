use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use anyhow::{anyhow, Result};
use crypto::{digest::Digest, sha2::Sha256};
use rand::Rng;
const MAGIC: &[u8; 4] = b"MEME";
const NONCE_SIZE: usize = 12;
const KEY_DOMAIN: &str = "meme-password-v1";

pub fn derive_key(password: &str, owner: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.input_str(password);
    hasher.input_str(owner);
    hasher.input_str(KEY_DOMAIN);
    let mut key = [0u8; 32];
    hasher.result(&mut key);
    key
}

pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    let nonce_bytes: [u8; NONCE_SIZE] = rand::rng().random();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| anyhow!("invalid key"))?;
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow!("encryption failed: {}", e))?;
    let mut result = Vec::with_capacity(MAGIC.len() + NONCE_SIZE + ciphertext.len());
    result.extend_from_slice(MAGIC);
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

pub fn decrypt(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    if data.len() < MAGIC.len() + NONCE_SIZE + 16 {
        return Err(anyhow!("data too short"));
    }
    if &data[..MAGIC.len()] != MAGIC {
        return Err(anyhow!("not encrypted"));
    }
    let nonce = Nonce::from_slice(&data[MAGIC.len()..MAGIC.len() + NONCE_SIZE]);
    let ciphertext = &data[MAGIC.len() + NONCE_SIZE..];
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| anyhow!("invalid key"))?;
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("decryption failed: {}", e))
}

pub fn is_encrypted(data: &[u8]) -> bool {
    data.len() >= MAGIC.len() && &data[..MAGIC.len()] == MAGIC
}
