use std::sync::Arc;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use sqlx::MySqlPool;

use core::{
    CreatePromptRequest, CreatePromptResponse, ErrorResponse, Prompt,
    create_prompt_record, get_all_prompts
};

// Map Arc<MySqlPool> as the type DatabaseConnection
// This allows the pool state to be extracted based on the state
// passed in when the router is initialized
// separates the db implementation from DatabaseConnection type in handlers

type DatabaseConnection = Arc<MySqlPool>;

pub async fn create_prompt_handler(
    State(pool): State<DatabaseConnection>, // extract db pool from api state (set in router declaration)
    Json(payload): Json<CreatePromptRequest>, // extract prompt json from request
) -> anyhow::Result<Json<CreatePromptResponse>, (StatusCode, Json<ErrorResponse>)> {
    if payload.prompt.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Prompt cannot be empty".to_string(),
            }),
        ));
    }

    match create_prompt_record(&pool, payload.prompt, None).await {
        Ok(prompt) => Ok(Json(prompt)), // return prompt as json on success
        Err(e) => {
            eprintln!("Database error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create prompt".to_string(),
                }),
            )) // return error json on failure
        }
    }
}

pub async fn get_prompts_handler(
    State(pool): State<DatabaseConnection>, // extract db pool from api state (router declaration)
) -> Result<Json<Vec<Prompt>>, (StatusCode, Json<ErrorResponse>)> {
    match get_all_prompts(&pool).await {
        Ok(prompts) => Ok(Json(prompts)), // return all prompts as json on success
        Err(e) => {
            eprintln!("Database error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch prompts".to_string(),
                }),
            )) // return error json on failure
        }
    }
}