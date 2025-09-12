use anyhow::{Context, Result};
use std::env;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct CoreConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub anthropic_url: String,
    pub anthropic_key: Option<String>,
    pub default_anthropic_model: String,
    pub openai_url: String,
    pub openai_key: Option<String>,
    pub default_openai_model: String,
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

        let default_anthropic_model = env::var("DEFAULT_ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());

        let openai_url =
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let openai_key = env::var("OPENAI_KEY").ok();

        let default_openai_model =
            env::var("DEFAULT_OPENAI_MODEL").unwrap_or_else(|_| "gpt-5".to_string());

        Ok(CoreConfig {
            database_url,
            max_connections,
            anthropic_url,
            anthropic_key,
            default_anthropic_model,
            openai_url,
            openai_key,
            default_openai_model,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    // Test version of from_env that doesn't load .env file
    fn test_config_from_env() -> Result<CoreConfig> {
        let database_url = CoreConfig::build_db_url()?;

        let max_connections = env::var("DB_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u32>()
            .context("DB_MAX_CONNECTIONS must be a valid number")?;

        let anthropic_url = env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com/v1".to_string());

        let anthropic_key = env::var("ANTHROPIC_KEY").ok();

        let default_anthropic_model = env::var("DEFAULT_ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());

        let openai_url =
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let openai_key = env::var("OPENAI_KEY").ok();

        let default_openai_model =
            env::var("DEFAULT_OPENAI_MODEL").unwrap_or_else(|_| "gpt-5".to_string());

        Ok(CoreConfig {
            database_url,
            max_connections,
            anthropic_url,
            anthropic_key,
            default_anthropic_model,
            openai_url,
            openai_key,
            default_openai_model,
        })
    }

    fn setup_test_env() {
        let _guard = TEST_MUTEX.lock().unwrap();
        
        // Remove any .env loaded variables first
        env::remove_var("DB_HOST");
        env::remove_var("DB_PORT");
        env::remove_var("DB_NAME");
        env::remove_var("DB_USER");
        env::remove_var("DB_PASSWORD");
        env::remove_var("DB_MAX_CONNECTIONS");
        env::remove_var("ANTHROPIC_BASE_URL");
        env::remove_var("ANTHROPIC_KEY");
        env::remove_var("DEFAULT_ANTHROPIC_MODEL");
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("OPENAI_KEY");
        env::remove_var("DEFAULT_OPENAI_MODEL");
        
        env::set_var("DB_HOST", "test-host");
        env::set_var("DB_PORT", "3307");
        env::set_var("DB_NAME", "test-db");
        env::set_var("DB_USER", "test-user");
        env::set_var("DB_PASSWORD", "test-pass");
        env::set_var("DB_MAX_CONNECTIONS", "5");
    }

    fn cleanup_test_env() {
        env::remove_var("DB_HOST");
        env::remove_var("DB_PORT");
        env::remove_var("DB_NAME");
        env::remove_var("DB_USER");
        env::remove_var("DB_PASSWORD");
        env::remove_var("DB_MAX_CONNECTIONS");
        env::remove_var("ANTHROPIC_BASE_URL");
        env::remove_var("ANTHROPIC_KEY");
        env::remove_var("DEFAULT_ANTHROPIC_MODEL");
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("OPENAI_KEY");
        env::remove_var("DEFAULT_OPENAI_MODEL");
    }

    #[test]
    #[serial]
    fn test_build_db_url_success() {
        setup_test_env();
        
        let result = CoreConfig::build_db_url().unwrap();
        assert_eq!(result, "mysql://test-user:test-pass@test-host:3307/test-db");
        
        cleanup_test_env();
    }

    #[test]
    #[serial]
    fn test_build_db_url_missing_host() {
        cleanup_test_env();
        env::set_var("DB_NAME", "test-db");
        env::set_var("DB_USER", "test-user");
        env::set_var("DB_PASSWORD", "test-pass");
        
        let result = CoreConfig::build_db_url();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("DB_HOST is required"));
        
        cleanup_test_env();
    }

    #[test]
    #[serial]
    fn test_build_db_url_missing_name() {
        cleanup_test_env();
        env::set_var("DB_HOST", "test-host");
        env::set_var("DB_USER", "test-user");
        env::set_var("DB_PASSWORD", "test-pass");
        
        let result = CoreConfig::build_db_url();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("DB_NAME is required"));
        
        cleanup_test_env();
    }

    #[test]
    #[serial]
    fn test_build_db_url_default_port() {
        cleanup_test_env();
        env::set_var("DB_HOST", "test-host");
        env::set_var("DB_NAME", "test-db");
        env::set_var("DB_USER", "test-user");
        env::set_var("DB_PASSWORD", "test-pass");
        
        let result = CoreConfig::build_db_url().unwrap();
        assert_eq!(result, "mysql://test-user:test-pass@test-host:3306/test-db");
        
        cleanup_test_env();
    }

    #[test]
    #[serial]
    fn test_from_env_with_defaults() {
        cleanup_test_env(); // Clean first to ensure no leftover values
        setup_test_env();
        
        let config = test_config_from_env().unwrap();
        
        assert_eq!(config.database_url, "mysql://test-user:test-pass@test-host:3307/test-db");
        assert_eq!(config.max_connections, 5);
        assert_eq!(config.anthropic_url, "https://api.anthropic.com/v1");
        assert_eq!(config.anthropic_key, None);
        assert_eq!(config.default_anthropic_model, "claude-sonnet-4-20250514");
        assert_eq!(config.openai_url, "https://api.openai.com/v1");
        assert_eq!(config.openai_key, None);
        assert_eq!(config.default_openai_model, "gpt-5");
        
        cleanup_test_env();
    }

    #[test]
    #[serial]
    fn test_from_env_with_custom_values() {
        setup_test_env();
        env::set_var("ANTHROPIC_BASE_URL", "https://custom-anthropic.com");
        env::set_var("ANTHROPIC_KEY", "test-anthropic-key");
        env::set_var("DEFAULT_ANTHROPIC_MODEL", "claude-3");
        env::set_var("OPENAI_BASE_URL", "https://custom-openai.com");
        env::set_var("OPENAI_KEY", "test-openai-key");
        env::set_var("DEFAULT_OPENAI_MODEL", "gpt-4");
        
        let config = CoreConfig::from_env().unwrap();
        
        assert_eq!(config.anthropic_url, "https://custom-anthropic.com");
        assert_eq!(config.anthropic_key, Some("test-anthropic-key".to_string()));
        assert_eq!(config.default_anthropic_model, "claude-3");
        assert_eq!(config.openai_url, "https://custom-openai.com");
        assert_eq!(config.openai_key, Some("test-openai-key".to_string()));
        assert_eq!(config.default_openai_model, "gpt-4");
        
        cleanup_test_env();
    }

    #[test]
    #[serial]
    fn test_from_env_invalid_max_connections() {
        setup_test_env();
        env::set_var("DB_MAX_CONNECTIONS", "not-a-number");
        
        let result = CoreConfig::from_env();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("DB_MAX_CONNECTIONS must be a valid number"));
        
        cleanup_test_env();
    }

    #[test]
    #[serial]
    fn test_from_env_default_max_connections() {
        cleanup_test_env(); // Clean first to ensure no leftover values
        setup_test_env();
        env::remove_var("DB_MAX_CONNECTIONS");
        
        let config = test_config_from_env().unwrap();
        assert_eq!(config.max_connections, 10);
        
        cleanup_test_env();
    }
}
