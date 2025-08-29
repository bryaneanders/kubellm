use anyhow::{Context, Result};
use std::env;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct CoreConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub anthropic_url: String,
    pub anthropic_key: Option<String>,
    pub openai_url: String,
    pub openai_key: Option<String>,
    pub default_claude_model: String,
}

static CONFIG: OnceLock<CoreConfig> = OnceLock::new();

// implements functions for the config struct
impl CoreConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let database_url = Self::build_db_url()?;

        let max_connections = env::var("DB_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u32>()
            .context("DB_MAX_CONNECTIONS must be a valid number")?;

        let anthropic_url = env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com/v1".to_string());

        let anthropic_key = env::var("ANTHROPIC_KEY").ok();

        let openai_url = env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com/v1".to_string());

        let openai_key = env::var("ANTHROPIC_KEY").ok();

        let default_claude_model = env::var("DEFAULT_CLAUDE_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());

        Ok(CoreConfig {
            database_url,
            max_connections,
            anthropic_url,
            anthropic_key,
            openai_url,
            openai_key,
            default_claude_model,
        })
    }

    pub fn get() -> &'static CoreConfig {
        CONFIG.get_or_init(|| Self::from_env().expect("Failed to load configuration"))
    }

    fn build_db_url() -> Result<String> {
        let host = env::var("DB_HOST").context("DB_HOST is required")?;
        let port = env::var("DB_PORT").unwrap_or_else(|_| "3306".to_string());
        let database = env::var("DB_NAME").context("DB_NAME is required")?;
        let username = env::var("DB_USER").context("DB_USER is required")?;
        let password = env::var("DB_PASSWORD").context("DB_PASSWORD is required")?;

        Ok(format!(
            "mysql://{}:{}@{}:{}/{}",
            username, password, host, port, database
        ))
    }
}
