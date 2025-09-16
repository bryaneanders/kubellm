use anyhow::Result;
use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct CliConfig {
    pub history_file_path: PathBuf,
}

static CLI_CONFIG: OnceLock<CliConfig> = OnceLock::new();

impl CliConfig {
    pub fn from_env() -> Result<Self> {
        let history_file_path = env::var("HISTORY_FILE_PATH");
        let history_file_path = history_file_path
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(Self::get_history_file_path);

        Ok(CliConfig { history_file_path })
    }

    pub fn get() -> &'static CliConfig {
        CLI_CONFIG.get_or_init(|| Self::from_env().expect("Failed to load configuration"))
    }

    pub fn get_history_file_path() -> PathBuf {
        dirs::home_dir()
            .map(|home| home.join(".kubellm-cli-history"))
            .unwrap_or_else(|| PathBuf::from(".kubellm-cli-history"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_history_file_path() {
        let path = CliConfig::get_history_file_path();
        assert!(path.to_string_lossy().contains(".kubellm-cli-history"));
    }

    #[test]
    fn test_from_env_with_custom_history_path() {
        env::set_var("HISTORY_FILE_PATH", "/tmp/test-history");
        let config = CliConfig::from_env().unwrap();
        assert_eq!(config.history_file_path, PathBuf::from("/tmp/test-history"));
        env::remove_var("HISTORY_FILE_PATH");
    }

    #[test]
    fn test_from_env_without_custom_history_path() {
        env::remove_var("HISTORY_FILE_PATH");
        let config = CliConfig::from_env().unwrap();
        assert!(config
            .history_file_path
            .to_string_lossy()
            .contains(".kubellm-cli-history"));
    }

    #[test]
    fn test_config_debug() {
        let config = CliConfig {
            history_file_path: PathBuf::from("/test/path"),
        };
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("CliConfig"));
        assert!(debug_str.contains("/test/path"));
    }
}
