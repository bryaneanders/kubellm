use anyhow::{Context, Result};
use std::env;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct WebConfig {
    pub app_server_host: String,
    pub app_server_port: u16,
}

static WEB_CONFIG: OnceLock<WebConfig> = OnceLock::new();

impl WebConfig {
    pub fn from_env() -> Result<Self> {
        let app_server_host =
            env::var("APP_SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

        let app_server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .context("SERVER_PORT must be a valid port number")?;

        Ok(WebConfig {
            app_server_host,
            app_server_port,
        })
    }

    pub fn get() -> &'static WebConfig {
        WEB_CONFIG.get_or_init(|| Self::from_env().expect("Failed to load configuration"))
    }
}
