use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use sqlx::MySqlPool;
use std::sync::Arc;

use core::{
    get_all_prompts, get_models, prompt_model, CreatePromptRequest, ErrorResponse, GetModelsQuery,
    Prompt,
};

// Map Arc<MySqlPool> as the type DatabaseConnection
// This allows the pool state to be extracted based on the state
// passed in when the router is initialized
// separates the db implementation from DatabaseConnection type in handlers

type DatabaseConnection = Arc<MySqlPool>;

pub async fn create_prompt_handler(
    State(pool): State<DatabaseConnection>, // extract db pool from api state (set in router declaration)
    Json(payload): Json<CreatePromptRequest>, // extract prompt json from request
) -> anyhow::Result<Json<Prompt>, (StatusCode, Json<ErrorResponse>)> {
    if payload.prompt.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Prompt cannot be empty".to_string(),
            }),
        ));
    }

    match prompt_model(
        &payload.prompt,
        &payload.provider,
        payload.model.as_deref(),
        &pool,
    )
    .await
    {
        Ok(prompt) => Ok(Json(prompt)), // return prompt as json on success
        Err(e) => {
            eprintln!(
                "Error prompting model for provider {}: {}",
                &payload.provider, e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    }
}

// Dunno why its marked dead code
#[allow(dead_code)]
pub async fn get_models_handler(
    Query(params): Query<GetModelsQuery>,
) -> anyhow::Result<Json<Vec<String>>, (StatusCode, Json<ErrorResponse>)> {
    match get_models(&params.provider).await {
        Ok(models) => Ok(Json(models)),
        Err(e) => {
            eprintln!(
                "Error retrieving models for provider {}: {}",
                &params.provider, e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve models".to_string(),
                }),
            ))
        }
    }
}

pub async fn get_prompts_handler(
    State(pool): State<DatabaseConnection>, // extract db pool from api state (router declaration)
) -> anyhow::Result<Json<Vec<Prompt>>, (StatusCode, Json<ErrorResponse>)> {
    match get_all_prompts(&pool).await {
        Ok(prompts) => Ok(Json(prompts)), // return all prompts as json on success
        Err(e) => {
            eprintln!("Database error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to fetch prompts".to_string(),
                }),
            ))
        }
    }
}
