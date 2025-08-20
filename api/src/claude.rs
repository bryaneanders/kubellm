use std::sync::Arc;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use sqlx::MySqlPool;

type DatabaseConnection = Arc<MySqlPool>;


use core::{
    CreatePromptRequest, CreatePromptResponse, ErrorResponse, call_claude
};

pub async fn prompt_claude_handler(
    State(pool): State<DatabaseConnection>,
    Json(payload): Json<CreatePromptRequest>
)-> anyhow::Result<Json<CreatePromptResponse>, (StatusCode, Json<ErrorResponse>)> {
    if payload.prompt.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Prompt cannot be empty".to_string(),
            }),
        ));
    }

    match call_claude(payload.prompt.trim(), &pool).await {
        Ok(prompt_response) => {
            Ok(Json(prompt_response)) // Return the response as JSON
        }
        Err(e) => {
            eprintln!("Error running prompt: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to run prompt".to_string(),
                }),
            ))
        }
    }
}