use tilcayo_auth::session::SessionService;
use tilcayo_core::UserId;
use tilcayo_db::sessions::SessionStore;

#[tokio::test]
async fn creates_refresh_session() {
    let store = SessionStore::new("redis://localhost").expect("failed to create session store");

    let service = SessionService::new(store);

    let user_id = UserId(42);
    let session_id = "test-auth-session";

    let session = service
        .create(&user_id, session_id)
        .await
        .expect("failed to create refresh session");

    assert_eq!(session.session_id, session_id);
    assert_eq!(session.refresh_token.len(), 64);
    assert_eq!(session.user_id.0, 42);
    let store = SessionStore::new("redis://localhost").expect("failed to create session store");

    let stored = store
        .find(session_id)
        .await
        .expect("failed to find session")
        .expect("session not found");

    assert_eq!(stored.user_id.0, 42);

    assert_ne!(stored.refresh_token_hash, session.refresh_token);

    store
        .delete(session_id)
        .await
        .expect("failed to delete session");
}

#[tokio::test]
async fn refresh_token_is_rotated() {
    let store = SessionStore::new("redis://localhost").expect("failed to create session store");

    let service = SessionService::new(store);

    let user_id = UserId(42);
    let session_id = "test-refresh-rotation";

    let first = service
        .create(&user_id, session_id)
        .await
        .expect("failed to create session");

    let second = service
        .refresh(session_id, &first.refresh_token)
        .await
        .expect("failed to refresh session")
        .expect("refresh token was rejected");

    assert_eq!(second.session_id, session_id);
    assert_ne!(first.refresh_token, second.refresh_token);

    let old_token = service
        .refresh(session_id, &first.refresh_token)
        .await
        .expect("failed to check old refresh token");

    assert!(old_token.is_none());

    let new_token = service
        .refresh(session_id, &second.refresh_token)
        .await
        .expect("failed to use rotated refresh token");

    assert!(new_token.is_some());

    let store = SessionStore::new("redis://localhost").expect("failed to create session store");

    store
        .delete(session_id)
        .await
        .expect("failed to delete session");
}

#[tokio::test]
async fn refresh_unknown_session_returns_none() {
    let store = SessionStore::new("redis://localhost").expect("failed to create session store");

    let service = SessionService::new(store);

    let result = service
        .refresh("does-not-exist", "some-refresh-token")
        .await
        .expect("refresh failed");

    assert!(result.is_none());
}
