use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// maps the json containing the message into this struct
#[derive(Deserialize)]
pub struct CreateMessageRequest {
    pub message: String,
}

// Serialize: used to convert this struct into JSON for responses
// FromRow: maps the database row into this struct
#[derive(Serialize, FromRow)]
pub struct Message {
    pub id: i64,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct CreateMessageResponse {
    pub id: i64,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}