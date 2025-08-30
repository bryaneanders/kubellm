use crate::create_prompt_record;
use crate::{CoreConfig, Prompt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIModelsResponse {
    pub object: String,
    pub data: Vec<OpenAIModel>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIModel {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    pub temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    pub usage: OpenAIUsage,
    pub service_tier: String,
    pub system_fingerprint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: OpenAIMessage,
    pub finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String,
    /*pub refusal: Option<String>,
    pub annotations: Vec<serde_json::Value>,*/
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub prompt_tokens_details: OpenAIPromptTokensDetails,
    pub completion_tokens_details: OpenAICompletionTokensDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIPromptTokensDetails {
    pub cached_tokens: u32,
    pub audio_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAICompletionTokensDetails {
    pub reasoning_tokens: u32,
    pub audio_tokens: u32,
    pub accepted_prediction_tokens: u32,
    pub rejected_prediction_tokens: u32,
}

pub async fn call_openai(
    prompt: &str,
    model: Option<&str>,
    pool: &MySqlPool,
) -> Result<Prompt, Box<dyn std::error::Error>> {
    let config = CoreConfig::get();
    let client = Client::new();

    if config.openai_key.is_none() {
        return Err("ANTHROPIC_KEY is not set".into());
    }

    let mut model = model.unwrap_or(&config.default_openai_model);
    let models = get_openai_models().await?;
    if !models.iter().any(|m| m.id == model) {
        model = &config.default_openai_model;
    }

    // todo want to make this more robust and less repetitive
    let request;
    if model == "gpt-5" {
        request = OpenAIChatRequest {
            model: model.to_string(),
            max_tokens: None,
            max_completion_tokens: Some(1024),
            temperature: 1.0,
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };
    } else {
        request = OpenAIChatRequest {
            model: model.to_string(),
            max_tokens: Some(2048),
            max_completion_tokens: None,
            temperature: 0.5,
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };
    }

    let response = client
        .post(&format!("{}/chat/completions", &config.openai_url))
        .header(
            "authorization",
            format!("Bearer {}", &config.openai_key.as_ref().unwrap()),
        )
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        let chat_response: OpenAIChatResponse = response.json().await?;
        if let Some(choice) = chat_response.choices.first() {
            let repose_text = choice.message.content.as_str();

            Ok(
                create_prompt_record(pool, prompt.to_string(), Some(repose_text), Some(model))
                    .await?,
            )
        } else {
            Err("No choices returned from OpenAI API".into())
        }
    } else {
        let error_text = response.text().await?;
        Err(format!("OpenAI API request failed: {}", error_text).into())
    }
}

pub async fn get_openai_models() -> Result<Vec<OpenAIModel>, Box<dyn std::error::Error>> {
    let config = CoreConfig::get();

    if config.openai_key.is_none() {
        return Err("OPENAI_KEY is not set".into());
    }

    let client = Client::new();
    let response = client
        .get(format!("{}/models", &config.openai_url))
        .header(
            "Authorization",
            format!("Bearer {}", &config.openai_key.clone().unwrap()),
        )
        .send()
        .await?;

    if response.status().is_success() {
        let models_response: OpenAIModelsResponse = response.json().await?;
        let models: Vec<OpenAIModel> = models_response.data;

        Ok(models)
    } else {
        let error_text = response.text().await?;
        Err(format!("OpenAI API request failed: {}", error_text).into())
    }
}
