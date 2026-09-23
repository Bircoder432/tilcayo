use axum::extract::Path;
use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tilcayo_auth::password::hash_password;
use tilcayo_core::RoleId;
use tilcayo_core::UserId;
use tilcayo_db::repositories::{
    role::RoleRepository, schema::SchemaRepository, user::UserRepository,
};

use crate::{AppState, middleware::AuthUser};

#[derive(Deserialize)]
pub struct CreateUserRequest {
    username: String,
    role_id: usize,
    password: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error: &'static str,
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let schemas = SchemaRepository::new(&state.database.pool);
    let user_schema = match schemas.find_by_name("users").await {
        Ok(Some(s)) => s,
        Ok(None) => {
            eprintln!("❌ ОШИБКА: Схема 'users' не найдена в БД");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
        Err(e) => {
            eprintln!("❌ ОШИБКА БД при поиске схемы 'users': {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    if !auth_user.role.can_write(&user_schema.id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let password_hash = match hash_password(&payload.password) {
        Ok(hash) => hash,
        Err(e) => {
            eprintln!("❌ ОШИБКА хэширования пароля: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "failed to hash password",
                }),
            )
                .into_response();
        }
    };

    let users = UserRepository::new(&state.database.pool);
    let role_id = RoleId(payload.role_id);

    let roles = RoleRepository::new(&state.database.pool);
    let role_check = match roles.find_by_id(&role_id).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ ОШИБКА БД при поиске роли {}: {:?}", payload.role_id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    if role_check.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "role not found",
            }),
        )
            .into_response();
    }

    match users
        .create(&payload.username, &role_id, &password_hash)
        .await
    {
        Ok(user_id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "id": user_id.0, "username": payload.username })),
        )
            .into_response(),
        Err(e) => {
            eprintln!("❌ ОШИБКА БД при создании пользователя: {:?}", e);
            if e.as_database_error()
                .is_some_and(|e| e.code().as_deref() == Some("23505"))
            {
                (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error: "username already exists",
                    }),
                )
                    .into_response()
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "internal server error",
                    }),
                )
                    .into_response()
            }
        }
    }
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    let schemas = SchemaRepository::new(&state.database.pool);
    let user_schema = match schemas.find_by_name("users").await {
        Ok(Some(s)) => s,
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    if !auth_user.role.can_read(&user_schema.id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let users = UserRepository::new(&state.database.pool);
    match users.find_all().await {
        Ok(user_list) => {
            let response: Vec<_> = user_list
                .into_iter()
                .map(|u| {
                    serde_json::json!({
                        "id": u.id.0,
                        "username": u.username,
                        "role_id": u.role_id.0
                    })
                })
                .collect();
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "internal server error",
            }),
        )
            .into_response(),
    }
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<usize>,
) -> impl IntoResponse {
    let schemas = SchemaRepository::new(&state.database.pool);
    let user_schema = match schemas.find_by_name("users").await {
        Ok(Some(s)) => s,
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    if !auth_user.role.can_read(&user_schema.id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let users = UserRepository::new(&state.database.pool);
    match users.find_by_id(&UserId(id)).await {
        Ok(Some(user)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "id": user.id.0,
                "username": user.username,
                "role_id": user.role_id.0
            })),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "user not found",
            }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "internal server error",
            }),
        )
            .into_response(),
    }
}
