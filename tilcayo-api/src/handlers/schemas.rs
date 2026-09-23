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

use crate::{AppState, middleware::AuthUser};

const SYSTEM_SCHEMA_NAME: &str = "schemas";

#[derive(Deserialize)]
pub struct CreateSchemaRequest {
    name: String,
    schema: ValueType,
}

#[derive(Deserialize)]
pub struct UpdateSchemaRequest {
    name: String,
    schema: ValueType,
}

#[derive(Serialize)]
pub struct SchemaResponse {
    id: usize,
    name: String,
    schema: ValueType,
}

#[derive(Serialize)]
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
            eprintln!("❌ ОШИБКА В SCHEMAS::LIST: {:?}", e);
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
        .map(|schema| SchemaResponse {
            id: schema.id.0,
            name: schema.name,
            schema: schema.schema,
        })
        .collect::<Vec<_>>();

    (StatusCode::OK, Json(response)).into_response()
}

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

    let Some(schema) = schema else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "schema not found",
            }),
        )
            .into_response();
    };

    (
        StatusCode::OK,
        Json(SchemaResponse {
            id: schema.id.0,
            name: schema.name,
            schema: schema.schema,
        }),
    )
        .into_response()
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<CreateSchemaRequest>,
) -> impl IntoResponse {
    if let Err(status) = check_schema_permission(&state, &auth_user, true).await {
        return status.into_response();
    }

    let repository = SchemaRepository::new(&state.database.pool);

    let schema_id = match repository.create(&payload.name, &payload.schema).await {
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

    if !exists {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "schema not found",
            }),
        )
            .into_response();
    }

    match repository
        .update(&SchemaId(id), &payload.name, &payload.schema)
        .await
    {
        Ok(true) => {}
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
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "schema does not exist or contains resources",
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
