use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tilcayo_core::UserId;

const ACCESS_TOKEN_TTL: u64 = 15 * 60;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: usize,
    pub iat: u64,
    pub exp: u64,
}

pub fn create_access_token(
    user_id: &UserId,
    secret: &[u8],
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before Unix epoch")
        .as_secs();

    let claims = Claims {
        sub: user_id.0,
        iat: now,
        exp: now + ACCESS_TOKEN_TTL,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret),
    )
}

pub fn verify_access_token(
    token: &str,
    secret: &[u8],
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let validation = Validation::new(Algorithm::HS256);

    let token = decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)?;

    Ok(token.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tilcayo_core::UserId;

    const SECRET: &[u8] = b"test-secret";

    #[test]
    fn access_token_can_be_created_and_verified() {
        let user_id = UserId(42);

        let token = create_access_token(&user_id, SECRET).expect("failed to create access token");

        let claims = verify_access_token(&token, SECRET).expect("failed to verify access token");

        assert_eq!(claims.sub, 42);
    }

    #[test]
    fn access_token_with_wrong_secret_is_rejected() {
        let user_id = UserId(42);

        let token = create_access_token(&user_id, SECRET).expect("failed to create access token");

        let result = verify_access_token(&token, b"wrong-secret");

        assert!(result.is_err());
    }

    #[test]
    fn invalid_access_token_is_rejected() {
        let result = verify_access_token("not-a-jwt", SECRET);

        assert!(result.is_err());
    }
}
