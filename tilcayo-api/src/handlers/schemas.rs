use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use tilcayo_core::{SchemaId, ValueType};
use tilcayo_db::repositories::{role::RoleRepository, schema::SchemaRepository};
use tracing::{error, info};
use utoipa::ToSchema;

use crate::{AppState, middleware::AuthUser};

const SYSTEM_SCHEMA_NAME: &str = "schemas";

#[derive(Deserialize, ToSchema)]
pub struct CreateSchemaRequest {
    name: String,
    #[schema(value_type = Object)]
    schema: serde_json::Value,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateSchemaRequest {
    name: String,
    #[schema(value_type = Object)]
    schema: serde_json::Value,
}

#[derive(Serialize, ToSchema)]
pub struct SchemaResponse {
    id: usize,
    name: String,
    #[schema(value_type = Object)]
    schema: serde_json::Value,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    error: &'static str,
}

async fn check_schema_permission(
    state: &AppState,
    auth_user: &AuthUser,
    write: bool,
) -> Result<(), StatusCode> {
    let schemas = SchemaRepository::new(&state.database.pool);

    let schema = schemas
        .find_by_name(SYSTEM_SCHEMA_NAME)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(schema) = schema else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let allowed = if write {
        auth_user.role.can_write(&schema.id)
    } else {
        auth_user.role.can_read(&schema.id)
    };

    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(())
}

#[utoipa::path(
    get,
    path = "/schemas",
    tag = "schemas",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of all schemas", body = Vec<SchemaResponse>),
        (status = 403, description = "Access denied", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    if let Err(status) = check_schema_permission(&state, &auth_user, false).await {
        return status.into_response();
    }

    let repository = SchemaRepository::new(&state.database.pool);

    let schemas = match repository.find_all().await {
        Ok(schemas) => schemas,
        Err(e) => {
            error!(error = %e, "Failed to fetch schemas");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    let response = schemas
        .into_iter()
        .map(|schema| {
            let schema_val = serde_json::to_value(&schema.schema).unwrap_or_default();
            SchemaResponse {
                id: schema.id.0,
                name: schema.name,
                schema: schema_val,
            }
        })
        .collect::<Vec<_>>();

    (StatusCode::OK, Json(response)).into_response()
}

#[utoipa::path(
    get,
    path = "/schemas/{id}",
    tag = "schemas",
    security(("bearer_auth" = [])),
    params(
        ("id" = usize, Path, description = "Schema ID")
    ),
    responses(
        (status = 200, description = "Schema data", body = SchemaResponse),
        (status = 403, description = "Access denied", body = ErrorResponse),
        (status = 404, description = "Schema not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<usize>,
) -> impl IntoResponse {
    if let Err(status) = check_schema_permission(&state, &auth_user, false).await {
        return status.into_response();
    }

    let repository = SchemaRepository::new(&state.database.pool);

    let schema = match repository.find_by_id(&SchemaId(id)).await {
        Ok(schema) => schema,
        Err(e) => {
            error!(schema_id = id, error = %e, "Database error fetching schema");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    let Some(schema) = schema else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "schema not found",
            }),
        )
            .into_response();
    };

    let schema_val = serde_json::to_value(&schema.schema).unwrap_or_default();

    (
        StatusCode::OK,
        Json(SchemaResponse {
            id: schema.id.0,
            name: schema.name,
            schema: schema_val,
        }),
    )
        .into_response()
}

#[utoipa::path(
    post,
    path = "/schemas",
    tag = "schemas",
    security(("bearer_auth" = [])),
    request_body = CreateSchemaRequest,
    responses(
        (status = 201, description = "Schema successfully created", body = SchemaResponse),
        (status = 400, description = "Invalid schema format", body = ErrorResponse),
        (status = 403, description = "Access denied", body = ErrorResponse),
        (status = 409, description = "Schema name already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn create(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<CreateSchemaRequest>,
) -> impl IntoResponse {
    if let Err(status) = check_schema_permission(&state, &auth_user, true).await {
        return status.into_response();
    }

    let schema: ValueType = match serde_json::from_value(payload.schema.clone()) {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Failed to parse schema");
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid schema format",
                }),
            )
                .into_response();
        }
    };

    let repository = SchemaRepository::new(&state.database.pool);

    let schema_id = match repository.create(&payload.name, &schema).await {
        Ok(id) => id,
        Err(error) => {
            let duplicate = error
                .as_database_error()
                .is_some_and(|error| error.code().as_deref() == Some("23505"));

            if duplicate {
                return (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error: "schema name already exists",
                    }),
                )
                    .into_response();
            }

            error!(error = %error, "Database error creating schema");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    if auth_user.role.name == "admin" {
        let roles = RoleRepository::new(&state.database.pool);

        if roles
            .set_permission(&auth_user.role.id, &schema_id, true, true)
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

    info!(schema_id = schema_id.0, name = %payload.name, "Schema created successfully");
    (
        StatusCode::CREATED,
        Json(SchemaResponse {
            id: schema_id.0,
            name: payload.name,
            schema: payload.schema,
        }),
    )
        .into_response()
}

#[utoipa::path(
    put,
    path = "/schemas/{id}",
    tag = "schemas",
    security(("bearer_auth" = [])),
    params(
        ("id" = usize, Path, description = "Schema ID")
    ),
    request_body = UpdateSchemaRequest,
    responses(
        (status = 200, description = "Schema successfully updated", body = SchemaResponse),
        (status = 400, description = "Invalid schema format", body = ErrorResponse),
        (status = 403, description = "Access denied", body = ErrorResponse),
        (status = 404, description = "Schema not found", body = ErrorResponse),
        (status = 409, description = "Schema name already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<usize>,
    Json(payload): Json<UpdateSchemaRequest>,
) -> impl IntoResponse {
    if let Err(status) = check_schema_permission(&state, &auth_user, true).await {
        return status.into_response();
    }

    let repository = SchemaRepository::new(&state.database.pool);

    let exists = match repository.find_by_id(&SchemaId(id)).await {
        Ok(schema) => schema.is_some(),
        Err(e) => {
            error!(schema_id = id, error = %e, "Database error checking schema existence");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    if !exists {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "schema not found",
            }),
        )
            .into_response();
    }

    let schema: ValueType = match serde_json::from_value(payload.schema.clone()) {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Failed to parse schema");
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid schema format",
                }),
            )
                .into_response();
        }
    };

    match repository
        .update(&SchemaId(id), &payload.name, &schema)
        .await
    {
        Ok(true) => {
            info!(schema_id = id, name = %payload.name, "Schema updated successfully");
        }
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "schema not found",
                }),
            )
                .into_response();
        }
        Err(error) => {
            let duplicate = error
                .as_database_error()
                .is_some_and(|error| error.code().as_deref() == Some("23505"));

            if duplicate {
                return (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error: "schema name already exists",
                    }),
                )
                    .into_response();
            }

            error!(schema_id = id, error = %error, "Database error updating schema");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    }

    (
        StatusCode::OK,
        Json(SchemaResponse {
            id,
            name: payload.name,
            schema: payload.schema,
        }),
    )
        .into_response()
}

#[utoipa::path(
    delete,
    path = "/schemas/{id}",
    tag = "schemas",
    security(("bearer_auth" = [])),
    params(
        ("id" = usize, Path, description = "Schema ID")
    ),
    responses(
        (status = 204, description = "Schema successfully deleted"),
        (status = 403, description = "Access denied", body = ErrorResponse),
        (status = 409, description = "Schema does not exist or contains resources", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<usize>,
) -> impl IntoResponse {
    if let Err(status) = check_schema_permission(&state, &auth_user, true).await {
        return status.into_response();
    }

    let repository = SchemaRepository::new(&state.database.pool);

    match repository.delete(&SchemaId(id)).await {
        Ok(true) => {
            info!(schema_id = id, "Schema deleted successfully");
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "schema does not exist or contains resources",
            }),
        )
            .into_response(),
        Err(e) => {
            error!(schema_id = id, error = %e, "Database error deleting schema");
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
