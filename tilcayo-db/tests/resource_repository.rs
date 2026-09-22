use std::collections::HashMap;

use tilcayo_core::{ResourceId, SchemaId, Value, ValueType};
use tilcayo_db::{
    Database,
    repositories::{resource::ResourceRepository, schema::SchemaRepository},
};

async fn database() -> Database {
    Database::connect("postgres://tilcayo:tilcayo@localhost/tilcayo")
        .await
        .unwrap()
}

async fn create_schema(db: &Database) -> SchemaId {
    let repository = SchemaRepository::new(&db.pool);

    repository
        .create(&ValueType::Object(HashMap::from([
            ("name".into(), ValueType::Text),
            ("age".into(), ValueType::Int),
        ])))
        .await
        .unwrap()
}

#[tokio::test]
async fn create_resource() {
    let db = database().await;
    let repository = ResourceRepository::new(&db.pool);

    let schema_id = create_schema(&db).await;

    let data = Value::Object(HashMap::from([
        ("name".into(), Value::Text("Vadim".into())),
        ("age".into(), Value::Int(17)),
    ]));

    let resource_id = repository
        .create(&schema_id, &data)
        .await
        .unwrap()
        .expect("failed to create resource");

    assert!(resource_id.0 > 0);

    repository.delete(&resource_id).await.unwrap();

    SchemaRepository::new(&db.pool)
        .delete(&schema_id)
        .await
        .unwrap();
}

#[tokio::test]
async fn create_invalid_resource() {
    let db = database().await;
    let repository = ResourceRepository::new(&db.pool);

    let schema_id = create_schema(&db).await;

    let data = Value::Object(HashMap::from([
        ("name".into(), Value::Text("Vadim".into())),
        ("age".into(), Value::Text("17".into())),
    ]));

    let resource_id = repository.create(&schema_id, &data).await.unwrap();

    assert!(resource_id.is_none());

    SchemaRepository::new(&db.pool)
        .delete(&schema_id)
        .await
        .unwrap();
}

#[tokio::test]
async fn find_resource() {
    let db = database().await;
    let repository = ResourceRepository::new(&db.pool);

    let schema_id = create_schema(&db).await;

    let data = Value::Object(HashMap::from([
        ("name".into(), Value::Text("Vadim".into())),
        ("age".into(), Value::Int(17)),
    ]));

    let resource_id = repository.create(&schema_id, &data).await.unwrap().unwrap();

    let resource = repository
        .find_by_id(&resource_id)
        .await
        .unwrap()
        .expect("resource not found");

    assert_eq!(resource.id.0, resource_id.0);
    assert_eq!(resource.schema_id.0, schema_id.0);

    assert!(matches!(resource.data, Value::Object(_)));

    repository.delete(&resource_id).await.unwrap();

    SchemaRepository::new(&db.pool)
        .delete(&schema_id)
        .await
        .unwrap();
}

#[tokio::test]
async fn find_nonexistent_resource() {
    let db = database().await;
    let repository = ResourceRepository::new(&db.pool);

    let resource = repository.find_by_id(&ResourceId(999999999)).await.unwrap();

    assert!(resource.is_none());
}

#[tokio::test]
async fn delete_resource() {
    let db = database().await;
    let repository = ResourceRepository::new(&db.pool);

    let schema_id = create_schema(&db).await;

    let data = Value::Object(HashMap::from([
        ("name".into(), Value::Text("Vadim".into())),
        ("age".into(), Value::Int(17)),
    ]));

    let resource_id = repository.create(&schema_id, &data).await.unwrap().unwrap();

    repository.delete(&resource_id).await.unwrap();

    let resource = repository.find_by_id(&resource_id).await.unwrap();

    assert!(resource.is_none());

    SchemaRepository::new(&db.pool)
        .delete(&schema_id)
        .await
        .unwrap();
}
