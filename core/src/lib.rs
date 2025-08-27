// allow these files to publicly accessed by things importing the core library
pub mod claude;
pub mod config;
pub mod database;
pub mod models;
pub mod prompt;

// allows use of these structs and functions outside the core library without
// needing to specify the full path
pub use claude::{call_claude, get_claude_models, AnthropicModel};
pub use config::CoreConfig;
pub use database::{create_database_pool, create_prompt_record, get_all_prompts, init_database};
pub use models::*;
pub use prompt::*;
