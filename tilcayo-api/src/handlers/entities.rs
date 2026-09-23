use std::{collections::HashMap, sync::Arc};

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use tilcayo_core::{EntityId, Value};
use tilcayo_db::repositories::{entity::EntityRepository, schema::SchemaRepository};

use crate::{AppState, middleware::AuthUser};

#[derive(Deserialize)]
pub struct EntityQuery {
    page: Option<usize>,
    limit: Option<usize>,
    #[serde(flatten)]
    filters: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct EntityListResponse {
    items: Vec<serde_json::Value>,
    page: usize,
    limit: usize,
    total: i64,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error: &'static str,
}

fn is_system_schema(schema_name: &str) -> bool {
    matches!(
        schema_name,
        "user" | "role" | "users" | "roles" | "schema" | "schemas"
    )
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Path(schema_name): Path<String>,
    Query(query): Query<EntityQuery>,
) -> impl IntoResponse {
    if is_system_schema(&schema_name) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "system schemas are managed via dedicated endpoints",
            }),
        )
            .into_response();
    }

    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);

    if page == 0 || limit == 0 || limit > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid pagination",
            }),
        )
            .into_response();
    }

    let schemas = SchemaRepository::new(&state.database.pool);
    let schema = match schemas.find_by_name(&schema_name).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Database error finding schema by name: {:?}", e);
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

    if !auth_user.role.can_read(&schema.id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let filters: Vec<(String, String)> = query.filters.into_iter().collect();
    let entities = EntityRepository::new(&state.database.pool);

    let (items, total) = match entities
        .find_all(&schema_name, &schema.schema, &filters, page, limit)
        .await
    {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Database error in find_all: {:?}", e);
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
        Json(EntityListResponse {
            items,
            page,
            limit,
            total,
        }),
    )
        .into_response()
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Path(schema_name): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    if is_system_schema(&schema_name) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "system schemas are managed via dedicated endpoints",
            }),
        )
            .into_response();
    }

    let schemas = SchemaRepository::new(&state.database.pool);
    let schema = match schemas.find_by_name(&schema_name).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Database error finding schema by name: {:?}", e);
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

    if !auth_user.role.can_write(&schema.id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let data = match serde_json::from_value::<Value>(payload) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Deserialization error: {:?}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid entity data",
                }),
            )
                .into_response();
        }
    };

    let entities = EntityRepository::new(&state.database.pool);

    let entity_id = match entities.create(&schema_name, &schema.schema, &data).await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Database error in create: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    let Some(entity_id) = entity_id else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid entity data",
            }),
        )
            .into_response();
    };

    let response_data = serde_json::to_value(&data).expect("failed to serialize");

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": entity_id.0,
            "data": response_data
        })),
    )
        .into_response()
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Path((schema_name, id)): Path<(String, usize)>,
) -> impl IntoResponse {
    if is_system_schema(&schema_name) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "system schemas are managed via dedicated endpoints",
            }),
        )
            .into_response();
    }

    let schemas = SchemaRepository::new(&state.database.pool);
    let schema = match schemas.find_by_name(&schema_name).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Database error finding schema by name: {:?}", e);
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

    if !auth_user.role.can_read(&schema.id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let entities = EntityRepository::new(&state.database.pool);

    let entity = match entities.find_by_id(&schema_name, &EntityId(id)).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Database error in find_by_id: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    let Some(entity) = entity else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "entity not found",
            }),
        )
            .into_response();
    };

    (StatusCode::OK, Json(entity)).into_response()
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Path((schema_name, id)): Path<(String, usize)>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    if is_system_schema(&schema_name) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "system schemas are managed via dedicated endpoints",
            }),
        )
            .into_response();
    }

    let schemas = SchemaRepository::new(&state.database.pool);
    let schema = match schemas.find_by_name(&schema_name).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Database error finding schema by name: {:?}", e);
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

    if !auth_user.role.can_write(&schema.id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let data = match serde_json::from_value::<Value>(payload) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Deserialization error: {:?}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid entity data",
                }),
            )
                .into_response();
        }
    };

    let entities = EntityRepository::new(&state.database.pool);

    let updated = match entities
        .update(&schema_name, &schema.schema, &EntityId(id), &data)
        .await
    {
        Ok(u) => u,
        Err(e) => {
            eprintln!("Database error in update: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    if updated.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "entity not found or invalid data",
            }),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "id": id,
            "data": serde_json::to_value(&data).expect("failed to serialize")
        })),
    )
        .into_response()
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Path((schema_name, id)): Path<(String, usize)>,
) -> impl IntoResponse {
    if is_system_schema(&schema_name) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "system schemas are managed via dedicated endpoints",
            }),
        )
            .into_response();
    }

    let schemas = SchemaRepository::new(&state.database.pool);
    let schema = match schemas.find_by_name(&schema_name).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Database error finding schema by name: {:?}", e);
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

    if !auth_user.role.can_write(&schema.id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let entities = EntityRepository::new(&state.database.pool);

    match entities.delete(&schema_name, &EntityId(id)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            eprintln!("Database error in delete: {:?}", e);
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
