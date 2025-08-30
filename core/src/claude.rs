use crate::create_prompt_record;
use crate::{CoreConfig, Prompt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

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
    pub messages: Vec<ClaudeMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl ClaudeRequest {
    pub fn new(model: String, messages: Vec<ClaudeMessage>) -> Self {
        Self {
            model,
            messages,
            temperature: 0.5, // default to moderate randomness
            max_tokens: 1024,
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }
}

pub struct ClaudeRequestBuilder {
    pub model: String,
    pub messages: Vec<ClaudeMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl ClaudeRequestBuilder {
    pub fn new(model: String) -> Self {
        Self {
            model,
            messages: Vec::new(),
            temperature: 0.5,
            max_tokens: 1024,
        }
    }

    pub fn messages(mut self, messages: Vec<ClaudeMessage>) -> Self {
        self.messages = messages;
        self
    }

    pub fn add_message(mut self, role: &str, content: &str) -> Self {
        self.messages.push(ClaudeMessage {
            role: role.to_string(),
            content: content.to_string(),
        });
        self
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = tokens;
        self
    }

    pub fn build(self) -> ClaudeRequest {
        ClaudeRequest::new(self.model, self.messages)
            .with_temperature(self.temperature)
            .with_max_tokens(self.max_tokens)
    }
}

#[derive(Debug, Serialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicModel {
    pub id: String,
    #[serde(rename = "type")]
    pub model_type: String,
    pub display_name: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicModelsResponse {
    data: Vec<AnthropicModel>,
    has_more: bool,
    first_id: Option<String>,
    last_id: Option<String>,
}

pub async fn call_claude(
    prompt: &str,
    model: Option<&str>,
    pool: &MySqlPool,
) -> Result<Prompt, Box<dyn std::error::Error>> {
    let config = CoreConfig::get();
    let client = Client::new();

    if config.anthropic_key.is_none() {
        return Err("ANTHROPIC_KEY is not set".into());
    }

    let mut model = model.unwrap_or(&config.default_claude_model);
    let models = get_claude_models().await?;
    // loop over models and make sure the passed in models is valid otherwise use default
    if !models.iter().any(|m| m.id == model) {
        println!("\r\x1b[2kInvalid model, {}, falling back to default model, {}", model, &config.default_claude_model);
        model = &config.default_claude_model;
    }

    let request = ClaudeRequestBuilder::new(model.to_string())
        .add_message("user", prompt)
        .max_tokens(1024)
        .build();

    let response = client
        .post(format!("{}/messages", &config.anthropic_url))
        .header("x-api-key", &config.anthropic_key.clone().unwrap())
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        // Parse the response
        let claude_response: ClaudeResponse = response.json().await?;

        // Extract the text from the first content block
        let response_text = claude_response
            .content
            .first()
            .map(|block| block.text.clone())
            .unwrap_or_else(|| "No response content".to_string());

        Ok(
            create_prompt_record(pool, prompt.to_string(), Some(&response_text), Some(model))
                .await?,
        )
    } else {
        let error_text = response.text().await?;
        Err(format!("Anthropic API request failed: {}", error_text).into())
    }
}

pub async fn get_claude_models() -> Result<Vec<AnthropicModel>, Box<dyn std::error::Error>> {
    let config = CoreConfig::get();

    if config.anthropic_key.is_none() {
        return Err("ANTHROPIC_KEY is not set".into());
    }

    let client = Client::new();
    let response = client
        .get(format!("{}/models", &config.anthropic_url))
        .header("x-api-key", &config.anthropic_key.clone().unwrap())
        .header("anthropic-version", "2023-06-01")
        .send()
        .await?;

    if response.status().is_success() {
        let models_response: AnthropicModelsResponse = response.json().await?;
        let models: Vec<AnthropicModel> = models_response.data;

        Ok(models)
    } else {
        let error_text = response.text().await?;
        Err(format!("Anthropic API request failed: {}", error_text).into())
    }
}
