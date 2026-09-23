use tilcayo_core::{User, UserId};
use tilcayo_db::repositories::user::UserRepository;

use crate::password::verify_password;

pub struct AuthService<'a> {
    users: UserRepository<'a>,
}

impl<'a> AuthService<'a> {
    pub fn new(users: UserRepository<'a>) -> Self {
        Self { users }
    }

    pub async fn authenticate(
        &self,
        user_id: &UserId,
        password: &str,
    ) -> Result<Option<User>, AuthError> {
        let Some(user) = self
            .users
            .find_by_id(user_id)
            .await
            .map_err(AuthError::Database)?
        else {
            return Ok(None);
        };

        let valid = verify_password(password, &user.password_hash).map_err(AuthError::Password)?;

        if !valid {
            return Ok(None);
        }

        Ok(Some(user))
    }
}

#[derive(Debug)]
pub enum AuthError {
    Database(sqlx::Error),
    Password(argon2::password_hash::Error),
}
