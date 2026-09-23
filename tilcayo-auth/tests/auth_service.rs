use tilcayo_auth::password::hash_password;
use tilcayo_auth::service::AuthService;
use tilcayo_core::RoleId;
use tilcayo_db::repositories::{role::RoleRepository, user::UserRepository};

async fn setup() -> (tilcayo_db::Database, RoleId) {
    let database = tilcayo_db::Database::connect("postgres://tilcayo:tilcayo@localhost/tilcayo")
        .await
        .expect("failed to connect to database");

    let roles = RoleRepository::new(&database.pool);

    let permissions = std::collections::HashMap::new();

    let role_id = roles
        .create(&permissions)
        .await
        .expect("failed to create role");

    (database, role_id)
}

#[tokio::test]
async fn authenticate_with_correct_password() {
    let (database, role_id) = setup().await;

    let username = "auth-test-correct-password";
    let password = "correct password";

    let password_hash = hash_password(password).expect("failed to hash password");

    let users = UserRepository::new(&database.pool);

    let user_id = users
        .create(&username, &role_id, &password_hash)
        .await
        .expect("failed to create user");

    let auth = AuthService::new(users);

    let user = auth
        .authenticate(username, password)
        .await
        .expect("authentication failed")
        .expect("user was not authenticated");

    assert_eq!(user.id.0, user_id.0);
    assert_eq!(user.username, username);

    users.delete(&user_id).await.expect("failed to delete user");

    database.pool.close().await;
}

#[tokio::test]
async fn authenticate_with_wrong_password() {
    let (database, role_id) = setup().await;

    let username = "auth-test-wrong-password";

    let password_hash = hash_password("correct password").expect("failed to hash password");

    let users = UserRepository::new(&database.pool);

    let user_id = users
        .create(&username, &role_id, &password_hash)
        .await
        .expect("failed to create user");

    let auth = AuthService::new(users);

    let user = auth
        .authenticate(username, "wrong password")
        .await
        .expect("authentication failed");

    assert!(user.is_none());

    users.delete(&user_id).await.expect("failed to delete user");

    database.pool.close().await;
}

#[tokio::test]
async fn authenticate_unknown_username() {
    let (database, role_id) = setup().await;

    let username = "auth-test-unknown-username";

    let password_hash = hash_password("password").expect("failed to hash password");

    let users = UserRepository::new(&database.pool);

    let user_id = users
        .create(&username, &role_id, &password_hash)
        .await
        .expect("failed to create user");

    let auth = AuthService::new(users);

    let user = auth
        .authenticate("unknown-user", "password")
        .await
        .expect("authentication failed");

    assert!(user.is_none());

    users.delete(&user_id).await.expect("failed to delete user");

    database.pool.close().await;
}
