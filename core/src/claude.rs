use serde::{Deserialize, Serialize};
use crate::{Config, CreatePromptResponse};
use reqwest::Client;
use sqlx::MySqlPool;
use crate::create_prompt_record;

#[derive(Debug, Deserialize)]
pub struct ClaudeResponse {
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub role: String,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    pub text: String,
    #[serde(rename = "type")]
    pub content_type: String,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// Request structures
#[derive(Debug, Serialize)]
pub struct ClaudeRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicModel {
    id: String,
    #[serde(rename = "type")]
    model_type: String,
    display_name: String,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicModelsResponse {
    data: Vec<AnthropicModel>,
    has_more: bool,
    first_id: Option<String>,
    last_id: Option<String>,
}

pub async fn call_claude(prompt: &str, model: Option<&str>, pool: &MySqlPool,) -> Result<CreatePromptResponse, Box<dyn std::error::Error>> {
    let config = Config::get();
    let client = Client::new();

    let mut model = model.unwrap_or(&config.default_claude_model);
    let models = get_claude_models().await?;
    // loop over models and make sure the passed in models is valid
    if !models.iter().any(|m| m.id == model) {
        model = &config.default_claude_model;
    }

    let request = ClaudeRequest {
        model: model.to_string(),
        max_tokens: 1024,
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let response = client
        .post(&format!("{}/messages", &config.anthropic_url))
        .header("x-api-key", &config.anthropic_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await?;

    // Check if request was successful
    if response.status().is_success() {
        // Parse the response
        let claude_response: ClaudeResponse = response.json().await?;

        // Extract the text from the first content block
        let response_text = claude_response
            .content
            .first()
            .map(|block| block.text.clone())
            .unwrap_or_else(|| "No response content".to_string());

        Ok(create_prompt_record(pool, prompt.to_string(), Some(&response_text), Some(model)).await?)
    } else {
        let error_text = response.text().await?;
        Err(format!("API request failed: {}", error_text).into())
    }
}

pub async fn get_claude_models() -> Result<Vec<AnthropicModel>, Box<dyn std::error::Error>> {
    let config = Config::get();
    let client = Client::new();

    let response = client
        .get(&format!("{}/models", &config.anthropic_url))
        .header("x-api-key", &config.anthropic_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await?;

    if response.status().is_success() {
        let models_response: AnthropicModelsResponse = response.json().await?;
        let models: Vec<AnthropicModel> = models_response.data;

        Ok(models)
    } else {
        let error_text = response.text().await?;
        Err(format!("API request failed: {}", error_text).into())
    }
}