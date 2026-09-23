use sqlx::PgPool;
use tilcayo_core::{Schema, SchemaId, ValueType};

pub struct SchemaRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> SchemaRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, name: &str, schema: &ValueType) -> Result<SchemaId, sqlx::Error> {
        let schema =
            serde_json::to_value(schema).map_err(|error| sqlx::Error::Decode(Box::new(error)))?;

        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO schemas (name, schema)
            VALUES ($1, $2)
            RETURNING id
            "#,
            name,
            schema,
        )
        .fetch_one(self.pool)
        .await?;

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

    pub async fn delete(&self, id: &SchemaId) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM schemas WHERE id = $1", id.0 as i64)
            .execute(self.pool)
            .await?;

        Ok(())
    }
}
