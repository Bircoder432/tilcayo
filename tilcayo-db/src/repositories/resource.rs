use sqlx::PgPool;
use tilcayo_core::{Resource, ResourceId, SchemaId, Value, ValueType};

pub struct ResourceRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> ResourceRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        schema_id: &SchemaId,
        data: &Value,
    ) -> Result<Option<ResourceId>, sqlx::Error> {
        let schema = sqlx::query_scalar!(
            r#"
            SELECT schema
            FROM schemas
            WHERE id = $1
            "#,
            schema_id.0 as i64
        )
        .fetch_optional(self.pool)
        .await?;

        let Some(schema) = schema else {
            return Ok(None);
        };

        let schema: ValueType =
            serde_json::from_value(schema).map_err(|error| sqlx::Error::Decode(Box::new(error)))?;

        if !schema.matches(data) {
            return Ok(None);
        }

        let data =
            serde_json::to_value(data).map_err(|error| sqlx::Error::Decode(Box::new(error)))?;

        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO resources (schema_id, data)
            VALUES ($1, $2)
            RETURNING id
            "#,
            schema_id.0 as i64,
            data
        )
        .fetch_one(self.pool)
        .await?;

        Ok(Some(ResourceId(id as usize)))
    }

    pub async fn find_by_id(&self, id: &ResourceId) -> Result<Option<Resource>, sqlx::Error> {
        let resource = sqlx::query!(
            r#"
            SELECT schema_id, data
            FROM resources
            WHERE id = $1
            "#,
            id.0 as i64
        )
        .fetch_optional(self.pool)
        .await?;

        let Some(resource) = resource else {
            return Ok(None);
        };

        let data = serde_json::from_value(resource.data)
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;

        Ok(Some(Resource {
            id: id.clone(),
            schema_id: SchemaId(resource.schema_id as usize),
            data,
        }))
    }

    pub async fn delete(&self, id: &ResourceId) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM resources WHERE id = $1", id.0 as i64)
            .execute(self.pool)
            .await?;

        Ok(())
    }
}
