use std::time::Duration;

use tilcayo_core::UserId;
use tilcayo_db::sessions::SessionStore;

#[tokio::test]
async fn session_lifecycle() {
    let store = SessionStore::new("redis://localhost").expect("failed to create session store");

    let session_id = "test-session";
    let user_id = UserId(42);
    let refresh_token_hash = "test-refresh-token-hash";

    store
        .create(
            session_id,
            &user_id,
            refresh_token_hash,
            Duration::from_secs(60),
        )
        .await
        .expect("failed to create session");

    let session = store
        .find(session_id)
        .await
        .expect("failed to find session")
        .expect("session not found");

    assert_eq!(session.id, session_id);
    assert_eq!(session.user_id.0, 42);
    assert_eq!(session.refresh_token_hash, refresh_token_hash);

    store
        .delete(session_id)
        .await
        .expect("failed to delete session");

    let session = store
        .find(session_id)
        .await
        .expect("failed to find deleted session");

    assert!(session.is_none());
}
