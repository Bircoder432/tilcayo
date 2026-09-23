use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use tilcayo_core::{RoleId, SchemaId};
use tilcayo_db::repositories::{role::RoleRepository, schema::SchemaRepository};
use tracing::{error, info};
use utoipa::ToSchema;

use crate::{AppState, middleware::AuthUser};

#[derive(Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    name: String,
}

#[derive(Deserialize, ToSchema)]
pub struct SchemaPermission {
    schema_id: usize,
    read: bool,
    write: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct GrantPermissionsRequest {
    permissions: Vec<SchemaPermission>,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    error: &'static str,
}

#[utoipa::path(
    get,
    path = "/roles",
    tag = "roles",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of roles", body = Vec<serde_json::Value>),
        (status = 403, description = "Access denied", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
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
            let count = role_list.len();
            let response: Vec<_> = role_list
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id.0,
                        "name": r.name
                    })
                })
                .collect();
            info!(count, "Roles list retrieved");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch roles");
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

#[utoipa::path(
    get,
    path = "/roles/{id}",
    tag = "roles",
    security(("bearer_auth" = [])),
    params(
        ("id" = usize, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "Role data with permissions", body = serde_json::Value),
        (status = 403, description = "Access denied", body = ErrorResponse),
        (status = 404, description = "Role not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
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

            info!(role_id = id, name = %role.name, "Role retrieved");
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
        Ok(None) => {
            info!(role_id = id, "Role not found");
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "role not found",
                }),
            )
                .into_response()
        }
        Err(e) => {
            error!(role_id = id, error = %e, "Database error fetching role");
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

#[utoipa::path(
    post,
    path = "/roles",
    tag = "roles",
    security(("bearer_auth" = [])),
    request_body = CreateRoleRequest,
    responses(
        (status = 201, description = "Role successfully created", body = serde_json::Value),
        (status = 403, description = "Access denied", body = ErrorResponse),
        (status = 409, description = "Role name already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
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
        Ok(role_id) => {
            info!(role_id = role_id.0, name = %payload.name, "Role created successfully");
            (
                StatusCode::CREATED,
                Json(serde_json::json!({ "id": role_id.0, "name": payload.name })),
            )
                .into_response()
        }
        Err(e) => {
            error!(name = %payload.name, error = %e, "Database error creating role");
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

#[utoipa::path(
    post,
    path = "/roles/{id}/permissions",
    tag = "roles",
    security(("bearer_auth" = [])),
    params(
        ("id" = usize, Path, description = "Role ID")
    ),
    request_body = GrantPermissionsRequest,
    responses(
        (status = 200, description = "Permissions successfully granted"),
        (status = 400, description = "Role or schema not found", body = ErrorResponse),
        (status = 403, description = "Access denied", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
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

    let perm_count = payload.permissions.len();
    for perm in &payload.permissions {
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
            error!(
                role_id = role_id,
                schema_id = perm.schema_id,
                "Failed to set permission"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    }

    info!(
        role_id = role_id,
        count = perm_count,
        "Permissions granted successfully"
    );
    StatusCode::OK.into_response()
}
