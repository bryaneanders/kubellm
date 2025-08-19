use axum::{
    response::{Html},
    routing::{get},
    Router,
};
use core::{
    Config, create_database_pool, init_database
};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, services::ServeDir};
use anyhow::{Context, Result};


// serve the contents of the html file
// the file is read at compile time and embedded in the binary (this gives speed but could explode a binary's size and memory size with many files)
// Html<T> sets `Content-Type: text/html`
// &'static str returns the file as a string slice
async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn serve_messages() -> Html<&'static str> {
    Html(include_str!("../static/messages.html"))
}

async fn health_check() -> &'static str {
    "Web app is running!"
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()
        .context("Failed to load configuration")?;

    println!("🔧 Configuration loaded");
    println!("   Server: {}:{}", config.app_server_host, config.app_server_port);
    println!("   Max DB connections: {}", config.max_connections);

    let pool = create_database_pool(&config).await?;

    init_database(&pool)
        .await
        .context("Failed to initialize database")?;

    let db_connection = Arc::new(pool);

    let app = Router::new()
        .route("/", get(serve_index)) // serve html content
        .route("/messages", get(serve_messages)) // serve html content
        .route("/health", get(health_check)) // rest endpoint
        .nest_service("/static", ServeDir::new("static"))
        .layer(CorsLayer::permissive()) // this is a bad idea for prod
        .with_state(db_connection); // store the Arc<MySqlPool> in the state (DatabaseConnection)

    let bind_address = format!("{}:{}", config.app_server_host, config.app_server_port);
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .context(format!("Failed to bind to {}", bind_address))?;

    println!("🚀 Web app running on http://{}", bind_address);
    println!("🌐 Open your browser to view the interface");
    println!("📂 View all messages at /messages");
    println!("❤️  GET /health for health check");

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}