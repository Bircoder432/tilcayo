use std::collections::HashMap;

use tilcayo_core::{EntityId, SchemaId, Value, ValueType};
use tilcayo_db::{
    Database,
    repositories::{entity::EntityRepository, schema::SchemaRepository},
};

async fn database() -> Database {
    Database::connect("postgres://tilcayo:tilcayo@localhost/tilcayo")
        .await
        .unwrap()
}

async fn create_schema(db: &Database, name: &str) -> SchemaId {
    let repository = SchemaRepository::new(&db.pool);

    repository
        .create(
            name,
            &ValueType::Object(HashMap::from([
                ("name".into(), ValueType::Text),
                ("age".into(), ValueType::Int),
            ])),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn create_entity() {
    let db = database().await;
    let schema_repository = SchemaRepository::new(&db.pool);
    let entity_repository = EntityRepository::new(&db.pool);

    let schema_name = "entity_test_create";
    let schema_id = create_schema(&db, schema_name).await;

    let data = Value::Object(HashMap::from([
        ("name".into(), Value::Text("Vadim".into())),
        ("age".into(), Value::Int(17)),
    ]));

    let schema = schema_repository
        .find_by_id(&schema_id)
        .await
        .unwrap()
        .unwrap();

    let entity_id = entity_repository
        .create(schema_name, &schema.schema, &data)
        .await
        .unwrap()
        .expect("failed to create entity");

    assert!(entity_id.0 > 0);

    entity_repository
        .delete(schema_name, &entity_id)
        .await
        .unwrap();
    schema_repository.delete(&schema_id).await.unwrap();
}

#[tokio::test]
async fn create_invalid_entity() {
    let db = database().await;
    let schema_repository = SchemaRepository::new(&db.pool);
    let entity_repository = EntityRepository::new(&db.pool);

    let schema_name = "entity_test_invalid";
    let schema_id = create_schema(&db, schema_name).await;

    let data = Value::Object(HashMap::from([
        ("name".into(), Value::Text("Vadim".into())),
        ("age".into(), Value::Text("17".into())),
    ]));

    let schema = schema_repository
        .find_by_id(&schema_id)
        .await
        .unwrap()
        .unwrap();

    let entity_id = entity_repository
        .create(schema_name, &schema.schema, &data)
        .await
        .unwrap();

    assert!(entity_id.is_none());

    schema_repository.delete(&schema_id).await.unwrap();
}

#[tokio::test]
async fn find_entity() {
    let db = database().await;
    let schema_repository = SchemaRepository::new(&db.pool);
    let entity_repository = EntityRepository::new(&db.pool);

    let schema_name = "entity_test_find";
    let schema_id = create_schema(&db, schema_name).await;

    let data = Value::Object(HashMap::from([
        ("name".into(), Value::Text("Vadim".into())),
        ("age".into(), Value::Int(17)),
    ]));

    let schema = schema_repository
        .find_by_id(&schema_id)
        .await
        .unwrap()
        .unwrap();

    let entity_id = entity_repository
        .create(schema_name, &schema.schema, &data)
        .await
        .unwrap()
        .unwrap();

    let entity = entity_repository
        .find_by_id(schema_name, &entity_id)
        .await
        .unwrap()
        .expect("entity not found");

    assert_eq!(entity["id"].as_i64().unwrap() as usize, entity_id.0);
    assert_eq!(entity["name"].as_str().unwrap(), "Vadim");
    assert_eq!(entity["age"].as_i64().unwrap(), 17);

    entity_repository
        .delete(schema_name, &entity_id)
        .await
        .unwrap();
    schema_repository.delete(&schema_id).await.unwrap();
}

#[tokio::test]
async fn find_nonexistent_entity() {
    let db = database().await;
    let schema_repository = SchemaRepository::new(&db.pool);
    let entity_repository = EntityRepository::new(&db.pool);

    let schema_name = "entity_test_find_nonexistent";
    let schema_id = create_schema(&db, schema_name).await;

    let entity = entity_repository
        .find_by_id(schema_name, &EntityId(999999999))
        .await
        .unwrap();

    assert!(entity.is_none());

    schema_repository.delete(&schema_id).await.unwrap();
}

#[tokio::test]
async fn delete_entity() {
    let db = database().await;
    let schema_repository = SchemaRepository::new(&db.pool);
    let entity_repository = EntityRepository::new(&db.pool);

    let schema_name = "entity_test_delete";
    let schema_id = create_schema(&db, schema_name).await;

    let data = Value::Object(HashMap::from([
        ("name".into(), Value::Text("Vadim".into())),
        ("age".into(), Value::Int(17)),
    ]));

    let schema = schema_repository
        .find_by_id(&schema_id)
        .await
        .unwrap()
        .unwrap();
    let entity_id = entity_repository
        .create(schema_name, &schema.schema, &data)
        .await
        .unwrap()
        .unwrap();

    entity_repository
        .delete(schema_name, &entity_id)
        .await
        .unwrap();

    let entity = entity_repository
        .find_by_id(schema_name, &entity_id)
        .await
        .unwrap();

    assert!(entity.is_none());

    schema_repository.delete(&schema_id).await.unwrap();
}
