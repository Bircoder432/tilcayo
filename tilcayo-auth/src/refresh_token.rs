use sha2::{Digest, Sha256};

const REFRESH_TOKEN_LENGTH: usize = 32;

pub fn generate_refresh_token() -> Result<String, getrandom::Error> {
    let mut bytes = [0u8; REFRESH_TOKEN_LENGTH];

    getrandom::fill(&mut bytes)?;

    Ok(hex::encode(bytes))
}

pub fn hash_refresh_token(token: &str) -> String {
    let hash = Sha256::digest(token.as_bytes());

    hex::encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_token_can_be_generated() {
        let token = generate_refresh_token().expect("failed to generate refresh token");

        assert_eq!(token.len(), 64);
    }

    #[test]
    fn generated_refresh_tokens_are_different() {
        let first = generate_refresh_token().expect("failed to generate first token");

        let second = generate_refresh_token().expect("failed to generate second token");

        assert_ne!(first, second);
    }

    #[test]
    fn refresh_token_hash_is_deterministic() {
        let token = "test-refresh-token";

        let first = hash_refresh_token(token);
        let second = hash_refresh_token(token);

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn different_refresh_tokens_have_different_hashes() {
        let first = hash_refresh_token("first-token");
        let second = hash_refresh_token("second-token");

        assert_ne!(first, second);
    }
}
