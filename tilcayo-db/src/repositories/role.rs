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
        name: &str,
        permissions: &HashMap<SchemaId, Permission>,
    ) -> Result<RoleId, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;

        let role_id = sqlx::query_scalar!(
            r#"
            INSERT INTO roles (name)
            VALUES ($1)
            RETURNING id
            "#,
            name,
        )
        .fetch_one(&mut *transaction)
        .await?;

        for (schema_id, permission) in permissions {
            sqlx::query!(
                r#"
                INSERT INTO permissions (
                    role_id,
                    schema_id,
                    read,
                    write
                )
                VALUES ($1, $2, $3, $4)
                "#,
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
        let role = sqlx::query!(
            r#"
            SELECT id, name
            FROM roles
            WHERE id = $1
            "#,
            id.0 as i64,
        )
        .fetch_optional(self.pool)
        .await?;

        let Some(role) = role else {
            return Ok(None);
        };

        let permissions = sqlx::query!(
            r#"
            SELECT schema_id, read, write
            FROM permissions
            WHERE role_id = $1
            "#,
            role.id,
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
            id: RoleId(role.id as usize),
            name: role.name,
            permissions,
        }))
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<Role>, sqlx::Error> {
        let role = sqlx::query!(
            r#"
            SELECT id, name
            FROM roles
            WHERE name = $1
            "#,
            name,
        )
        .fetch_optional(self.pool)
        .await?;

        let Some(role) = role else {
            return Ok(None);
        };

        let permissions = sqlx::query!(
            r#"
            SELECT schema_id, read, write
            FROM permissions
            WHERE role_id = $1
            "#,
            role.id,
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
            id: RoleId(role.id as usize),
            name: role.name,
            permissions,
        }))
    }

    pub async fn set_permission(
        &self,
        role_id: &RoleId,
        schema_id: &SchemaId,
        read: bool,
        write: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO permissions (
                role_id,
                schema_id,
                read,
                write
            )
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (role_id, schema_id)
            DO UPDATE SET
                read = EXCLUDED.read,
                write = EXCLUDED.write
            "#,
            role_id.0 as i64,
            schema_id.0 as i64,
            read,
            write,
        )
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: &RoleId) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM roles WHERE id = $1", id.0 as i64)
            .execute(self.pool)
            .await?;

        Ok(())
    }
    pub async fn find_all(&self) -> Result<Vec<Role>, sqlx::Error> {
        let roles = sqlx::query!(
            r#"
            SELECT id, name
            FROM roles
            ORDER BY id
            "#
        )
        .fetch_all(self.pool)
        .await?;

        let mut result = Vec::new();
        for r in roles {
            let permissions = sqlx::query!(
                r#"
                SELECT schema_id, read, write
                FROM permissions
                WHERE role_id = $1
                "#,
                r.id,
            )
            .fetch_all(self.pool)
            .await?
            .into_iter()
            .map(|p| {
                (
                    SchemaId(p.schema_id as usize),
                    Permission {
                        read: p.read,
                        write: p.write,
                    },
                )
            })
            .collect();

            result.push(Role {
                id: RoleId(r.id as usize),
                name: r.name,
                permissions,
            });
        }
        Ok(result)
    }
}
