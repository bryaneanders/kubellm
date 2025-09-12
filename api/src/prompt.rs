use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use sqlx::MySqlPool;
use std::sync::Arc;

use kubellm_core::{
    get_all_prompts, get_models, prompt_model, CreatePromptRequest, ErrorResponse, GetModelsQuery,
    Prompt, Provider,
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

pub async fn get_providers_handler(
) -> anyhow::Result<Json<Vec<String>>, (StatusCode, Json<ErrorResponse>)> {
    let providers = Provider::all();
    let provider_strings: Vec<String> = providers.iter().map(|p| p.to_string()).collect();
    Ok(Json(provider_strings))
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use kubellm_core::{CreatePromptRequest, ErrorResponse};

    #[test]
    fn test_create_prompt_request_empty_prompt() {
        let payload = CreatePromptRequest {
            prompt: "".to_string(),
            provider: "Anthropic".to_string(),
            model: None,
        };

        assert_eq!(payload.prompt, "");
        assert_eq!(payload.provider, "Anthropic");
        assert!(payload.model.is_none());
    }

    #[test]
    fn test_create_prompt_request_with_model() {
        let payload = CreatePromptRequest {
            prompt: "Test prompt".to_string(),
            provider: "OpenAI".to_string(),
            model: Some("gpt-4".to_string()),
        };

        assert_eq!(payload.prompt, "Test prompt");
        assert_eq!(payload.provider, "OpenAI");
        assert_eq!(payload.model, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_error_response_creation() {
        let error = ErrorResponse {
            error: "Test error message".to_string(),
        };

        assert_eq!(error.error, "Test error message");
    }

    #[test]
    fn test_empty_prompt_validation() {
        assert!(
            "".trim().is_empty(),
            "Empty string should be considered empty"
        );
        assert!(
            "   \n\t  ".trim().is_empty(),
            "Whitespace-only string should be considered empty"
        );
        assert!(
            !"Hello world".trim().is_empty(),
            "Non-empty string should not be considered empty"
        );
    }

    #[test]
    fn test_provider_validation() {
        let providers = Provider::all();
        let provider_strings: Vec<String> = providers.iter().map(|p| p.to_string()).collect();

        assert_eq!(provider_strings.len(), 2);
        assert!(provider_strings.contains(&"Anthropic".to_string()));
        assert!(provider_strings.contains(&"OpenAI".to_string()));
    }

    #[test]
    fn test_status_code_mappings() {
        assert_eq!(StatusCode::BAD_REQUEST.as_u16(), 400);
        assert_eq!(StatusCode::INTERNAL_SERVER_ERROR.as_u16(), 500);
        assert_eq!(StatusCode::OK.as_u16(), 200);
    }

    #[test]
    fn test_prompt_validation_logic() {
        let empty_prompt = "";
        let whitespace_prompt = "   \n\t  ";
        let valid_prompt = "Hello, world!";

        assert!(empty_prompt.trim().is_empty());
        assert!(whitespace_prompt.trim().is_empty());
        assert!(!valid_prompt.trim().is_empty());
    }

    #[test]
    fn test_database_connection_type_definition() {
        use std::any::TypeId;

        assert_eq!(
            TypeId::of::<DatabaseConnection>(),
            TypeId::of::<Arc<MySqlPool>>()
        );
    }
}
