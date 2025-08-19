use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use core::{
    Config, CreateMessageRequest, CreateMessageResponse, ErrorResponse, Message,
    create_database_pool, init_database, create_message, get_all_messages,
};
// pool of mysql connections
use sqlx::mysql::MySqlPool;

use std::sync::Arc;
use tower_http::cors::CorsLayer;
use anyhow::{Context, Result};

// Map Arc<MySqlPool> as the type DatabaseConnection
// This allows the pool state to be extracted based on the state
// passed in when the router is initialized
// separates the db implementation from DatabaseConnection type in handlers
type DatabaseConnection = Arc<MySqlPool>;

async fn create_message_handler(
    State(pool): State<DatabaseConnection>, // extract db pool from api state (set in router declaration)
    Json(payload): Json<CreateMessageRequest>, // extract message json from request
) -> Result<Json<CreateMessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    if payload.message.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Message cannot be empty".to_string(),
            }),
        ));
    }

    match create_message(&pool, payload.message).await {
        Ok(message) => Ok(Json(message)), // return message as json on success
        Err(e) => {
            eprintln!("Database error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create message".to_string(),
                }),
            )) // return error json on failure
        }
    }
}

async fn get_messages_handler(
    State(pool): State<DatabaseConnection>, // extract db pool from api state (router declaration)
) -> Result<Json<Vec<Message>>, (StatusCode, Json<ErrorResponse>)> {
    match get_all_messages(&pool).await {
        Ok(messages) => Ok(Json(messages)), // return all messages as json on success
        Err(e) => {
            eprintln!("Database error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch messages".to_string(),
                }),
            )) // return error json on failure
        }
    }
}

async fn health_check() -> &'static str {
    "API is running!"
}

// Create a multi-threaded Tokio runtime for the api server
#[tokio::main]
async fn main() -> Result<()> {
    // load config from .env file
    let config = Config::from_env()
        .context("Failed to load configuration")?;

    println!("🔧 Configuration loaded");
    println!("   Server: {}:{}", config.server_host, config.server_port);
    println!("   Max DB connections: {}", config.max_connections);

    // create mysql pool using properties in config
    let pool = create_database_pool(&config).await?;

    // wait for the pool to initialize
    init_database(&pool)
        .await
        .context("Failed to initialize database")?;

    // Wrap db pool in a thread safe reference
    let db_connection_pool = Arc::new(pool);

    // initialize app with routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/messages", post(create_message_handler))
        .route("/messages", get(get_messages_handler))
        .layer(CorsLayer::permissive()) // this is not a good idea for production
        .with_state(db_connection_pool); // set the DatabaseConnection state

    let bind_address = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .context(format!("Failed to bind to {}", bind_address))?;

    println!("🚀 Server running on http://{}", bind_address);
    println!("📝 POST to /messages to create a message");
    println!("📋 GET /messages to view all messages");
    println!("❤️ GET /health for health check");

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}