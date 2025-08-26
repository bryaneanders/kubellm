use crate::models::{Provider, CreatePromptResponse};
use crate::claude;
use sqlx::MySqlPool;
use std::str::FromStr;

// prompt the provider - model optional
pub async fn prompt_model(prompt: &str, provider: &str, model: Option<&str>, pool: &MySqlPool,
) -> Result<CreatePromptResponse, Box<dyn std::error::Error>> {

    match Provider::from_str(provider) {
        Ok(provider) => {
            match provider {
                Provider::Anthropic => match claude::call_claude(prompt, model, pool).await {
                    Ok(create_prompt_response) => Ok(create_prompt_response),
                    Err(e) => Err(e),
                },
            }
        }
        Err(e) => Err(Box::from(e))
    }
}

// get models for a given provider
pub async fn get_models(provider: &str
) -> Result<Vec<String>, Box<dyn std::error::Error>> {

    match Provider::from_str(provider) {
        Ok(provider) => {
            match provider {
                Provider::Anthropic => match claude::get_claude_models().await {
                    Ok(models) => {
                        let model_names = models.into_iter()
                            .map(|m| m.id)
                            .collect();
                        Ok(model_names)
                    },
                    Err(e) => Err(e),
                },
            }
        },
        Err(e) => Err(Box::from(e))
    }
}