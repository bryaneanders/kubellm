use std::sync::Arc;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use sqlx::MySqlPool;
use core::{
    CreatePromptRequest, CreatePromptResponse, ErrorResponse, AnthropicModel, call_claude, get_claude_models
};

type DatabaseConnection = Arc<MySqlPool>;

pub async fn claude_prompt_handler(
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

    let model = payload.model
        .as_ref()
        .map(|m| m.trim())
        .filter(|m| !m.is_empty());
    
    match call_claude(payload.prompt.trim(), model,  &pool).await {
        Ok(prompt_response) => Ok(Json(prompt_response)),
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

pub async fn claude_models_handler()-> Result<Json<Vec<AnthropicModel>>, (StatusCode, Json<ErrorResponse>)> {
    match get_claude_models().await {
        Ok(models) => Ok(Json(models)),
        Err(e) => {
            eprintln!("Error retrieving Anthropic models: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve Anthropic models".to_string(),
                }),
            ))
        }
    }
}