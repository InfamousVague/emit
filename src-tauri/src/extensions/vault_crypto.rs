use argon2::{Argon2, Algorithm, Params, Version};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

const NONCE_LEN: usize = 12;
const SALT_LEN: usize = 32;
const KEY_LEN: usize = 32;

/// Argon2id parameters: 64 MB memory, 3 iterations, 1 thread.
fn argon2_params() -> Params {
    Params::new(65536, 3, 1, Some(KEY_LEN)).expect("valid argon2 params")
}

/// Generate a random 32-byte salt.
pub fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

/// Derive a 256-bit key from password + salt using Argon2id.
pub fn derive_key(password: &str, salt: &[u8]) -> Result<Vec<u8>, String> {
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params());
    let mut key = vec![0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Key derivation failed: {e}"))?;
    Ok(key)
}

/// Encrypt plaintext bytes with ChaCha20-Poly1305. Returns nonce + ciphertext.
pub fn encrypt(plaintext: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| format!("Invalid key: {e}"))?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {e}"))?;

    let mut output = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

/// Decrypt nonce + ciphertext with ChaCha20-Poly1305.
pub fn decrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < NONCE_LEN + 16 {
        return Err("Ciphertext too short".to_string());
    }

    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| format!("Invalid key: {e}"))?;

    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Decryption failed — wrong password or corrupted vault".to_string())
}

/// A single generated password stored in history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordHistoryEntry {
    pub id: String,
    pub password: String,
    pub generated_at: u64,
    pub label: Option<String>,
    pub mode: String,
    pub length: usize,
}

/// The vault file format: salt (32 bytes) + encrypted blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultData {
    pub passwords: Vec<PasswordHistoryEntry>,
    pub version: u32,
}

impl Default for VaultData {
    fn default() -> Self {
        Self {
            passwords: Vec::new(),
            version: 1,
        }
    }
}

/// Encrypt VaultData to bytes (salt + ciphertext).
pub fn encrypt_vault(data: &VaultData, password: &str) -> Result<Vec<u8>, String> {
    let salt = generate_salt();
    let mut key = derive_key(password, &salt)?;

    let json = serde_json::to_vec(data).map_err(|e| format!("Serialization failed: {e}"))?;
    let encrypted = encrypt(&json, &key)?;

    key.zeroize();

    let mut output = Vec::with_capacity(SALT_LEN + encrypted.len());
    output.extend_from_slice(&salt);
    output.extend_from_slice(&encrypted);
    Ok(output)
}

/// Decrypt vault bytes (salt + ciphertext) to VaultData.
pub fn decrypt_vault(data: &[u8], password: &str) -> Result<(VaultData, Vec<u8>), String> {
    if data.len() < SALT_LEN + NONCE_LEN + 16 {
        return Err("Vault file too short".to_string());
    }

    let (salt, ciphertext) = data.split_at(SALT_LEN);
    let mut key = derive_key(password, salt)?;

    let plaintext = decrypt(ciphertext, &key)?;
    let vault: VaultData =
        serde_json::from_slice(&plaintext).map_err(|e| format!("Vault parse error: {e}"))?;

    let key_copy = key.clone();
    key.zeroize();

    Ok((vault, key_copy))
}

/// Re-encrypt vault with an existing derived key (for saves after unlock).
pub fn reencrypt_vault(data: &VaultData, key: &[u8], salt: &[u8]) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec(data).map_err(|e| format!("Serialization failed: {e}"))?;
    let encrypted = encrypt(&json, key)?;

    let mut output = Vec::with_capacity(SALT_LEN + encrypted.len());
    output.extend_from_slice(salt);
    output.extend_from_slice(&encrypted);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = b"hello world";
        let key = [42u8; 32];
        let encrypted = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let plaintext = b"secret";
        let key = [42u8; 32];
        let wrong_key = [99u8; 32];
        let encrypted = encrypt(plaintext, &key).unwrap();
        assert!(decrypt(&encrypted, &wrong_key).is_err());
    }

    #[test]
    fn test_vault_roundtrip() {
        let vault = VaultData {
            passwords: vec![PasswordHistoryEntry {
                id: "test-1".into(),
                password: "s3cret!Pass".into(),
                generated_at: 1000,
                label: Some("Test Entry".into()),
                mode: "random".into(),
                length: 11,
            }],
            version: 1,
        };

        let password = "master-password-123";
        let encrypted = encrypt_vault(&vault, password).unwrap();
        let (decrypted, _key) = decrypt_vault(&encrypted, password).unwrap();

        assert_eq!(decrypted.passwords.len(), 1);
        assert_eq!(decrypted.passwords[0].password, "s3cret!Pass");
        assert_eq!(decrypted.passwords[0].label.as_deref(), Some("Test Entry"));
    }

    #[test]
    fn test_wrong_password_fails() {
        let vault = VaultData::default();
        let encrypted = encrypt_vault(&vault, "correct").unwrap();
        assert!(decrypt_vault(&encrypted, "wrong").is_err());
    }
}
