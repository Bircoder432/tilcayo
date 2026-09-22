use tilcayo_core::RoleId;
use tilcayo_db::{
    Database,
    repositories::{role::RoleRepository, user::UserRepository},
};

async fn database() -> Database {
    Database::connect("postgres://tilcayo:tilcayo@localhost/tilcayo")
        .await
        .unwrap()
}

async fn create_role(db: &Database) -> RoleId {
    RoleRepository::new(&db.pool)
        .create(&Default::default())
        .await
        .unwrap()
}

#[tokio::test]
async fn create_user() {
    let db = database().await;

    let role_id = create_role(&db).await;
    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$test$hash";

    let repository = UserRepository::new(&db.pool);

    let user_id = repository
        .create(&role_id, password_hash)
        .await
        .expect("failed to create user");

    assert!(user_id.0 > 0);

    repository.delete(&user_id).await.unwrap();
    RoleRepository::new(&db.pool)
        .delete(&role_id)
        .await
        .unwrap();
}

#[tokio::test]
async fn find_user() {
    let db = database().await;

    let role_id = create_role(&db).await;
    let repository = UserRepository::new(&db.pool);
    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$test$hash";
    let user_id = repository.create(&role_id, password_hash).await.unwrap();

    let user = repository
        .find_by_id(&user_id)
        .await
        .unwrap()
        .expect("user not found");

    assert_eq!(user.id.0, user_id.0);
    assert_eq!(user.role_id.0, role_id.0);
    assert_eq!(user.password_hash, password_hash);
    repository.delete(&user_id).await.unwrap();
    RoleRepository::new(&db.pool)
        .delete(&role_id)
        .await
        .unwrap();
}

#[tokio::test]
async fn find_nonexistent_user() {
    let db = database().await;
    let repository = UserRepository::new(&db.pool);

    let user = repository
        .find_by_id(&tilcayo_core::UserId(999999999))
        .await
        .unwrap();

    assert!(user.is_none());
}

#[tokio::test]
async fn delete_user() {
    let db = database().await;

    let role_id = create_role(&db).await;
    let repository = UserRepository::new(&db.pool);
    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$test$hash";
    let user_id = repository.create(&role_id, password_hash).await.unwrap();

    repository.delete(&user_id).await.unwrap();

    let user = repository.find_by_id(&user_id).await.unwrap();

    assert!(user.is_none());

    RoleRepository::new(&db.pool)
        .delete(&role_id)
        .await
        .unwrap();
}
