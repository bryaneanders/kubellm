mod config;

use crate::config::WebConfig;
use anyhow::{Context, Result};
use axum::{response::Html, routing::get, Router};
use kubellm_core::{create_database_pool, init_database, CoreConfig};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, services::ServeDir};

// serve the contents of the html file
// the file is read at compile time and embedded in the binary (this gives speed but could explode a binary's size and memory size with many files)
// Html<T> sets `Content-Type: text/html`
// &'static str returns the file as a string slice
async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn serve_prompts() -> Html<&'static str> {
    Html(include_str!("../static/prompts.html"))
}

async fn serve_response() -> Html<&'static str> {
    Html(include_str!("../static/response.html"))
}

async fn health_check() -> &'static str {
    "Web app is running!"
}

#[tokio::main]
async fn main() -> Result<()> {
    let core_config = CoreConfig::get();
    let web_config = WebConfig::get();

    println!("🔧 Configuration loaded");
    println!(
        "   Server: {}:{}",
        &web_config.app_server_host, &web_config.app_server_port
    );
    println!("   Max DB connections: {}", core_config.max_connections);

    let pool = create_database_pool(core_config).await?;

    init_database(&pool)
        .await
        .context("Failed to initialize database")?;

    let db_connection = Arc::new(pool);

    let app = Router::new()
        .route("/", get(serve_index)) // serve html content
        .route("/prompts", get(serve_prompts)) // serve html content
        .route("/response", get(serve_response)) // serve html content
        .route("/health", get(health_check)) // rest endpoint
        .nest_service("/static", ServeDir::new("static"))
        .layer(CorsLayer::permissive()) // this is a bad idea for prod
        .with_state(db_connection); // store the Arc<MySqlPool> in the state (DatabaseConnection)

    let bind_address = format!(
        "{}:{}",
        &web_config.app_server_host, &web_config.app_server_port
    );
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .context(format!("Failed to bind to {}", bind_address))?;

    println!("🚀 Web app running on http://{}", bind_address);
    println!("🌐 Open your browser to view the interface");
    println!("📂 View all prompts at /prompts");
    println!("❤️  GET /health for health check");

    axum::serve(listener, app).await.context("Server error")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_serve_index() {
        let response = serve_index().await;
        let html_content = response.0;
        assert!(html_content.contains("html"));
    }

    #[tokio::test]
    async fn test_serve_prompts() {
        let response = serve_prompts().await;
        let html_content = response.0;
        assert!(html_content.contains("html"));
    }

    #[tokio::test]
    async fn test_serve_response() {
        let response = serve_response().await;
        let html_content = response.0;
        assert!(html_content.contains("html"));
    }

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        assert_eq!(response, "Web app is running!");
    }

    #[tokio::test]
    async fn test_routes_without_database() {
        let app = Router::new()
            .route("/", get(serve_index))
            .route("/prompts", get(serve_prompts))
            .route("/response", get(serve_response))
            .route("/health", get(health_check))
            .layer(CorsLayer::permissive());

        let server = TestServer::new(app).unwrap();

        let response = server.get("/health").await;
        response.assert_status(StatusCode::OK);
        response.assert_text("Web app is running!");

        let response = server.get("/").await;
        response.assert_status(StatusCode::OK);
        response.assert_header("content-type", "text/html; charset=utf-8");

        let response = server.get("/prompts").await;
        response.assert_status(StatusCode::OK);
        response.assert_header("content-type", "text/html; charset=utf-8");

        let response = server.get("/response").await;
        response.assert_status(StatusCode::OK);
        response.assert_header("content-type", "text/html; charset=utf-8");
    }

    #[tokio::test]
    async fn test_invalid_route() {
        let app = Router::new()
            .route("/health", get(health_check))
            .layer(CorsLayer::permissive());

        let server = TestServer::new(app).unwrap();

        let response = server.get("/invalid").await;
        response.assert_status(StatusCode::NOT_FOUND);
    }
}
