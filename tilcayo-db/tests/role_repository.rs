use std::collections::HashMap;

use tilcayo_core::{Permission, RoleId, SchemaId};
use tilcayo_db::{
    Database,
    repositories::{role::RoleRepository, schema::SchemaRepository},
};

async fn database() -> Database {
    Database::connect("postgres://tilcayo:tilcayo@localhost:5432/tilcayo")
        .await
        .unwrap()
}

async fn create_schema(db: &Database, name: &str) -> SchemaId {
    SchemaRepository::new(&db.pool)
        .create(name, &tilcayo_core::ValueType::Object(HashMap::new()))
        .await
        .unwrap()
}

#[tokio::test]
async fn create_role() {
    let db = database().await;
    let repository = RoleRepository::new(&db.pool);

    let schema_id = create_schema(&db, "role-test-schema-create").await;

    let permissions = HashMap::from([(
        schema_id,
        Permission {
            read: true,
            write: false,
        },
    )]);

    let role_id = repository
        .create("role-test-create", &permissions)
        .await
        .expect("failed to create role");

    assert!(role_id.0 > 0);

    repository.delete(&role_id).await.unwrap();

    SchemaRepository::new(&db.pool)
        .delete(&schema_id)
        .await
        .unwrap();
}

#[tokio::test]
async fn find_role() {
    let db = database().await;
    let repository = RoleRepository::new(&db.pool);

    let schema_id_1 = create_schema(&db, "role-test-schema-find-1").await;

    let schema_id_2 = create_schema(&db, "role-test-schema-find-2").await;

    let permissions = HashMap::from([
        (
            schema_id_1,
            Permission {
                read: true,
                write: false,
            },
        ),
        (
            schema_id_2,
            Permission {
                read: true,
                write: true,
            },
        ),
    ]);

    let role_id = repository
        .create("role-test-find", &permissions)
        .await
        .unwrap();

    let role = repository
        .find_by_id(&role_id)
        .await
        .unwrap()
        .expect("role not found");

    assert_eq!(role.id.0, role_id.0);
    assert_eq!(role.name, "role-test-find");
    assert_eq!(role.permissions.len(), 2);

    assert!(role.permissions[&schema_id_1].read);
    assert!(!role.permissions[&schema_id_1].write);

    assert!(role.permissions[&schema_id_2].read);
    assert!(role.permissions[&schema_id_2].write);

    repository.delete(&role_id).await.unwrap();

    SchemaRepository::new(&db.pool)
        .delete(&schema_id_1)
        .await
        .unwrap();

    SchemaRepository::new(&db.pool)
        .delete(&schema_id_2)
        .await
        .unwrap();
}

#[tokio::test]
async fn find_nonexistent_role() {
    let db = database().await;
    let repository = RoleRepository::new(&db.pool);

    let role = repository.find_by_id(&RoleId(999999999)).await.unwrap();

    assert!(role.is_none());
}

#[tokio::test]
async fn delete_role() {
    let db = database().await;
    let repository = RoleRepository::new(&db.pool);

    let schema_id = create_schema(&db, "role-test-schema-delete").await;

    let permissions = HashMap::from([(
        schema_id,
        Permission {
            read: true,
            write: true,
        },
    )]);

    let role_id = repository
        .create("role-test-delete", &permissions)
        .await
        .unwrap();

    repository.delete(&role_id).await.unwrap();

    let role = repository.find_by_id(&role_id).await.unwrap();

    assert!(role.is_none());

    let permissions_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM permissions WHERE role_id = $1",
        role_id.0 as i64
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert_eq!(permissions_count, Some(0));

    SchemaRepository::new(&db.pool)
        .delete(&schema_id)
        .await
        .unwrap();
}
