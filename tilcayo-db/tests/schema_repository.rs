use std::collections::HashMap;

use tilcayo_core::{SchemaId, ValueType};
use tilcayo_db::{Database, repositories::schema::SchemaRepository};

async fn database() -> Database {
    Database::connect("postgres://tilcayo:tilcayo@localhost:5432/tilcayo")
        .await
        .unwrap()
}

#[tokio::test]
async fn create_schema() {
    let db = database().await;
    let repository = SchemaRepository::new(&db.pool);

    let schema = ValueType::Object(HashMap::from([
        ("name".into(), ValueType::Text),
        ("age".into(), ValueType::Int),
    ]));

    let id = repository
        .create("schema_test_create", &schema)
        .await
        .expect("failed to create schema");

    assert!(id.0 > 0);

    repository.delete(&id).await.unwrap();
}

#[tokio::test]
async fn find_schema() {
    let db = database().await;
    let repository = SchemaRepository::new(&db.pool);

    let schema = ValueType::Object(HashMap::from([
        ("name".into(), ValueType::Text),
        ("age".into(), ValueType::Int),
    ]));

    let id = repository
        .create("schema_test_find", &schema)
        .await
        .unwrap();

    let found = repository
        .find_by_id(&id)
        .await
        .unwrap()
        .expect("schema not found");

    assert_eq!(found.id.0, id.0);
    assert_eq!(found.name, "schema_test_find");

    let ValueType::Object(fields) = found.schema else {
        panic!("expected object schema");
    };

    assert!(matches!(fields.get("name"), Some(ValueType::Text)));
    assert!(matches!(fields.get("age"), Some(ValueType::Int)));

    repository.delete(&id).await.unwrap();
}

#[tokio::test]
async fn find_nonexistent_schema() {
    let db = database().await;
    let repository = SchemaRepository::new(&db.pool);

    let schema = repository.find_by_id(&SchemaId(999999999)).await.unwrap();

    assert!(schema.is_none());
}

#[tokio::test]
async fn delete_schema() {
    let db = database().await;
    let repository = SchemaRepository::new(&db.pool);

    let schema = ValueType::Text;

    let id = repository
        .create("schema_test_delete", &schema)
        .await
        .unwrap();

    repository.delete(&id).await.unwrap();

    let schema = repository.find_by_id(&id).await.unwrap();

    assert!(schema.is_none());
}
