use sqlx::PgPool;
use tilcayo_core::{RoleId, User, UserId};

#[derive(Clone, Copy)]
pub struct UserRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> UserRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        username: &str,
        role_id: &RoleId,
        password_hash: &str,
    ) -> Result<UserId, sqlx::Error> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO users (username, role_id, password_hash)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            username,
            role_id.0 as i64,
            password_hash,
        )
        .fetch_one(self.pool)
        .await?;

        Ok(UserId(id as usize))
    }

    pub async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, sqlx::Error> {
        let user = sqlx::query!(
            r#"
            SELECT username, role_id, password_hash
            FROM users
            WHERE id = $1
            "#,
            id.0 as i64
        )
        .fetch_optional(self.pool)
        .await?;

        let Some(user) = user else {
            return Ok(None);
        };

        Ok(Some(User {
            id: UserId(id.0),
            username: user.username,
            role_id: RoleId(user.role_id as usize),
            password_hash: user.password_hash,
        }))
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        let user = sqlx::query!(
            r#"
            SELECT id, username, role_id, password_hash
            FROM users
            WHERE username = $1
            "#,
            username,
        )
        .fetch_optional(self.pool)
        .await?;

        let Some(user) = user else {
            return Ok(None);
        };

        Ok(Some(User {
            id: UserId(user.id as usize),
            username: user.username,
            role_id: RoleId(user.role_id as usize),
            password_hash: user.password_hash,
        }))
    }

    pub async fn delete(&self, id: &UserId) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM users WHERE id = $1", id.0 as i64)
            .execute(self.pool)
            .await?;

        Ok(())
    }
}
