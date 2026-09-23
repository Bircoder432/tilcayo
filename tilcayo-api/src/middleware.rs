use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use tilcayo_auth::token::verify_access_token;
use tilcayo_core::{Role, User};
use tilcayo_db::repositories::{role::RoleRepository, user::UserRepository};

use crate::AppState;

#[derive(Clone)]
pub struct AuthUser {
    pub user: User,
    pub role: Role,
}

pub async fn auth(
    State(state): State<std::sync::Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(header) = request.headers().get(header::AUTHORIZATION) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let Ok(header) = header.to_str() else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let Some(token) = header.strip_prefix("Bearer ") else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let claims = match verify_access_token(token, &state.jwt_secret) {
        Ok(claims) => claims,
        Err(_) => {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    let users = UserRepository::new(&state.database.pool);

    let Some(user) = (match users.find_by_id(&tilcayo_core::UserId(claims.sub)).await {
        Ok(user) => user,
        Err(_) => {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let roles = RoleRepository::new(&state.database.pool);

    let Some(role) = (match roles.find_by_id(&user.role_id).await {
        Ok(role) => role,
        Err(_) => {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    request.extensions_mut().insert(AuthUser { user, role });

    next.run(request).await
}
