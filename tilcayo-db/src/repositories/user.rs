use sqlx::PgPool;
use tilcayo_core::{RoleId, User, UserId};

pub struct UserRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> UserRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, role_id: &RoleId) -> Result<UserId, sqlx::Error> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO users (role_id)
            VALUES ($1)
            RETURNING id
            "#,
            role_id.0 as i64
        )
        .fetch_one(self.pool)
        .await?;

        Ok(UserId(id as usize))
    }

    pub async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, sqlx::Error> {
        let user = sqlx::query!(
            r#"
            SELECT role_id
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
            id: id.clone(),
            role_id: RoleId(user.role_id as usize),
        }))
    }

    pub async fn delete(&self, id: &UserId) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM users WHERE id = $1", id.0 as i64)
            .execute(self.pool)
            .await?;

        Ok(())
    }
}
