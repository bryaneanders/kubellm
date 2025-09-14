mod prompt;

use crate::prompt::{create_prompt_handler, get_prompts_handler, get_providers_handler};
use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use kubellm_core::{create_database_pool, init_database};
use prompts_api::{get_models_handler, ApiConfig};
use serde_json::json;
use sqlx::MySqlPool;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

async fn health_check() -> &'static str {
    "API is running!"
}

async fn readiness_check(
    State(pool): State<Arc<MySqlPool>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Test database connection
    match sqlx::query("SELECT 1").fetch_one(pool.as_ref()).await {
        Ok(_) => Ok(Json(json!({
            "status": "ready",
            "database": "connected"
        }))),
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

// Create a multi-threaded Tokio runtime for the api server
#[tokio::main]
async fn main() -> Result<()> {
    let core_config = kubellm_core::CoreConfig::get();
    let api_config = ApiConfig::get();

    println!("🔧 Configuration loaded");
    println!(
        "   Server: {}:{}",
        &api_config.api_server_host, &api_config.api_server_port
    );
    println!("   Max DB connections: {}", core_config.max_connections);

    // create mysql pool using properties in config
    let pool = create_database_pool(core_config).await?;

    // wait for the pool to initialize
    init_database(&pool)
        .await
        .context("Failed to initialize database")?;

    // Wrap db pool in a thread safe reference
    let db_connection_pool = Arc::new(pool);

    // initialize app with routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/prompt", post(create_prompt_handler))
        .route("/prompts", get(get_prompts_handler))
        .route("/get-models", get(get_models_handler))
        .route("/get-providers", get(get_providers_handler))
        .layer(CorsLayer::permissive()) // this is not a good idea for production
        .with_state(db_connection_pool); // set the DatabaseConnection state

    let bind_address = format!(
        "{}:{}",
        &api_config.api_server_host, &api_config.api_server_port
    );
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .context(format!("Failed to bind to {}", bind_address))?;

    println!("🚀 Server running on http://{}", bind_address);
    println!("📝 POST to /prompt to create a prompt");
    println!("📋 GET /prompts to view all prompts");
    println!("⚛️ GET /models to view a provider's models");
    println!("❤️ GET /health for health check");
    println!("✅ GET /ready for readiness check");

    axum::serve(listener, app).await.context("Server error")?;

    Ok(())
}
