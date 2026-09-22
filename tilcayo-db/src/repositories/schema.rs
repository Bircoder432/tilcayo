use sqlx::PgPool;
use tilcayo_core::{Schema, SchemaId, ValueType};

pub struct SchemaRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> SchemaRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, schema: &ValueType) -> Result<SchemaId, sqlx::Error> {
        let schema =
            serde_json::to_value(schema).map_err(|error| sqlx::Error::Decode(Box::new(error)))?;

        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO schemas (schema)
            VALUES ($1)
            RETURNING id
            "#,
            schema
        )
        .fetch_one(self.pool)
        .await?;

        Ok(SchemaId(id as usize))
    }

    pub async fn find_by_id(&self, id: &SchemaId) -> Result<Option<Schema>, sqlx::Error> {
        let schema = sqlx::query_scalar!(
            r#"
            SELECT schema
            FROM schemas
            WHERE id = $1
            "#,
            id.0 as i64
        )
        .fetch_optional(self.pool)
        .await?;

        let Some(schema) = schema else {
            return Ok(None);
        };

        let schema =
            serde_json::from_value(schema).map_err(|error| sqlx::Error::Decode(Box::new(error)))?;

        Ok(Some(Schema {
            id: id.clone(),
            schema,
        }))
    }

    pub async fn delete(&self, id: &SchemaId) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM schemas WHERE id = $1", id.0 as i64)
            .execute(self.pool)
            .await?;

        Ok(())
    }
}
