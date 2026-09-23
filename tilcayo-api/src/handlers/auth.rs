use std::sync::Arc;

use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use tilcayo_auth::{
    refresh_token::generate_refresh_token, service::AuthService, token::create_access_token,
};
use tilcayo_db::repositories::user::UserRepository;

use crate::{AppState, middleware::AuthUser};

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    session_id: String,
    refresh_token: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    access_token: String,
    refresh_token: String,
    session_id: String,
    token_type: &'static str,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error: &'static str,
}

#[derive(Serialize)]
pub struct MeResponse {
    id: usize,
    username: String,
    role_id: usize,
    role: String,
}

pub async fn login(
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

pub async fn refresh(
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

pub async fn me(Extension(auth_user): Extension<AuthUser>) -> impl IntoResponse {
    Json(MeResponse {
        id: auth_user.user.id.0,
        username: auth_user.user.username,
        role_id: auth_user.user.role_id.0,
        role: auth_user.role.name,
    })
}
