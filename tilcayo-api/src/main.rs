use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use dotenvy::dotenv;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

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

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::auth::login,
        handlers::auth::refresh,
        handlers::auth::me,
        handlers::users::list,
        handlers::users::get,
        handlers::users::create,
        handlers::roles::list,
        handlers::roles::get,
        handlers::roles::create,
        handlers::roles::grant_permissions,
        handlers::schemas::list,
        handlers::schemas::get,
        handlers::schemas::create,
        handlers::schemas::update,
        handlers::schemas::delete,
        handlers::entities::list,
        handlers::entities::get,
        handlers::entities::create,
        handlers::entities::update,
        handlers::entities::delete,
    ),
    components(
        schemas(
            handlers::auth::LoginRequest,
            handlers::auth::LoginResponse,
            handlers::auth::RefreshRequest,
            handlers::auth::MeResponse,
            handlers::auth::ErrorResponse,
            handlers::users::CreateUserRequest,
            handlers::roles::CreateRoleRequest,
            handlers::roles::GrantPermissionsRequest,
            handlers::roles::SchemaPermission,
            handlers::schemas::CreateSchemaRequest,
            handlers::schemas::UpdateSchemaRequest,
            handlers::schemas::SchemaResponse,
            handlers::entities::EntityListResponse,
            handlers::entities::ErrorResponse,
        )
    ),
    tags(
        (name = "auth", description = "Authentication and sessions"),
        (name = "users", description = "User management"),
        (name = "roles", description = "Role and permission management"),
        (name = "schemas", description = "Data schema management"),
        (name = "entities", description = "Dynamic entities (CRUD by schema)")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let bind_address = format!("{}:{}", host, port);

    let database = Database::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".into());
    let admin_password = std::env::var("ADMIN_PASSWORD").ok();

    if let Err(e) =
        bootstrap::bootstrap(&database, &admin_username, admin_password.as_deref()).await
    {
        tracing::error!("Failed to bootstrap application: {:?}", e);
        panic!("Bootstrap failed");
    }

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
    let session_store = SessionStore::new(&redis_url).expect("failed to create session store");

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
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(protected)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .expect("failed to bind server");

    tracing::info!("Listening on http://{}", bind_address);
    tracing::info!("Swagger UI available at http://{}/swagger-ui", bind_address);

    axum::serve(listener, app).await.expect("server error");
}
