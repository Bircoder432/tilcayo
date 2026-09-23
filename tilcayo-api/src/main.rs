use std::sync::Arc;

use axum::{
    Router,
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use tilcayo_auth::{
    refresh_token::generate_refresh_token, service::AuthService, session::SessionService,
    token::create_access_token,
};
use tilcayo_db::{Database, repositories::user::UserRepository, sessions::SessionStore};

mod bootstrap;

struct AppState {
    database: Database,
    sessions: SessionService,
    jwt_secret: Vec<u8>,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct RefreshRequest {
    session_id: String,
    refresh_token: String,
}

#[derive(Serialize)]
struct LoginResponse {
    access_token: String,
    refresh_token: String,
    session_id: String,
    token_type: &'static str,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

async fn health() -> &'static str {
    "ok"
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let users = UserRepository::new(&state.database.pool);

    let auth = AuthService::new(users);

    let Some(user) = (match auth
        .authenticate(&payload.username, &payload.password)
        .await
    {
        Ok(user) => user,

        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    }) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid credentials",
            }),
        )
            .into_response();
    };

    let access_token = match create_access_token(&user.id, &state.jwt_secret) {
        Ok(token) => token,

        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    let session_id = match generate_refresh_token() {
        Ok(session_id) => session_id,

        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    let session = match state.sessions.create(&user.id, &session_id).await {
        Ok(session) => session,

        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(LoginResponse {
            access_token,
            refresh_token: session.refresh_token,
            session_id: session.session_id,
            token_type: "Bearer",
        }),
    )
        .into_response()
}

async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RefreshRequest>,
) -> impl IntoResponse {
    let session = match state
        .sessions
        .refresh(&payload.session_id, &payload.refresh_token)
        .await
    {
        Ok(session) => session,

        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    let Some(session) = session else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid refresh session",
            }),
        )
            .into_response();
    };

    let access_token = match create_access_token(&session.user_id, &state.jwt_secret) {
        Ok(token) => token,

        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(LoginResponse {
            access_token,
            refresh_token: session.refresh_token,
            session_id: session.session_id,
            token_type: "Bearer",
        }),
    )
        .into_response()
}

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");

    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET is not set");

    let database = Database::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".into());

    let admin_password = std::env::var("ADMIN_PASSWORD");

    let has_users = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM users
        )
        "#
    )
    .fetch_one(&database.pool)
    .await
    .expect("failed to check users")
    .unwrap_or(false);

    if !has_users {
        let admin_password = admin_password.expect("ADMIN_PASSWORD is not set");

        bootstrap::bootstrap(&database, &admin_username, &admin_password)
            .await
            .expect("failed to bootstrap application");
    } else {
        bootstrap::bootstrap(&database, &admin_username, "")
            .await
            .expect("failed to initialize application");
    }

    let session_store =
        SessionStore::new("redis://localhost").expect("failed to create session store");

    let state = Arc::new(AppState {
        database,
        sessions: SessionService::new(session_store),
        jwt_secret: jwt_secret.into_bytes(),
    });

    let app = Router::new()
        .route("/", get(health))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind server");

    println!("listening on http://0.0.0.0:3000");

    axum::serve(listener, app).await.expect("server error");
}
