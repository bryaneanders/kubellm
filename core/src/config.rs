// use error contexts and result statuses from anyhow
use anyhow::{Context, Result};
// use dotenvy to load environment variables from a .env file
use std::env;
use std::sync::OnceLock;

// provides debug logging for configuration
#[derive(Debug)]
pub struct Config { // defines the config struct
    pub database_url: String,
    pub app_server_host: String,
    pub app_server_port: u16,
    pub api_server_host: String,
    pub api_server_port: u16,
    pub max_connections: u32,
    pub anthropic_url: String,
    pub anthropic_key: String,
    pub default_claude_model: String,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

// implements functions for the config struct
impl Config {
    pub fn from_env() -> Result<Self> {
        // Load environment variables from a .env file if it exists
        dotenvy::dotenv().ok();

        // .context is use provide an error prompt if the environment variable is not set
        let database_url = env::var("DATABASE_URL")
            .context("DATABASE_URL environment variable is required")?;

        let app_server_host = env::var("APP_SERVER_HOST")
            .unwrap_or_else(|_| "127.0.0.1".to_string());

        let app_server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .context("SERVER_PORT must be a valid port number")?;

        let api_server_host = env::var("APP_SERVER_HOST")
            .unwrap_or_else(|_| "127.0.0.1".to_string());

        let api_server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3001".to_string())
            .parse::<u16>()
            .context("SERVER_PORT must be a valid port number")?;

        let max_connections = env::var("DB_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u32>()
            .context("DB_MAX_CONNECTIONS must be a valid number")?;

        let anthropic_url = env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com/v1".to_string());

        let anthropic_key = env::var("ANTHROPIC_KEY")?;

        let default_claude_model = env::var("DEFAULT_CLAUDE_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());

        Ok(Config {
            database_url,
            app_server_host,
            app_server_port,
            api_server_host,
            api_server_port,
            max_connections,
            anthropic_url,
            anthropic_key,
            default_claude_model,
        }) // return last statement with no semicolon, in this case
        // returns an instance of Config wrapped in an Ok response
    }

    pub fn get() -> &'static Config {
        CONFIG.get_or_init(|| Self::from_env().expect("Failed to load configuration"))
    }

    pub fn _from_components() -> Result<String> {
        let host = env::var("DB_HOST").context("DB_HOST is required")?;
        let port = env::var("DB_PORT").unwrap_or_else(|_| "3306".to_string());
        let database = env::var("DB_NAME").context("DB_NAME is required")?;
        let username = env::var("DB_USER").context("DB_USER is required")?;
        let password = env::var("DB_PASSWORD").context("DB_PASSWORD is required")?;

        Ok(format!(
            "mysql://{}:{}@{}:{}/{}",
            username, password, host, port, database
        )) // returns the mysql connection string wrapped in an ok response
    }
}