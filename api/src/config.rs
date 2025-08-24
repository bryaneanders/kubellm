use anyhow::{Context, Result};
use std::env;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct ApiConfig {
    pub api_server_host: String,
    pub api_server_port: u16,
}

static API_CONFIG: OnceLock<ApiConfig> = OnceLock::new();

impl ApiConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let api_server_host = env::var("API_SERVER_HOST")
            .unwrap_or_else(|_| "127.0.0.1".to_string());

        let api_server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3001".to_string())
            .parse::<u16>()
            .context("SERVER_PORT must be a valid port number")?;

        Ok(ApiConfig {
            api_server_host,
            api_server_port,
        })
    }

    pub fn get() -> &'static ApiConfig {
        API_CONFIG.get_or_init(|| Self::from_env().expect("Failed to load api configuration"))
    }
}