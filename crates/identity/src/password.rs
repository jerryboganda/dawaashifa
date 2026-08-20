use crate::error::AuthError;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params,
};

/// List of common insecure passwords to reject during validation.
const COMMON_PASSWORDS: &[&str] = &[
    "password123",
    "password",
    "1234567890",
    "12345678",
    "qwerty1234",
    "admin12345",
    "welcome123",
    "pakistan123",
    "shifa12345",
    "karachi123",
];

/// Hash a password using Argon2id with parameters:
/// memory = 19456 KiB, iterations = 2, parallelism = 1
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    validate_password_strength(password)?;

    let params = Params::new(19456, 2, 1, Some(32))
        .map_err(|e| AuthError::WeakPassword(format!("Invalid Argon2 params: {}", e)))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AuthError::WeakPassword(format!("Hashing failure: {}", e)))?
        .to_string();

    Ok(password_hash)
}

/// Verify a password against an Argon2id hash.
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// Validate that a password meets complexity rules:
/// - Minimum 10 characters
/// - Not in common password blacklist
pub fn validate_password_strength(password: &str) -> Result<(), AuthError> {
    if password.len() < 10 {
        return Err(AuthError::WeakPassword(
            "Password must be at least 10 characters long".to_string(),
        ));
    }
    if COMMON_PASSWORDS.contains(&password.to_lowercase().as_str()) {
        return Err(AuthError::WeakPassword(
            "Password is too common and easily guessable".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing_and_verification() {
        let raw = "SuperSecurePassword123!";
        let hash = hash_password(raw).expect("hash password");
        assert!(verify_password(raw, &hash));
        assert!(!verify_password("WrongPassword123!", &hash));
    }

    #[test]
    fn test_weak_and_short_passwords_rejected() {
        assert!(hash_password("short").is_err());
        assert!(hash_password("password123").is_err());
    }
}
