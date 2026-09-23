use std::collections::HashMap;

use tilcayo_auth::{password::hash_password, service::AuthService};
use tilcayo_core::UserId;
use tilcayo_db::{
    Database,
    repositories::{role::RoleRepository, user::UserRepository},
};

const DATABASE_URL: &str = "postgres://tilcayo:tilcayo@localhost/tilcayo";

#[tokio::test]
async fn authenticate_with_correct_password() {
    let database = Database::connect(DATABASE_URL)
        .await
        .expect("failed to connect to database");

    let role_repository = RoleRepository::new(&database.pool);

    let role_id = role_repository
        .create(&HashMap::new())
        .await
        .expect("failed to create role");

    let password = "correct password";

    let password_hash = hash_password(password).expect("failed to hash password");

    let user_repository = UserRepository::new(&database.pool);

    let user_id = user_repository
        .create(&role_id, &password_hash)
        .await
        .expect("failed to create user");

    let auth = AuthService::new(user_repository);

    let user = auth
        .authenticate(&user_id, password)
        .await
        .expect("authentication failed")
        .expect("user was not authenticated");

    assert_eq!(user.id.0, user_id.0);
    assert_eq!(user.role_id.0, role_id.0);

    user_repository
        .delete(&user_id)
        .await
        .expect("failed to delete user");

    role_repository
        .delete(&role_id)
        .await
        .expect("failed to delete role");
}

#[tokio::test]
async fn authenticate_with_wrong_password() {
    let database = Database::connect(DATABASE_URL)
        .await
        .expect("failed to connect to database");

    let role_repository = RoleRepository::new(&database.pool);

    let role_id = role_repository
        .create(&HashMap::new())
        .await
        .expect("failed to create role");

    let password_hash = hash_password("correct password").expect("failed to hash password");

    let user_repository = UserRepository::new(&database.pool);

    let user_id = user_repository
        .create(&role_id, &password_hash)
        .await
        .expect("failed to create user");

    let auth = AuthService::new(user_repository);

    let user = auth
        .authenticate(&user_id, "wrong password")
        .await
        .expect("authentication failed");

    assert!(user.is_none());

    user_repository
        .delete(&user_id)
        .await
        .expect("failed to delete user");

    role_repository
        .delete(&role_id)
        .await
        .expect("failed to delete role");
}

#[tokio::test]
async fn authenticate_nonexistent_user() {
    let database = Database::connect(DATABASE_URL)
        .await
        .expect("failed to connect to database");

    let user_repository = UserRepository::new(&database.pool);
    let auth = AuthService::new(user_repository);

    let user = auth
        .authenticate(&UserId(999_999_999), "password")
        .await
        .expect("authentication failed");

    assert!(user.is_none());
}
