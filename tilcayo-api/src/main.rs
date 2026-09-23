use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use tilcayo_auth::session::SessionService;
use tilcayo_db::{Database, sessions::SessionStore};

mod bootstrap;
mod handlers;
mod middleware;

pub struct AppState {
    pub database: Database,
    pub sessions: SessionService,
    pub jwt_secret: Vec<u8>,
}

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET is not set");

    let database = Database::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".into());
    let admin_password = std::env::var("ADMIN_PASSWORD").ok();

    bootstrap::bootstrap(&database, &admin_username, admin_password.as_deref())
        .await
        .expect("failed to bootstrap application");

    let session_store =
        SessionStore::new("redis://localhost").expect("failed to create session store");

    let state = Arc::new(AppState {
        database,
        sessions: SessionService::new(session_store),
        jwt_secret: jwt_secret.into_bytes(),
    });

    let protected = Router::new()
        .route("/auth/me", get(handlers::auth::me))
        .route(
            "/users",
            get(handlers::users::list).post(handlers::users::create),
        )
        .route("/users/{id}", get(handlers::users::get))
        .route(
            "/roles",
            get(handlers::roles::list).post(handlers::roles::create),
        )
        .route("/roles/{id}", get(handlers::roles::get))
        .route(
            "/roles/{id}/permissions",
            post(handlers::roles::grant_permissions),
        )
        .route(
            "/schemas",
            get(handlers::schemas::list).post(handlers::schemas::create),
        )
        .route(
            "/schemas/{id}",
            get(handlers::schemas::get)
                .put(handlers::schemas::update)
                .delete(handlers::schemas::delete),
        )
        .route(
            "/{schema_name}",
            get(handlers::entities::list).post(handlers::entities::create),
        )
        .route(
            "/{schema_name}/{id}",
            get(handlers::entities::get)
                .put(handlers::entities::update)
                .delete(handlers::entities::delete),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::auth,
        ));

    let app = Router::new()
        .route("/", get(handlers::health))
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/refresh", post(handlers::auth::refresh))
        .merge(protected)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind server");

    println!("listening on http://0.0.0.0:3000");

    axum::serve(listener, app).await.expect("server error");
}
