use std::collections::HashMap;

use sqlx::PgPool;
use tilcayo_core::{Permission, Role, RoleId, SchemaId};

pub struct RoleRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> RoleRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        permissions: &HashMap<SchemaId, Permission>,
    ) -> Result<RoleId, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;

        let role_id = sqlx::query_scalar!("INSERT INTO roles DEFAULT VALUES RETURNING id")
            .fetch_one(&mut *transaction)
            .await?;

        for (schema_id, permission) in permissions {
            sqlx::query!(
                "
                INSERT INTO permissions (role_id, schema_id, read, write)
                VALUES ($1, $2, $3, $4)
                ",
                role_id,
                schema_id.0 as i64,
                permission.read,
                permission.write,
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(RoleId(role_id as usize))
    }

    pub async fn find_by_id(&self, id: &RoleId) -> Result<Option<Role>, sqlx::Error> {
        let exists = sqlx::query_scalar!("SELECT id FROM roles WHERE id = $1", id.0 as i64)
            .fetch_optional(self.pool)
            .await?;

        let Some(role_id) = exists else {
            return Ok(None);
        };

        let permissions = sqlx::query!(
            "
            SELECT schema_id, read, write
            FROM permissions
            WHERE role_id = $1
            ",
            role_id
        )
        .fetch_all(self.pool)
        .await?
        .into_iter()
        .map(|permission| {
            (
                SchemaId(permission.schema_id as usize),
                Permission {
                    read: permission.read,
                    write: permission.write,
                },
            )
        })
        .collect();

        Ok(Some(Role {
            id: RoleId(role_id as usize),
            permissions,
        }))
    }

    pub async fn delete(&self, id: &RoleId) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM roles WHERE id = $1", id.0 as i64)
            .execute(self.pool)
            .await?;

        Ok(())
    }
}
