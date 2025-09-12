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

        let api_server_host =
            env::var("API_SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_from_env_with_defaults() {
        env::remove_var("API_SERVER_HOST");
        env::remove_var("SERVER_PORT");

        let config = ApiConfig::from_env().unwrap();
        assert_eq!(config.api_server_host, "127.0.0.1");
        assert_eq!(config.api_server_port, 3001);
    }

    #[test]
    #[serial]
    fn test_from_env_with_custom_values() {
        env::set_var("API_SERVER_HOST", "0.0.0.0");
        env::set_var("SERVER_PORT", "8080");

        let config = ApiConfig::from_env().unwrap();
        assert_eq!(config.api_server_host, "0.0.0.0");
        assert_eq!(config.api_server_port, 8080);

        env::remove_var("API_SERVER_HOST");
        env::remove_var("SERVER_PORT");
    }

    #[test]
    #[serial]
    fn test_from_env_invalid_port() {
        env::set_var("SERVER_PORT", "not_a_number");

        let result = ApiConfig::from_env();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("SERVER_PORT must be a valid port number"));

        env::remove_var("SERVER_PORT");
    }

    #[test]
    #[serial]
    fn test_from_env_port_out_of_range() {
        env::set_var("SERVER_PORT", "70000");

        let result = ApiConfig::from_env();
        assert!(result.is_err());

        env::remove_var("SERVER_PORT");
    }
}
