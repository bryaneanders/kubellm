// allow these files to publicly accessed by things importing the core library
pub mod config;
pub mod database;
pub mod models;
pub mod claude;
pub mod prompt;

// allows use of these structs and functions outside the core library without
// needing to specify the full path
pub use config::CoreConfig;
pub use database::{create_database_pool, init_database, create_prompt_record, get_all_prompts};
pub use models::*;
pub use claude::{call_claude, get_claude_models, AnthropicModel};
pub use prompt::*;