use sha2::{Digest, Sha256};
use argon2::{Argon2, password_hash::{SaltString, PasswordHasher, PasswordHash, PasswordVerifier}};
use rand::rngs::OsRng;

pub const FINGERPRINT_LEN: usize = 16; // characters

pub fn compute_fingerprint(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let digest = hasher.finalize();
    let hex = hex::encode(digest);
    hex.chars().take(FINGERPRINT_LEN).collect()
}

pub fn hash_key(key: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(key.as_bytes(), &salt)?;
    Ok(password_hash.to_string())
}

pub fn verify_key(key: &str, stored_hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed = PasswordHash::new(stored_hash)?;
    let verified = Argon2::default().verify_password(key.as_bytes(), &parsed).is_ok();
    Ok(verified)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_length() {
        let key = "mysecretkey123";
        let fp = compute_fingerprint(key);
        assert_eq!(fp.len(), FINGERPRINT_LEN);
    }

    #[test]
    fn hash_and_verify() {
        let key = "another-secret-456";
        let hashed = hash_key(key).expect("hash should succeed");
        let ok = verify_key(key, &hashed).expect("verify should run");
        assert!(ok);
        let bad = verify_key("wrong", &hashed).expect("verify should run");
        assert!(!bad);
    }
}
