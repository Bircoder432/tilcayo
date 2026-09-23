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
use tracing::{error, info};
use utoipa::ToSchema;

use crate::{AppState, middleware::AuthUser};

#[derive(Deserialize)]
pub struct EntityQuery {
    page: Option<usize>,
    limit: Option<usize>,
    #[serde(flatten)]
    filters: HashMap<String, String>,
}

#[derive(Serialize, ToSchema)]
pub struct EntityListResponse {
    items: Vec<serde_json::Value>,
    page: usize,
    limit: usize,
    total: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    error: &'static str,
}

fn is_system_schema(schema_name: &str) -> bool {
    matches!(
        schema_name,
        "user" | "role" | "users" | "roles" | "schema" | "schemas"
    )
}

#[utoipa::path(
    get,
    path = "/{schema_name}",
    tag = "entities",
    security(("bearer_auth" = [])),
    params(
        ("schema_name" = String, Path, description = "Schema (table) name"),
        ("page" = Option<usize>, Query, description = "Page number (default 1)"),
        ("limit" = Option<usize>, Query, description = "Number of items per page (default 20, maximum 100)")
    ),
    responses(
        (status = 200, description = "List of entities", body = EntityListResponse),
        (status = 400, description = "Invalid pagination", body = ErrorResponse),
        (status = 403, description = "Access denied or system schema", body = ErrorResponse),
        (status = 404, description = "Schema not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
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
            error!(schema_name = %schema_name, error = %e, "Database error finding schema by name");
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
            error!(schema_name = %schema_name, error = %e, "Database error in find_all");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal server error",
                }),
            )
                .into_response();
        }
    };

    info!(schema_name = %schema_name, count = items.len(), total, "Entities list retrieved");
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

#[utoipa::path(
    post,
    path = "/{schema_name}",
    tag = "entities",
    security(("bearer_auth" = [])),
    params(
        ("schema_name" = String, Path, description = "Schema (table) name")
    ),
    request_body(content = serde_json::Value, description = "Entity data according to the schema"),
    responses(
        (status = 201, description = "Entity successfully created", body = serde_json::Value),
        (status = 400, description = "Invalid entity data", body = ErrorResponse),
        (status = 403, description = "Access denied or system schema", body = ErrorResponse),
        (status = 404, description = "Schema not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
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
            error!(schema_name = %schema_name, error = %e, "Database error finding schema by name");
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
            error!(schema_name = %schema_name, error = %e, "Deserialization error");
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
            error!(schema_name = %schema_name, error = %e, "Database error in create");
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

    info!(schema_name = %schema_name, entity_id = entity_id.0, "Entity created successfully");
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

#[utoipa::path(
    get,
    path = "/{schema_name}/{id}",
    tag = "entities",
    security(("bearer_auth" = [])),
    params(
        ("schema_name" = String, Path, description = "Schema (table) name"),
        ("id" = usize, Path, description = "Entity ID")
    ),
    responses(
        (status = 200, description = "Entity data", body = serde_json::Value),
        (status = 403, description = "Access denied or system schema", body = ErrorResponse),
        (status = 404, description = "Schema or entity not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
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
            error!(schema_name = %schema_name, entity_id = id, error = %e, "Database error finding schema");
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
            error!(schema_name = %schema_name, entity_id = id, error = %e, "Database error in find_by_id");
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

    info!(schema_name = %schema_name, entity_id = id, "Entity retrieved");
    (StatusCode::OK, Json(entity)).into_response()
}

#[utoipa::path(
    put,
    path = "/{schema_name}/{id}",
    tag = "entities",
    security(("bearer_auth" = [])),
    params(
        ("schema_name" = String, Path, description = "Schema (table) name"),
        ("id" = usize, Path, description = "Entity ID")
    ),
    request_body(content = serde_json::Value, description = "New entity data according to the schema"),
    responses(
        (status = 200, description = "Entity successfully updated", body = serde_json::Value),
        (status = 400, description = "Invalid entity data", body = ErrorResponse),
        (status = 403, description = "Access denied or system schema", body = ErrorResponse),
        (status = 404, description = "Schema or entity not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
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
            error!(schema_name = %schema_name, entity_id = id, error = %e, "Database error finding schema");
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
            error!(schema_name = %schema_name, entity_id = id, error = %e, "Deserialization error");
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
            error!(schema_name = %schema_name, entity_id = id, error = %e, "Database error in update");
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
        info!(schema_name = %schema_name, entity_id = id, "Entity update returned none (not found or invalid)");
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "entity not found or invalid data",
            }),
        )
            .into_response();
    }

    info!(schema_name = %schema_name, entity_id = id, "Entity updated successfully");
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "id": id,
            "data": serde_json::to_value(&data).expect("failed to serialize")
        })),
    )
        .into_response()
}

#[utoipa::path(
    delete,
    path = "/{schema_name}/{id}",
    tag = "entities",
    security(("bearer_auth" = [])),
    params(
        ("schema_name" = String, Path, description = "Schema (table) name"),
        ("id" = usize, Path, description = "Entity ID")
    ),
    responses(
        (status = 204, description = "Entity successfully deleted"),
        (status = 403, description = "Access denied or system schema", body = ErrorResponse),
        (status = 404, description = "Schema not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
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
            error!(schema_name = %schema_name, entity_id = id, error = %e, "Database error finding schema");
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
        Ok(()) => {
            info!(schema_name = %schema_name, entity_id = id, "Entity deleted successfully");
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            error!(schema_name = %schema_name, entity_id = id, error = %e, "Database error in delete");
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
