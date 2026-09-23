use sqlx::{AssertSqlSafe, PgPool, QueryBuilder};
use tilcayo_core::{EntityId, Value, ValueType};

pub struct EntityRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> EntityRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        schema_name: &str,
        schema: &ValueType,
        data: &Value,
    ) -> Result<Option<EntityId>, sqlx::Error> {
        if let ValueType::Object(fields) = schema {
            if let Value::Object(data_fields) = data {
                let mut col_defs = Vec::new();
                let mut col_names = Vec::new();

                for (field_name, field_type) in fields {
                    if let Some(val) = data_fields.get(field_name) {
                        if !field_type.matches(val) {
                            return Ok(None);
                        }
                        let safe_name = sanitize_ident(field_name)?;
                        col_names.push(safe_name.clone());
                        let sql_type = match field_type {
                            ValueType::Int => "BIGINT",
                            ValueType::Float => "DOUBLE PRECISION",
                            ValueType::Bool => "BOOLEAN",
                            ValueType::Text => "TEXT",
                            _ => "JSONB",
                        };
                        col_defs.push(format!("{} {}", safe_name, sql_type));
                    }
                }

                if col_names.is_empty() {
                    return Ok(None);
                }

                let safe_table = sanitize_ident(schema_name)?;
                let query_str = format!(
                    "INSERT INTO {} ({}) SELECT {} FROM jsonb_to_record($1::jsonb) AS x({}) RETURNING id",
                    safe_table,
                    col_names.join(", "),
                    col_names.join(", "),
                    col_defs.join(", ")
                );

                let json_data = serde_json::to_value(data_fields)
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
                let id: i64 = sqlx::query_scalar(AssertSqlSafe(query_str.as_str()))
                    .bind(json_data)
                    .fetch_one(self.pool)
                    .await?;

                return Ok(Some(EntityId(id as usize)));
            }
        }
        Ok(None)
    }

    pub async fn find_by_id(
        &self,
        schema_name: &str,
        id: &EntityId,
    ) -> Result<Option<serde_json::Value>, sqlx::Error> {
        let safe_table = sanitize_ident(schema_name)?;
        let query_str = format!(
            "SELECT row_to_json(t) FROM {} AS t WHERE t.id = $1",
            safe_table
        );

        let row: Option<serde_json::Value> = sqlx::query_scalar(AssertSqlSafe(query_str.as_str()))
            .bind(id.0 as i64)
            .fetch_optional(self.pool)
            .await?;

        Ok(row)
    }

    pub async fn find_all(
        &self,
        schema_name: &str,
        schema: &ValueType,
        filters: &[(String, String)],
        page: usize,
        limit: usize,
    ) -> Result<(Vec<serde_json::Value>, i64), sqlx::Error> {
        let safe_table = sanitize_ident(schema_name)?;

        let mut count_query =
            QueryBuilder::new(format!("SELECT COUNT(*) FROM {} AS t", safe_table));
        let mut data_query =
            QueryBuilder::new(format!("SELECT row_to_json(t) FROM {} AS t", safe_table));

        if !filters.is_empty() {
            let valid_filters: Vec<_> = filters
                .iter()
                .filter(|(field, _)| {
                    if let ValueType::Object(fields) = schema {
                        fields.contains_key(field)
                    } else {
                        false
                    }
                })
                .collect();

            if !valid_filters.is_empty() {
                count_query.push(" WHERE ");
                data_query.push(" WHERE ");

                let mut is_first = true;
                for (field, value) in valid_filters {
                    if !is_first {
                        count_query.push(" AND ");
                        data_query.push(" AND ");
                    }
                    is_first = false;

                    let safe_col = sanitize_ident(field)?;

                    count_query.push(format!("t.{} = ", safe_col));
                    count_query.push_bind(value.clone());

                    data_query.push(format!("t.{} = ", safe_col));
                    data_query.push_bind(value.clone());
                }
            }
        }

        let total: i64 = count_query
            .build_query_scalar()
            .fetch_one(self.pool)
            .await?;

        let offset = (page - 1) * limit;
        data_query.push(" ORDER BY t.id ");
        data_query.push(" LIMIT ");
        data_query.push_bind(limit as i64);
        data_query.push(" OFFSET ");
        data_query.push_bind(offset as i64);

        let rows: Vec<serde_json::Value> =
            data_query.build_query_scalar().fetch_all(self.pool).await?;

        Ok((rows, total))
    }

    pub async fn update(
        &self,
        schema_name: &str,
        schema: &ValueType,
        id: &EntityId,
        data: &Value,
    ) -> Result<Option<()>, sqlx::Error> {
        if let ValueType::Object(fields) = schema {
            if let Value::Object(data_fields) = data {
                let mut col_defs = Vec::new();
                let mut set_clauses = Vec::new();

                for (field_name, field_type) in fields {
                    if let Some(val) = data_fields.get(field_name) {
                        if !field_type.matches(val) {
                            return Ok(None);
                        }
                        let safe_name = sanitize_ident(field_name)?;
                        col_defs.push(format!(
                            "{} {}",
                            safe_name.clone(),
                            match field_type {
                                ValueType::Int => "BIGINT",
                                ValueType::Float => "DOUBLE PRECISION",
                                ValueType::Bool => "BOOLEAN",
                                ValueType::Text => "TEXT",
                                _ => "JSONB",
                            }
                        ));
                        set_clauses.push(format!("{} = data.{}", safe_name, safe_name));
                    }
                }

                if set_clauses.is_empty() {
                    return Ok(None);
                }

                let safe_table = sanitize_ident(schema_name)?;
                let query_str = format!(
                    "UPDATE {} AS t SET {} FROM jsonb_to_record($1::jsonb) AS data({}) WHERE t.id = $2",
                    safe_table,
                    set_clauses.join(", "),
                    col_defs.join(", ")
                );

                let json_data = serde_json::to_value(data_fields)
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

                let result = sqlx::query(AssertSqlSafe(query_str.as_str()))
                    .bind(json_data)
                    .bind(id.0 as i64)
                    .execute(self.pool)
                    .await?;

                if result.rows_affected() == 0 {
                    return Ok(None);
                }

                return Ok(Some(()));
            }
        }
        Ok(None)
    }

    pub async fn delete(&self, schema_name: &str, id: &EntityId) -> Result<(), sqlx::Error> {
        let safe_table = sanitize_ident(schema_name)?;
        let query_str = format!("DELETE FROM {} WHERE id = $1", safe_table);

        sqlx::query(AssertSqlSafe(query_str.as_str()))
            .bind(id.0 as i64)
            .execute(self.pool)
            .await?;

        Ok(())
    }
}

fn sanitize_ident(name: &str) -> Result<String, sqlx::Error> {
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(sqlx::Error::Protocol("Invalid identifier name".into()));
    }
    Ok(format!("\"{}\"", name))
}
