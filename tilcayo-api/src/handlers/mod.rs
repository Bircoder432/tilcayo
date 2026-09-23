pub mod auth;
pub mod resources;

use axum::response::IntoResponse;

pub async fn health() -> impl IntoResponse {
    "ok"
}
