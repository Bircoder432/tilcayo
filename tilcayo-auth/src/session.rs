use std::time::Duration;

use tilcayo_core::UserId;
use tilcayo_db::sessions::SessionStore;

use crate::refresh_token::{generate_refresh_token, hash_refresh_token};

const REFRESH_TOKEN_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);

pub struct RefreshSession {
    pub session_id: String,
    pub refresh_token: String,
}

pub struct SessionService {
    store: SessionStore,
}

impl SessionService {
    pub fn new(store: SessionStore) -> Self {
        Self { store }
    }

    pub async fn create(
        &self,
        user_id: &UserId,
        session_id: &str,
    ) -> Result<RefreshSession, SessionError> {
        let refresh_token = generate_refresh_token().map_err(SessionError::TokenGeneration)?;

        let refresh_token_hash = hash_refresh_token(&refresh_token);

        self.store
            .create(session_id, user_id, &refresh_token_hash, REFRESH_TOKEN_TTL)
            .await
            .map_err(SessionError::Store)?;

        Ok(RefreshSession {
            session_id: session_id.to_owned(),
            refresh_token,
        })
    }
    pub async fn refresh(
        &self,
        session_id: &str,
        refresh_token: &str,
    ) -> Result<Option<RefreshSession>, SessionError> {
        let Some(session) = self
            .store
            .find(session_id)
            .await
            .map_err(SessionError::Store)?
        else {
            return Ok(None);
        };

        let token_hash = hash_refresh_token(refresh_token);

        if token_hash != session.refresh_token_hash {
            return Ok(None);
        }

        let new_refresh_token = generate_refresh_token().map_err(SessionError::TokenGeneration)?;

        let new_refresh_token_hash = hash_refresh_token(&new_refresh_token);

        let updated = self
            .store
            .update_refresh_token(session_id, &new_refresh_token_hash, REFRESH_TOKEN_TTL)
            .await
            .map_err(SessionError::Store)?;

        if !updated {
            return Ok(None);
        }

        Ok(Some(RefreshSession {
            session_id: session_id.to_owned(),
            refresh_token: new_refresh_token,
        }))
    }
}

#[derive(Debug)]
pub enum SessionError {
    TokenGeneration(getrandom::Error),
    Store(redis::RedisError),
}
