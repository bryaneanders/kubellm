// allow these files to publicly accessed by things importing the core library
pub mod config;
pub mod database;
pub mod models;

// allows use of these structis and functions outside the core library without
// needing to specify the full path
pub use config::Config;
pub use database::{create_database_pool, init_database, create_message, get_all_messages};
pub use models::*;