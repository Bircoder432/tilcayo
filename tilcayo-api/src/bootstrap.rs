use std::collections::HashMap;

use tilcayo_auth::password::hash_password;
use tilcayo_core::{Permission, SchemaId, ValueType};
use tilcayo_db::{
    Database,
    repositories::{role::RoleRepository, schema::SchemaRepository, user::UserRepository},
};

const ADMIN_ROLE_NAME: &str = "admin";

const USER_SCHEMA_NAME: &str = "user";
const ROLE_SCHEMA_NAME: &str = "role";
const SCHEMA_SCHEMA_NAME: &str = "schema";

pub async fn bootstrap(
    database: &Database,
    admin_username: &str,
    admin_password: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let schemas = SchemaRepository::new(&database.pool);

    ensure_schema(
        &schemas,
        USER_SCHEMA_NAME,
        ValueType::Object(HashMap::from([
            ("username".into(), ValueType::Text),
            ("role_id".into(), ValueType::Int),
        ])),
    )
    .await?;

    ensure_schema(
        &schemas,
        ROLE_SCHEMA_NAME,
        ValueType::Object(HashMap::from([
            ("name".into(), ValueType::Text),
            (
                "permissions".into(),
                ValueType::List(Box::new(ValueType::Object(HashMap::from([
                    ("schema_id".into(), ValueType::Int),
                    ("read".into(), ValueType::Bool),
                    ("write".into(), ValueType::Bool),
                ])))),
            ),
        ])),
    )
    .await?;

    ensure_schema(
        &schemas,
        SCHEMA_SCHEMA_NAME,
        ValueType::Object(HashMap::from([
            ("name".into(), ValueType::Text),
            ("schema".into(), ValueType::Any),
        ])),
    )
    .await?;

    let schema_ids = schemas.find_all_ids().await?;

    let roles = RoleRepository::new(&database.pool);

    let admin_role_id = match roles.find_by_name(ADMIN_ROLE_NAME).await? {
        Some(role) => role.id,
        None => roles.create(ADMIN_ROLE_NAME, &HashMap::new()).await?,
    };

    for schema_id in schema_ids {
        roles
            .set_permission(&admin_role_id, &schema_id, true, true)
            .await?;
    }

    let has_users = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM users
        )
        "#
    )
    .fetch_one(&database.pool)
    .await?
    .unwrap_or(false);

    if !has_users {
        let password_hash = hash_password(admin_password)?;

        let users = UserRepository::new(&database.pool);

        users
            .create(admin_username, &admin_role_id, &password_hash)
            .await?;
    }

    Ok(())
}

async fn ensure_schema(
    schemas: &SchemaRepository<'_>,
    name: &str,
    schema: ValueType,
) -> Result<SchemaId, sqlx::Error> {
    if let Some(schema) = schemas.find_by_name(name).await? {
        return Ok(schema.id);
    }

    schemas.create(name, &schema).await
}
