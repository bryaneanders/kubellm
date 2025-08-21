use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// maps the json containing the prompt into this struct
#[derive(Deserialize)]
pub struct CreatePromptRequest {
    pub prompt: String,
    pub model: Option<String>
}

// Serialize: used to convert this struct into JSON for responses
// FromRow: maps the database row into this struct
#[derive(Serialize, FromRow)]
pub struct Prompt {
    pub id: i64,
    pub prompt: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct CreatePromptResponse {
    pub id: i64,
    pub prompt: String,
    pub model: String,
    pub response: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
