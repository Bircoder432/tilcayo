use sqlx::PgPool;
use tilcayo_core::{RoleId, User, UserId};

pub struct UserRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> UserRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        role_id: &RoleId,
        password_hash: &str,
    ) -> Result<UserId, sqlx::Error> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO users (role_id, password_hash)
            VALUES ($1, $2)
            RETURNING id
            "#,
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
            SELECT role_id, password_hash
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
