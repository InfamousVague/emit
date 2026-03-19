use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::digest;
use std::path::PathBuf;

const ENC_PREFIX: &str = "enc:";
const NONCE_LEN: usize = 12;

/// Fields that should be encrypted when stored on disk.
const SECRET_FIELDS: &[&str] = &["api_key"];

/// Derive a 256-bit key from machine-specific identifiers.
fn derive_key() -> LessSafeKey {
    let hostname = hostname::get()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .to_string_lossy()
        .to_string();

    let material = format!("emit-secret-key:{hostname}:{config_dir}");
    let hash = digest::digest(&digest::SHA256, material.as_bytes());

    let unbound = UnboundKey::new(&AES_256_GCM, hash.as_ref()).expect("valid key length");
    LessSafeKey::new(unbound)
}

/// Encrypt a plaintext string, returning a base64 string prefixed with `enc:`.
fn encrypt(plaintext: &str) -> Result<String, String> {
    let key = derive_key();
    let rng = ring::rand::SystemRandom::new();
    let mut nonce_bytes = [0u8; NONCE_LEN];
    ring::rand::SecureRandom::fill(&rng, &mut nonce_bytes)
        .map_err(|_| "Failed to generate nonce".to_string())?;

    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = plaintext.as_bytes().to_vec();
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| "Encryption failed".to_string())?;

    // Prepend nonce to ciphertext+tag
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&in_out);

    Ok(format!("{ENC_PREFIX}{}", B64.encode(&combined)))
}

/// Decrypt a value previously encrypted with `encrypt`. Input must start with `enc:`.
fn decrypt(encoded: &str) -> Result<String, String> {
    let b64 = encoded
        .strip_prefix(ENC_PREFIX)
        .ok_or("Missing enc: prefix")?;
    let combined = B64.decode(b64).map_err(|e| format!("Base64 decode error: {e}"))?;

    if combined.len() < NONCE_LEN + AES_256_GCM.tag_len() {
        return Err("Ciphertext too short".to_string());
    }

    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LEN);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes.try_into().unwrap());
    let key = derive_key();

    let mut buf = ciphertext.to_vec();
    let plaintext = key
        .open_in_place(nonce, Aad::empty(), &mut buf)
        .map_err(|_| "Decryption failed — key may have changed".to_string())?;

    String::from_utf8(plaintext.to_vec()).map_err(|e| format!("UTF-8 error: {e}"))
}

/// Encrypt secret fields in a settings Value before writing to disk.
pub fn encrypt_secrets(settings: &serde_json::Value) -> serde_json::Value {
    let mut out = settings.clone();
    if let Some(obj) = out.as_object_mut() {
        for &field in SECRET_FIELDS {
            if let Some(val) = obj.get(field).and_then(|v| v.as_str()) {
                if !val.is_empty() && !val.starts_with(ENC_PREFIX) {
                    if let Ok(encrypted) = encrypt(val) {
                        obj.insert(field.to_string(), serde_json::Value::String(encrypted));
                    }
                }
            }
        }
    }
    out
}

/// Decrypt secret fields in a settings Value after reading from disk.
pub fn decrypt_secrets(settings: &serde_json::Value) -> serde_json::Value {
    let mut out = settings.clone();
    if let Some(obj) = out.as_object_mut() {
        for &field in SECRET_FIELDS {
            if let Some(val) = obj.get(field).and_then(|v| v.as_str()) {
                if val.starts_with(ENC_PREFIX) {
                    if let Ok(plaintext) = decrypt(val) {
                        obj.insert(field.to_string(), serde_json::Value::String(plaintext));
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let original = "ntn_test_abc123";
        let encrypted = encrypt(original).unwrap();
        assert!(encrypted.starts_with(ENC_PREFIX));
        assert_ne!(encrypted, original);
        let decrypted = decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_encrypt_secrets_settings() {
        let settings = serde_json::json!({
            "api_key": "secret-token",
            "database_id": "plain-value"
        });
        let encrypted = encrypt_secrets(&settings);
        let api_key = encrypted["api_key"].as_str().unwrap();
        assert!(api_key.starts_with(ENC_PREFIX));
        assert_eq!(encrypted["database_id"].as_str().unwrap(), "plain-value");
    }

    #[test]
    fn test_decrypt_secrets_settings() {
        let settings = serde_json::json!({
            "api_key": "secret-token",
            "database_id": "plain-value"
        });
        let encrypted = encrypt_secrets(&settings);
        let decrypted = decrypt_secrets(&encrypted);
        assert_eq!(decrypted["api_key"].as_str().unwrap(), "secret-token");
        assert_eq!(decrypted["database_id"].as_str().unwrap(), "plain-value");
    }

    #[test]
    fn test_already_encrypted_not_double_encrypted() {
        let settings = serde_json::json!({ "api_key": "secret" });
        let encrypted = encrypt_secrets(&settings);
        let double_encrypted = encrypt_secrets(&encrypted);
        // Should be the same — already-encrypted values are skipped
        assert_eq!(encrypted["api_key"], double_encrypted["api_key"]);
    }
}
