use std::collections::HashMap;

use tilcayo_auth::password::hash_password;
use tilcayo_core::ValueType;
use tilcayo_db::{
    Database,
    repositories::{role::RoleRepository, schema::SchemaRepository, user::UserRepository},
};

const ADMIN_ROLE_NAME: &str = "admin";
const USERS_SCHEMA_NAME: &str = "users";
const ROLES_SCHEMA_NAME: &str = "roles";
const SCHEMAS_SCHEMA_NAME: &str = "schemas";

pub async fn bootstrap(
    database: &Database,
    admin_username: &str,
    admin_password: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let schemas = SchemaRepository::new(&database.pool);

    ensure_schema(
        &schemas,
        USERS_SCHEMA_NAME,
        ValueType::Object(HashMap::from([
            ("username".into(), ValueType::Text),
            ("role_id".into(), ValueType::Int),
        ])),
    )
    .await?;

    ensure_schema(
        &schemas,
        ROLES_SCHEMA_NAME,
        ValueType::Object(HashMap::from([("name".into(), ValueType::Text)])),
    )
    .await?;

    ensure_schema(
        &schemas,
        SCHEMAS_SCHEMA_NAME,
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

    let users = UserRepository::new(&database.pool);

    if !users.exists().await? {
        let admin_password = admin_password.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "ADMIN_PASSWORD is not set",
            )
        })?;

        let password_hash = hash_password(admin_password)?;

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
) -> Result<tilcayo_core::SchemaId, sqlx::Error> {
    if let Some(schema) = schemas.find_by_name(name).await? {
        return Ok(schema.id);
    }
    schemas.create(name, &schema).await
}
