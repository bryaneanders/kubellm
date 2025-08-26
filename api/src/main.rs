mod prompt;

use axum::{
    routing::{get, post},
    Router,
};
use core::{
    create_database_pool, init_database,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use anyhow::{Context, Result};
use prompts_api::{get_models_handler, ApiConfig};
use crate::prompt::{create_prompt_handler, get_prompts_handler,};

async fn health_check() -> &'static str {
    "API is running!"
}

// Create a multi-threaded Tokio runtime for the api server
#[tokio::main]
async fn main() -> Result<()> {
    let core_config = core::CoreConfig::get();
    let api_config = ApiConfig::get();

    println!("🔧 Configuration loaded");
    println!("   Server: {}:{}", &api_config.api_server_host, &api_config.api_server_port);
    println!("   Max DB connections: {}", &core_config.max_connections);

    // create mysql pool using properties in config
    let pool = create_database_pool(&core_config).await?;

    // wait for the pool to initialize
    init_database(&pool)
        .await
        .context("Failed to initialize database")?;

    // Wrap db pool in a thread safe reference
    let db_connection_pool = Arc::new(pool);

    // initialize app with routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/prompt", post(create_prompt_handler))
        .route("/prompts", get(get_prompts_handler))
        .route("/get-models", get(get_models_handler))
        .layer(CorsLayer::permissive()) // this is not a good idea for production
        .with_state(db_connection_pool); // set the DatabaseConnection state

    let bind_address = format!("{}:{}", &api_config.api_server_host, &api_config.api_server_port);
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .context(format!("Failed to bind to {}", bind_address))?;

    println!("🚀 Server running on http://{}", bind_address);
    println!("📝 POST to /prompt to create a prompt");
    println!("📋 GET /prompts to view all prompts");
    println!("⚛️ GET /models to view a provider's models");
    println!("❤️ GET /health for health check");

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}