pub mod auth;
pub mod entities;
pub mod roles;
pub mod schemas;
pub mod users;

use axum::response::IntoResponse;

pub async fn health() -> impl IntoResponse {
    "ok"
}
