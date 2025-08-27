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
            .map(|home| home.join(".prompt-cli-history"))
            .unwrap_or_else(|| PathBuf::from(".prompt-cli-history"))
    }
}
