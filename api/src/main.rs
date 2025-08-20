use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use core::{
    Config, CreatePromptRequest, CreatePromptResponse, ErrorResponse, Prompt,
    create_database_pool, init_database, create_prompt, get_all_prompts,
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

async fn create_prompt_handler(
    State(pool): State<DatabaseConnection>, // extract db pool from api state (set in router declaration)
    Json(payload): Json<CreatePromptRequest>, // extract prompt json from request
) -> Result<Json<CreatePromptResponse>, (StatusCode, Json<ErrorResponse>)> {
    if payload.prompt.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Prompt cannot be empty".to_string(),
            }),
        ));
    }

    match create_prompt(&pool, payload.prompt).await {
        Ok(prompt) => Ok(Json(prompt)), // return prompt as json on success
        Err(e) => {
            eprintln!("Database error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create prompt".to_string(),
                }),
            )) // return error json on failure
        }
    }
}

async fn get_prompts_handler(
    State(pool): State<DatabaseConnection>, // extract db pool from api state (router declaration)
) -> Result<Json<Vec<Prompt>>, (StatusCode, Json<ErrorResponse>)> {
    match get_all_prompts(&pool).await {
        Ok(prompts) => Ok(Json(prompts)), // return all prompts as json on success
        Err(e) => {
            eprintln!("Database error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch prompts".to_string(),
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
    let config = Config::get();

    println!("🔧 Configuration loaded");
    println!("   Server: {}:{}", &config.api_server_host, &config.api_server_port);
    println!("   Max DB connections: {}", &config.max_connections);

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
        .route("/prompts", post(create_prompt_handler))
        .route("/prompts", get(get_prompts_handler))
        .layer(CorsLayer::permissive()) // this is not a good idea for production
        .with_state(db_connection_pool); // set the DatabaseConnection state

    let bind_address = format!("{}:{}", &config.api_server_host, &config.api_server_port);
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .context(format!("Failed to bind to {}", bind_address))?;

    println!("🚀 Server running on http://{}", bind_address);
    println!("📝 POST to /prompts to create a prompt");
    println!("📋 GET /prompts to view all prompts");
    println!("❤️ GET /health for health check");

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}