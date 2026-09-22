use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let hash = Argon2::default()
        .hash_password(password.as_bytes())?
        .to_string();

    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let hash = PasswordHash::new(hash)?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_can_be_verified() {
        let password = "correct horse battery staple";

        let hash = hash_password(password).expect("failed to hash password");

        assert!(verify_password(password, &hash).expect("failed to verify password"));
    }

    #[test]
    fn wrong_password_is_rejected() {
        let hash = hash_password("correct password").expect("failed to hash password");

        assert!(!verify_password("wrong password", &hash).expect("failed to verify password"));
    }
}
