use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tilcayo_core::{RoleId, SchemaId};
use tilcayo_db::repositories::{role::RoleRepository, schema::SchemaRepository};

use crate::{AppState, middleware::AuthUser};

#[derive(Deserialize)]
pub struct CreateRoleRequest {
    name: String,
}

#[derive(Deserialize)]
pub struct SchemaPermission {
    schema_id: usize,
    read: bool,
    write: bool,
}

#[derive(Deserialize)]
pub struct GrantPermissionsRequest {
    permissions: Vec<SchemaPermission>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error: &'static str,
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<CreateRoleRequest>,
) -> impl IntoResponse {
    let schemas = SchemaRepository::new(&state.database.pool);
    let role_schema = match schemas.find_by_name("roles").await {
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

    if !auth_user.role.can_write(&role_schema.id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let roles = RoleRepository::new(&state.database.pool);
    let empty_permissions = HashMap::new();

    match roles.create(&payload.name, &empty_permissions).await {
        Ok(role_id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "id": role_id.0, "name": payload.name })),
        )
            .into_response(),
        Err(e) => {
            if e.as_database_error()
                .is_some_and(|e| e.code().as_deref() == Some("23505"))
            {
                (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error: "role name already exists",
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

pub async fn grant_permissions(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Path(role_id): Path<usize>,
    Json(payload): Json<GrantPermissionsRequest>,
) -> impl IntoResponse {
    let schemas = SchemaRepository::new(&state.database.pool);
    let role_schema = match schemas.find_by_name("roles").await {
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

    if !auth_user.role.can_write(&role_schema.id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let roles = RoleRepository::new(&state.database.pool);
    let r_id = RoleId(role_id);

    if roles.find_by_id(&r_id).await.unwrap().is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "role not found",
            }),
        )
            .into_response();
    }

    for perm in payload.permissions {
        let s_id = SchemaId(perm.schema_id);
        if schemas.find_by_id(&s_id).await.unwrap().is_none() {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "schema not found",
                }),
            )
                .into_response();
        }

        if roles
            .set_permission(&r_id, &s_id, perm.read, perm.write)
            .await
            .is_err()
        {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    }

    StatusCode::OK.into_response()
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    let schemas = SchemaRepository::new(&state.database.pool);
    let role_schema = match schemas.find_by_name("roles").await {
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

    if !auth_user.role.can_read(&role_schema.id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let roles = RoleRepository::new(&state.database.pool);
    match roles.find_all().await {
        Ok(role_list) => {
            let response: Vec<_> = role_list
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id.0,
                        "name": r.name
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
    let role_schema = match schemas.find_by_name("roles").await {
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

    if !auth_user.role.can_read(&role_schema.id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let roles = RoleRepository::new(&state.database.pool);
    match roles.find_by_id(&RoleId(id)).await {
        Ok(Some(role)) => {
            let perms: Vec<_> = role
                .permissions
                .into_iter()
                .map(|(schema_id, perm)| {
                    serde_json::json!({
                        "schema_id": schema_id.0,
                        "read": perm.read,
                        "write": perm.write
                    })
                })
                .collect();

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "id": role.id.0,
                    "name": role.name,
                    "permissions": perms
                })),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "role not found",
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
