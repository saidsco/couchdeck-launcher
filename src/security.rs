//! Local admin PIN hashing.
//!
//! The PIN is only a UI guard against accidental changes. It is not an OS
//! security boundary: anyone with access to the same Linux account can modify
//! the files directly. PINs are stored exclusively as Argon2id PHC strings.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

use crate::model::AdminSecrets;

pub const MIN_PIN_LEN: usize = 4;
pub const MAX_PIN_LEN: usize = 12;

pub fn validate_pin(pin: &str) -> Result<(), String> {
    if !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err("Die PIN darf nur Ziffern enthalten.".into());
    }
    if pin.len() < MIN_PIN_LEN {
        return Err(format!(
            "Die PIN muss mindestens {MIN_PIN_LEN} Ziffern haben."
        ));
    }
    if pin.len() > MAX_PIN_LEN {
        return Err(format!(
            "Die PIN darf höchstens {MAX_PIN_LEN} Ziffern haben."
        ));
    }
    Ok(())
}

pub fn hash_pin(pin: &str) -> Result<String, String> {
    validate_pin(pin)?;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pin.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| format!("PIN konnte nicht gehasht werden: {err}"))
}

pub fn verify_pin(pin: &str, phc_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(phc_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(pin.as_bytes(), &parsed)
        .is_ok()
}

impl AdminSecrets {
    pub fn has_pin(&self, profile_id: &str) -> bool {
        self.pins.contains_key(profile_id)
    }

    pub fn set_pin(&mut self, profile_id: &str, pin: &str) -> Result<(), String> {
        let hash = hash_pin(pin)?;
        self.pins.insert(profile_id.to_owned(), hash);
        Ok(())
    }

    pub fn remove_pin(&mut self, profile_id: &str) -> bool {
        self.pins.remove(profile_id).is_some()
    }

    pub fn verify(&self, profile_id: &str, pin: &str) -> bool {
        self.pins
            .get(profile_id)
            .map(|hash| verify_pin(pin, hash))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_numeric_length() {
        assert!(validate_pin("1234").is_ok());
        assert!(validate_pin("123").is_err());
        assert!(validate_pin("abcd").is_err());
        assert!(validate_pin("1234567890123").is_err());
    }

    #[test]
    fn hash_is_not_plaintext_and_verifies() {
        let mut secrets = AdminSecrets::new();
        secrets.set_pin("admin", "4711").unwrap();
        let hash = secrets.pins.get("admin").unwrap();
        assert!(!hash.contains("4711"));
        assert!(hash.starts_with("$argon2id$"));
        assert!(secrets.verify("admin", "4711"));
        assert!(!secrets.verify("admin", "4712"));
        assert!(!secrets.verify("other", "4711"));
        assert!(secrets.remove_pin("admin"));
        assert!(!secrets.has_pin("admin"));
    }
}
