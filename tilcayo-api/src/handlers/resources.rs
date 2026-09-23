use std::{collections::HashMap, sync::Arc};

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use tilcayo_core::{ResourceId, SchemaId, Value};
use tilcayo_db::repositories::{resource::ResourceRepository, schema::SchemaRepository};

use crate::{AppState, middleware::AuthUser};

#[derive(Deserialize)]
pub struct ResourceQuery {
    schema_id: usize,
    page: Option<usize>,
    limit: Option<usize>,

    #[serde(flatten)]
    filters: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct CreateResourceRequest {
    schema_id: usize,
    data: serde_json::Value,
}

#[derive(Deserialize)]
pub struct UpdateResourceRequest {
    data: serde_json::Value,
}

#[derive(Serialize)]
pub struct ResourceResponse {
    id: usize,
    schema_id: usize,
    data: serde_json::Value,
}

#[derive(Serialize)]
pub struct ResourceListResponse {
    items: Vec<ResourceResponse>,
    page: usize,
    limit: usize,
    total: i64,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error: &'static str,
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<ResourceQuery>,
) -> impl IntoResponse {
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

    let schema_id = SchemaId(query.schema_id);

    let schemas = SchemaRepository::new(&state.database.pool);

    let Some(_) = (match schemas.find_by_id(&schema_id).await {
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
    }) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "schema not found",
            }),
        )
            .into_response();
    };

    if !auth_user.role.can_read(&schema_id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let filters = query.filters.into_iter().collect::<Vec<_>>();

    let resources = ResourceRepository::new(&state.database.pool);

    let (resources, total) = match resources.find_all(&schema_id, &filters, page, limit).await {
        Ok(result) => result,

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

    let items = resources
        .into_iter()
        .map(|resource| {
            Ok::<_, serde_json::Error>(ResourceResponse {
                id: resource.id.0,
                schema_id: resource.schema_id.0,
                data: serde_json::to_value(resource.data)?,
            })
        })
        .collect::<Result<Vec<_>, _>>();

    let Ok(items) = items else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "failed to serialize resource",
            }),
        )
            .into_response();
    };

    (
        StatusCode::OK,
        Json(ResourceListResponse {
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
    Json(payload): Json<CreateResourceRequest>,
) -> impl IntoResponse {
    let schema_id = SchemaId(payload.schema_id);

    let data = match serde_json::from_value::<Value>(payload.data) {
        Ok(data) => data,

        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid resource data",
                }),
            )
                .into_response();
        }
    };

    let schemas = SchemaRepository::new(&state.database.pool);

    let Some(_) = (match schemas.find_by_id(&schema_id).await {
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
    }) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "schema not found",
            }),
        )
            .into_response();
    };

    if !auth_user.role.can_write(&schema_id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let resources = ResourceRepository::new(&state.database.pool);

    let Some(resource_id) = (match resources.create(&schema_id, &data).await {
        Ok(resource_id) => resource_id,

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
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid resource data",
            }),
        )
            .into_response();
    };

    (
        StatusCode::CREATED,
        Json(ResourceResponse {
            id: resource_id.0,
            schema_id: schema_id.0,
            data: serde_json::to_value(data).expect("failed to serialize resource"),
        }),
    )
        .into_response()
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<usize>,
) -> impl IntoResponse {
    let resources = ResourceRepository::new(&state.database.pool);

    let resource = match resources.find_by_id(&ResourceId(id)).await {
        Ok(resource) => resource,

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

    let Some(resource) = resource else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "resource not found",
            }),
        )
            .into_response();
    };

    if !auth_user.role.can_read(&resource.schema_id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let data = match serde_json::to_value(resource.data) {
        Ok(data) => data,

        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "failed to serialize resource",
                }),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(ResourceResponse {
            id: resource.id.0,
            schema_id: resource.schema_id.0,
            data,
        }),
    )
        .into_response()
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<usize>,
    Json(payload): Json<UpdateResourceRequest>,
) -> impl IntoResponse {
    let resource_id = ResourceId(id);

    let data = match serde_json::from_value::<Value>(payload.data) {
        Ok(data) => data,

        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid resource data",
                }),
            )
                .into_response();
        }
    };

    let resources = ResourceRepository::new(&state.database.pool);

    let resource = match resources.find_by_id(&resource_id).await {
        Ok(resource) => resource,

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

    let Some(resource) = resource else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "resource not found",
            }),
        )
            .into_response();
    };

    if !auth_user.role.can_write(&resource.schema_id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    let updated = match resources.update(&resource_id, &data).await {
        Ok(updated) => updated,

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

    if updated.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid resource data",
            }),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(ResourceResponse {
            id: resource_id.0,
            schema_id: resource.schema_id.0,
            data: serde_json::to_value(data).expect("failed to serialize resource"),
        }),
    )
        .into_response()
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<usize>,
) -> impl IntoResponse {
    let resource_id = ResourceId(id);

    let resources = ResourceRepository::new(&state.database.pool);

    let resource = match resources.find_by_id(&resource_id).await {
        Ok(resource) => resource,

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

    let Some(resource) = resource else {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "resource not found",
            }),
        )
            .into_response();
    };

    if !auth_user.role.can_write(&resource.schema_id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: "forbidden" }),
        )
            .into_response();
    }

    match resources.delete(&resource_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),

        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "internal server error",
            }),
        )
            .into_response(),
    }
}
