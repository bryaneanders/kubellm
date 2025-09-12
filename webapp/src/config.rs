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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_from_env_with_defaults() {
        env::remove_var("APP_SERVER_HOST");
        env::remove_var("SERVER_PORT");

        let config = WebConfig::from_env().unwrap();
        assert_eq!(config.app_server_host, "127.0.0.1");
        assert_eq!(config.app_server_port, 3000);
    }

    #[test]
    #[serial]
    fn test_from_env_with_custom_values() {
        env::set_var("APP_SERVER_HOST", "0.0.0.0");
        env::set_var("SERVER_PORT", "8080");

        let config = WebConfig::from_env().unwrap();
        assert_eq!(config.app_server_host, "0.0.0.0");
        assert_eq!(config.app_server_port, 8080);

        env::remove_var("APP_SERVER_HOST");
        env::remove_var("SERVER_PORT");
    }

    #[test]
    #[serial]
    fn test_from_env_invalid_port() {
        env::set_var("SERVER_PORT", "invalid_port");

        let result = WebConfig::from_env();
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

        let result = WebConfig::from_env();
        assert!(result.is_err());

        env::remove_var("SERVER_PORT");
    }

    #[test]
    fn test_debug_implementation() {
        let config = WebConfig {
            app_server_host: "127.0.0.1".to_string(),
            app_server_port: 3000,
        };

        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("WebConfig"));
        assert!(debug_str.contains("127.0.0.1"));
        assert!(debug_str.contains("3000"));
    }
}
