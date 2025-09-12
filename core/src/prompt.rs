use crate::anthropic;
use crate::models::{Prompt, Provider};
use crate::openai;
use sqlx::MySqlPool;
use std::str::FromStr;

// prompt the provider - model optional
pub async fn prompt_model(
    prompt: &str,
    provider: &str,
    model: Option<&str>,
    pool: &MySqlPool,
) -> Result<Prompt, Box<dyn std::error::Error>> {
    match Provider::from_str(provider) {
        Ok(provider) => match provider {
            Provider::Anthropic => match anthropic::call_anthropic(prompt, model, pool).await {
                Ok(create_prompt_response) => Ok(create_prompt_response),
                Err(e) => Err(e),
            },
            Provider::OpenAI => match openai::call_openai(prompt, model, pool).await {
                Ok(create_prompt_response) => Ok(create_prompt_response),
                Err(e) => Err(e),
            },
        },
        Err(e) => Err(Box::from(e)),
    }
}

// get models for a given provider
pub async fn get_models(provider: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    match Provider::from_str(provider) {
        Ok(provider) => match provider {
            Provider::Anthropic => match anthropic::get_anthropic_models().await {
                Ok(models) => {
                    let model_names = models.into_iter().map(|m| m.id).collect();
                    Ok(model_names)
                }
                Err(e) => Err(e),
            },
            Provider::OpenAI => match openai::get_openai_models().await {
                Ok(models) => {
                    let model_names = models.into_iter().map(|m| m.id).collect();
                    Ok(model_names)
                }
                Err(e) => Err(e),
            },
        },
        Err(e) => Err(Box::from(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Provider;

    #[test]
    fn test_prompt_model_invalid_provider() {
        // Since we can't create a real database pool in unit tests,
        // we test the provider parsing logic separately
        let result = Provider::from_str("invalid_provider");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown provider"));
    }

    #[tokio::test]
    async fn test_get_models_invalid_provider() {
        let result = get_models("invalid_provider").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown provider"));
    }

    #[test]
    fn test_provider_string_parsing() {
        assert!(Provider::from_str("anthropic").is_ok());
        assert!(Provider::from_str("openai").is_ok());
        assert!(Provider::from_str("Anthropic").is_ok());
        assert!(Provider::from_str("OpenAI").is_ok());
        assert!(Provider::from_str("invalid").is_err());
    }

    #[test]
    fn test_error_propagation() {
        // Test that errors are properly wrapped and returned
        let provider_error = Provider::from_str("invalid");
        assert!(provider_error.is_err());

        let error_message = provider_error.unwrap_err();
        assert!(error_message.contains("Unknown provider"));
    }

    #[test]
    fn test_valid_providers() {
        let anthropic_result = Provider::from_str("anthropic");
        assert!(anthropic_result.is_ok());
        assert_eq!(anthropic_result.unwrap(), Provider::Anthropic);

        let openai_result = Provider::from_str("openai");
        assert!(openai_result.is_ok());
        assert_eq!(openai_result.unwrap(), Provider::OpenAI);
    }
}
