mod auth;
mod db;
mod models;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Json, Router,
};
use dotenv::dotenv;
use reqwest::{header, Client};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::{self, TraceLayer};
use tracing::{error, info, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::auth::AuthUser;
use crate::db::{
    can_finish_today, create_practice_action, create_practice_record, create_user,
    get_practice_action, get_practice_records, get_user_by_username, list_actions_with_stats,
};
use crate::models::{
    CreateActionRequest, LoginRequest, LoginResponse, PracticeAction, PracticeRecord, QueryParams,
    RegisterRequest,
};

pub struct AppState {
    pub pool: sqlx::PgPool,
}

pub struct AppError(StatusCode, String);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = (self.0, self.1);
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => {
                error!("Database error: Row not found");
                AppError(StatusCode::NOT_FOUND, "Not found".to_string())
            }
            sqlx::Error::Database(e) => {
                error!("Database error: {}", e);
                if e.is_unique_violation() {
                    AppError(StatusCode::CONFLICT, "Resource already exists".to_string())
                } else {
                    AppError(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal server error".to_string(),
                    )
                }
            }
            _ => {
                error!("Unexpected database error: {}", err);
                AppError(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        }
    }
}

pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let password_hash = crate::auth::hash_password(&req.password).map_err(|_| {
        AppError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to hash password".to_string(),
        )
    })?;

    let user = create_user(&state.pool, &req.username, &password_hash).await?;

    let token = crate::auth::create_token(user.id).map_err(|_| {
        AppError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create token".to_string(),
        )
    })?;

    Ok(Json(LoginResponse { token, user }))
}

pub async fn login_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let user = get_user_by_username(&state.pool, &req.username)
        .await?
        .ok_or_else(|| AppError(StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))?;

    if !crate::auth::verify_password(&req.password, &user.password_hash) {
        return Err(AppError(
            StatusCode::UNAUTHORIZED,
            "Invalid credentials".to_string(),
        ));
    }

    let token = crate::auth::create_token(user.id).map_err(|_| {
        AppError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create token".to_string(),
        )
    })?;

    Ok(Json(LoginResponse { token, user }))
}

pub async fn create_action(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateActionRequest>,
) -> Result<Json<PracticeAction>, AppError> {
    println!("create action req: {:#?} userId {}", req, auth_user.user_id);
    let action = create_practice_action(&state.pool, auth_user.user_id, req.name).await?;
    Ok(Json(action))
}

pub async fn list_actions(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<crate::models::ActionWithStats>>, AppError> {
    let actions = list_actions_with_stats(&state.pool, auth_user.user_id).await?;
    Ok(Json(actions))
}

pub async fn get_action(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Option<PracticeAction>>, AppError> {
    let action = get_practice_action(&state.pool, auth_user.user_id, id).await?;
    Ok(Json(action))
}

pub async fn finish_action(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<PracticeRecord>, AppError> {
    // Check if action exists and belongs to user
    let action = get_practice_action(&state.pool, auth_user.user_id, id)
        .await?
        .ok_or_else(|| AppError(StatusCode::NOT_FOUND, "Action not found".to_string()))?;

    // Check if already completed today
    if !can_finish_today(&state.pool, auth_user.user_id, action.id).await? {
        return Err(AppError(
            StatusCode::CONFLICT,
            "Already completed today".to_string(),
        ));
    }

    let note = Some(String::new());
    let record = create_practice_record(&state.pool, auth_user.user_id, action.id, note).await?;
    Ok(Json(record))
}

pub async fn get_action_records(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<PracticeRecord>>, AppError> {
    let records = get_practice_records(&state.pool, auth_user.user_id, id).await?;
    Ok(Json(records))
}

async fn handle_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Not Found" })))
}

async fn get_coins(
    Query(params): Query<QueryParams>,
) -> Result<Json<Value>, axum::response::IntoResponse> {
    let mut param_map = HashMap::new();
    if let Some(param1) = params.ids {
        param_map.insert("ids".to_string(), param1);
    }
    param_map.insert("vs_currency", "usd");
    let mut key = "";
    if let Some(param2) = params.key {
        key = param2;
    }
    let client = Client::new();
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "accept",
        header::HeaderValue::from_str("application/json").unwrap(),
    );
    headers.insert("", header::HeaderValue::from_str(key).unwrap());
    let response = client
        .get("https://api.coingecko.com/api/v3/coins/markets")
        .query(&params)
        .headers(headers)
        .send()
        .await?;
    let body = response.json::<Value>().await?;
    Ok(Json(body))
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting application...");

    // Load .env file
    dotenv().ok();

    // Get database configuration from environment variables
    let db_user = env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string());
    let db_password = env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "postgres".to_string());
    let db_name = env::var("POSTGRES_DB").unwrap_or_else(|_| "postgres".to_string());
    let db_host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let db_port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());

    let db_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        db_user, db_password, db_host, db_port, db_name
    );
    info!("Database URL: {}", db_url);

    println!("Connecting to database...");
    let pool = db::init_db(&db_url)
        .await
        .expect("Failed to initialize database");
    info!("Database connection established");

    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    // Add tracing middleware
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
        .on_response(trace::DefaultOnResponse::new().level(Level::INFO));

    let app_state = Arc::new(AppState { pool });

    let app = Router::new()
        .route("/api/register", post(register_user))
        .route("/api/login", post(login_user))
        .route("/api/actions", post(create_action))
        .route("/api/actions", get(list_actions))
        .route("/api/actions/:id", get(get_action))
        .route("/api/actions/:id/records", get(get_action_records))
        .route("/api/actions/:id/finish", post(finish_action))
        .route("/api/coins", get(get_coins))
        .fallback(handle_404)
        .layer(trace_layer)
        .layer(cors)
        .with_state(app_state);

    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Server running on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
