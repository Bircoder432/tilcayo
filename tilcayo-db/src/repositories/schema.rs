use sqlx::{AssertSqlSafe, PgPool};
use tilcayo_core::{Schema, SchemaId, ValueType};

pub struct SchemaRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> SchemaRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, name: &str, schema: &ValueType) -> Result<SchemaId, sqlx::Error> {
        let schema_json =
            serde_json::to_value(schema).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO schemas (name, schema)
            VALUES ($1, $2)
            RETURNING id
            "#,
            name,
            schema_json,
        )
        .fetch_one(self.pool)
        .await?;

        if let ValueType::Object(fields) = schema {
            let mut cols = vec!["id BIGSERIAL PRIMARY KEY".to_string()];
            for (field_name, field_type) in fields {
                let safe_name = sanitize_ident(field_name)?;
                let sql_type = match field_type {
                    ValueType::Int => "BIGINT",
                    ValueType::Float => "DOUBLE PRECISION",
                    ValueType::Bool => "BOOLEAN",
                    ValueType::Text => "TEXT",
                    _ => "JSONB",
                };
                cols.push(format!("{} {}", safe_name, sql_type));
            }
            let safe_table = sanitize_ident(name)?;
            let create_table_query = format!(
                "CREATE TABLE IF NOT EXISTS {} ({})",
                safe_table,
                cols.join(", ")
            );
            sqlx::query(AssertSqlSafe(create_table_query.as_str()))
                .execute(self.pool)
                .await?;
        }

        Ok(SchemaId(id as usize))
    }

    pub async fn find_by_id(&self, id: &SchemaId) -> Result<Option<Schema>, sqlx::Error> {
        let schema = sqlx::query!(
            r#"
            SELECT id, name, schema
            FROM schemas
            WHERE id = $1
            "#,
            id.0 as i64,
        )
        .fetch_optional(self.pool)
        .await?;

        let Some(schema) = schema else {
            return Ok(None);
        };

        let schema_value = serde_json::from_value(schema.schema)
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;

        Ok(Some(Schema {
            id: SchemaId(schema.id as usize),
            name: schema.name,
            schema: schema_value,
        }))
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<Schema>, sqlx::Error> {
        let schema = sqlx::query!(
            r#"
            SELECT id, name, schema
            FROM schemas
            WHERE name = $1
            "#,
            name,
        )
        .fetch_optional(self.pool)
        .await?;

        let Some(schema) = schema else {
            return Ok(None);
        };

        let schema_value = serde_json::from_value(schema.schema)
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;

        Ok(Some(Schema {
            id: SchemaId(schema.id as usize),
            name: schema.name,
            schema: schema_value,
        }))
    }

    pub async fn find_all(&self) -> Result<Vec<Schema>, sqlx::Error> {
        let schemas = sqlx::query!(
            r#"
            SELECT id, name, schema
            FROM schemas
            ORDER BY id
            "#
        )
        .fetch_all(self.pool)
        .await?;

        schemas
            .into_iter()
            .map(|schema| {
                let schema_value = serde_json::from_value(schema.schema)
                    .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;

                Ok(Schema {
                    id: SchemaId(schema.id as usize),
                    name: schema.name,
                    schema: schema_value,
                })
            })
            .collect()
    }

    pub async fn find_all_ids(&self) -> Result<Vec<SchemaId>, sqlx::Error> {
        let schemas = sqlx::query!(
            r#"
            SELECT id
            FROM schemas
            ORDER BY id
            "#
        )
        .fetch_all(self.pool)
        .await?;

        Ok(schemas
            .into_iter()
            .map(|schema| SchemaId(schema.id as usize))
            .collect())
    }

    pub async fn update(
        &self,
        id: &SchemaId,
        name: &str,
        schema: &ValueType,
    ) -> Result<bool, sqlx::Error> {
        let schema_json =
            serde_json::to_value(schema).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let old_schema = self.find_by_id(id).await?;

        let result = sqlx::query!(
            r#"
            UPDATE schemas
            SET name = $1, schema = $2
            WHERE id = $3
            "#,
            name,
            schema_json,
            id.0 as i64,
        )
        .execute(self.pool)
        .await?;

        if result.rows_affected() > 0 {
            if let Some(old) = old_schema {
                if old.name != name {
                    let old_safe = sanitize_ident(&old.name)?;
                    let new_safe = sanitize_ident(name)?;
                    let rename_query = format!("ALTER TABLE {} RENAME TO {}", old_safe, new_safe);
                    sqlx::query(AssertSqlSafe(rename_query.as_str()))
                        .execute(self.pool)
                        .await?;
                }
            }
        }

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete(&self, id: &SchemaId) -> Result<bool, sqlx::Error> {
        let schema = self.find_by_id(id).await?;

        let Some(schema) = schema else {
            return Ok(false);
        };

        let mut transaction = self.pool.begin().await?;

        let safe_table = sanitize_ident(&schema.name)?;
        let drop_query = format!("DROP TABLE IF EXISTS {}", safe_table);
        sqlx::query(AssertSqlSafe(drop_query.as_str()))
            .execute(&mut *transaction)
            .await?;

        sqlx::query!(
            r#"
            DELETE FROM permissions
            WHERE schema_id = $1
            "#,
            id.0 as i64,
        )
        .execute(&mut *transaction)
        .await?;

        let result = sqlx::query!(
            r#"
            DELETE FROM schemas
            WHERE id = $1
            "#,
            id.0 as i64,
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(result.rows_affected() > 0)
    }
}

fn sanitize_ident(name: &str) -> Result<String, sqlx::Error> {
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(sqlx::Error::Protocol("Invalid identifier name".into()));
    }
    Ok(format!("\"{}\"", name))
}
