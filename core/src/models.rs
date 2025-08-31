use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::str::FromStr;
use strum::{Display, EnumIter, IntoEnumIterator};

// maps the json containing the prompt into this struct
#[derive(Deserialize)]
pub struct CreatePromptRequest {
    pub prompt: String,
    pub provider: String,
    pub model: Option<String>,
}

// Serialize: used to convert this struct into JSON for responses
// FromRow: maps the database row into this struct
#[derive(Serialize, FromRow)]
pub struct Prompt {
    pub id: i64,
    pub prompt: String,
    pub model: String,
    pub provider: String,
    pub response: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Deserialize)]
pub struct GetModelsQuery {
    pub provider: String,
}

#[derive(Display, EnumIter)]
pub enum Provider {
    #[strum(to_string = "Anthropic")]
    Anthropic,
    #[strum(to_string = "OpenAI")]
    OpenAI,
}

impl FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "anthropic" => Ok(Provider::Anthropic),
            "openai" => Ok(Provider::OpenAI),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}

impl Provider {
    pub fn all() -> Vec<Provider> {
        Provider::iter().collect()
    }

    pub fn all_names() -> Vec<String> {
        Provider::iter().map(|p| p.to_string()).collect()
    }
}
